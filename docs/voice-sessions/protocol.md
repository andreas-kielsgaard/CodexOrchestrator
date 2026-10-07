# Phone protocol v1

Connect to `ws://<laptop Tailscale IPv4>:4818/phone` with `Authorization: Bearer <token>`. The server binds to that exact Tailscale address. Loopback is permitted for desktop tests; wildcard, LAN, and public bindings are rejected. Tailscale supplies encrypted device transport. One phone connection is accepted at a time.

All WebSocket data messages are JSON text. Native WebSocket ping/pong maintains transport liveness.

| Direction | Type | Fields |
| --- | --- | --- |
| Laptop to phone | `hello` | `version: 1`, `conversationId`, `mainSessionId` |
| Phone to laptop | `audio` | unique `id`, `audio` containing base64 WAV: mono signed 16-bit PCM, little endian, 16 kHz |
| Phone to laptop | `text` | unique `id`, `text` |
| Phone to laptop | `speech_state` | `speaking: true` at speech start, `false` after its final audio segment |
| Laptop to phone | `transcript` | `id`, completed `text` |
| Laptop to phone | `cue` | `cue: listening` when accepted input opens; `processing` when released |
| Laptop to phone | `speak` | `utterance: {id,kind,text,topicId,createdAt}` |
| Phone to laptop | `playback` or `interrupt` | `playback: {utteranceId,playedMs,state}` |
| Laptop to phone | `error` | `message` |
| Laptop to phone | `ended` | no additional fields |

Publication kinds are `acknowledgement`, `announcement`, and `explanation`. The agent selects the kind and material. The phone synthesizes only deliberately published text. Ordinary agent output is not played automatically.

Playback reports distinguish playing, completed, interrupted, failed, and unavailable output. `playedMs` is elapsed playback time reported by the phone; it is approximate. Publication and network send are not proof of listening.

Phone defaults to the phone microphone, keeping Bluetooth playback on its media route. The Voice tab's Phone / External microphone switch applies immediately while listening. External selects a connected Bluetooth communication headset or wired/USB microphone; the selected Bluetooth route is released on stop. Device callbacks handle headset connection changes; without an external microphone, Android selects its normal input. Classic Bluetooth headset microphones require lower-quality call audio. Speech retains media-volume playback in either mode. Live status displays the actual route, capture state, signal, recorded/sent segments, input decisions, and reasoning states separately.

Phone Settings offers Lower other audio (default), Pause other audio, and Keep other audio unchanged. The player holds transient audio focus only across queued speech, and releases it on completion, interruption, failure, or shutdown. Android controls ducking attenuation and other apps' response; an exact adjustable percentage is unavailable through the ordinary audio-focus API. See [Android audio focus](https://developer.android.com/media/optimize/audio-focus).

The phone segments continuous speech approximately every five seconds and retains a short pre-roll for voice activity detection. Segment boundaries do not end speaking. Speech start interrupts current playback and clears queued speech locally. Segments reach a serial transcription queue; playback receipts remain responsive while transcription runs. Speech captured while disconnected is discarded, and reconnect does not replay output.

Every transcript passes separate typed JEV relevance and completion evaluations. Accepted blocks accumulate without reaching reasoning sessions until JEV judges the input ready and speaking, transcription, and filtering are idle. Silence alone does not release input. The two brief cues use media volume and are skipped at media mute.

Conversation state, handoff relationships, transcripts, published text, pending topics, and playback context persist on the laptop. Raw audio is held in memory during transcription and is not retained by the feature.

## Assistant activation

Phone Settings > Assistant shortcuts requests Android's assistant role. Once selected, Android assistant gestures and compatible Bluetooth assistant buttons can activate Orchid. The voice-interaction service handles system sessions and locked-screen activation; `ACTION_ASSIST` and `ACTION_VOICE_COMMAND` also open the activation screen. Caller extras and screen context are ignored.

Activation starts the existing conversation service with the saved microphone preference. Repeated activation reuses its recording and connection. Listening continues until stopped; dismissing the activation screen leaves listening active. Speech passes JEV before accepted, completed input reaches the reasoning stages. The locked activation screen exposes listening controls, while opening settings or history requires unlocking.

Android requires recognition-service metadata for assistant registration. Orchid's registration component rejects generic Android speech-recognizer requests and is not selectable as a default dictation engine; Orchid continues using laptop transcription. The S21's existing Google dictation service remains selected. No custom wake phrase is registered. Garmin/H95 hardware invocation and Bluetooth routing require separate device-button verification.

## Phone history

The same enabled gateway serves authenticated read-only HTTP requests using the WebSocket bearer token:

- `GET /sessions`: main interaction, JEV, summarization, digest, output, and delegated session summaries, including nested delegations.
- `GET /sessions/{id}?before={invocationId}`: up to ten turns, with input, replies, status, and normalized display events. `nextBefore` loads the previous page. Unrelated sessions are rejected.
- `GET /status`: authenticated live capture, transcription, relevance/completion, held-input reason, and reasoning states. The phone sends `capture_status` telemetry once a second while recording; no audio samples are included.

History does not open another WebSocket, activate a microphone, dispatch agent work, or return raw provider transport payloads. It remains available while the phone's voice connection is active.
