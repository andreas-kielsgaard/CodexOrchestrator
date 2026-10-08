# Phone protocol v2

Connect to `ws://<laptop Tailscale IPv4>:4818/phone` with `Authorization: Bearer <token>`. The server binds to that exact Tailscale address. Loopback is permitted for desktop tests; wildcard, LAN, and public bindings are rejected. Tailscale supplies encrypted device transport. One phone connection is accepted at a time.

All WebSocket data messages are JSON text. Native WebSocket ping/pong maintains transport liveness.

| Direction | Type | Fields |
| --- | --- | --- |
| Laptop to phone | `hello` | `version: 2`, `conversationId`, `mainSessionId`, recent `inputClassifications: [{id,relevance}]` |
| Phone to laptop | `input_settings` | `settings: {sentencesPerQuery,pauseSeconds,contextSeconds,followupSeconds}` |
| Phone to laptop | `audio_stream` | `streamId`, `epochMs`, absolute `startSample`, `hasSpeech`, base64 `pcm`: consecutive mono signed 16-bit PCM packets, little endian, 16 kHz |
| Phone to laptop | `text` | unique `id`, `text` |
| Phone to laptop | `speech_state` | `speaking: true` at speech start, `false` when voice activity ends |
| Laptop to phone | `transcript` | manual text `id`, `text` |
| Laptop to phone | `transcript_window` | `streamId`, `windowStartMs`, revisable sentence `blocks: [{id,text,startMs,endMs,revision,relevance,...}]` |
| Laptop to phone | `relevant_input` | `id`; newly accepted or revised relevant input renews listening |
| Laptop to phone | `input_classification` | `id`, `relevance: direct_request`, `task_context`, or `unaddressed`; updates the color of an already displayed transcript |
| Laptop to phone | `input_released` | `requestId`, `followupSeconds`; start the follow-up listening window |
| Laptop to phone | `input_continuation` | the recent request continues; resume accumulation |
| Laptop to phone | `cue` | `cue: listening` when accepted input opens; `processing` when released |
| Laptop to phone | `speak` | `utterance: {id,kind,text,topicId,createdAt}` |
| Phone to laptop | `playback` or `interrupt` | `playback: {utteranceId,playedMs,state}` |
| Laptop to phone | `error` | `message` |
| Laptop to phone | `ended` | no additional fields |

Publication kinds are `acknowledgement`, `announcement`, and `explanation`. The agent selects the kind and material. The phone synthesizes only deliberately published text. Ordinary agent output is not played automatically.

Playback reports distinguish playing, completed, interrupted, failed, and unavailable output. `playedMs` is elapsed playback time reported by the phone; it is approximate. Publication and network send are not proof of listening.

Phone defaults to the phone microphone, keeping Bluetooth playback on its media route. The Voice tab's Phone / External microphone switch applies immediately while listening. External selects a connected Bluetooth communication headset or wired/USB microphone; the selected Bluetooth route is released on stop. Device callbacks handle headset connection changes; without an external microphone, Android selects its normal input. Classic Bluetooth headset microphones require lower-quality call audio. Speech retains media-volume playback in either mode. Live status displays the actual route, capture state, signal, recorded/sent segments, input decisions, and reasoning states separately.

Phone capture uses Android's `VOICE_RECOGNITION` source without an additional noise suppressor; external capture retains `VOICE_COMMUNICATION` for communication routing. WebRTC uses normal voice detection. Signal telemetry reports the strongest frame RMS since the previous update so brief speech is visible. Android documents the recognition source as avoiding call-oriented gain control and noise suppression: [recording audio](https://developer.android.com/media/platform/mediarecorder).

Phone Settings offers Lower other audio (default), Pause other audio, and Keep other audio unchanged. The player holds transient audio focus only across queued speech, and releases it on completion, interruption, failure, or shutdown. Android controls ducking attenuation and other apps' response; an exact adjustable percentage is unavailable through the ordinary audio-focus API. See [Android audio focus](https://developer.android.com/media/optimize/audio-focus).

The phone sends consecutive audio packets, including silence, approximately once per second. Voice detection controls speaking indicators and playback interruption, not which samples reach transcription. The laptop retains a continuous rolling audio buffer (60 seconds by default). A single local transcriber revisits that whole buffer as audio arrives; superseded decoding requests can be skipped without dropping their samples. Sentence text remains revisable after classification and delivery. A stream gap requires reactivation rather than silently joining unrelated audio.

Phone Settings > Input processing controls sentences per JEV query (default 2), voice pause before a query (2 seconds), recent audio/text context (60 seconds), and listening after delivery (30 seconds). Each query evaluates every sentence in the recent window for relevance and completion, with a separate continuation question when a recent request was delivered. Relevance is sticky: a previously accepted sentence remains accepted. A pause triggers evaluation but does not itself establish semantic completion.

JEV completion releases accepted input immediately, including during active capture. The delivery event starts the follow-up listening window. A continuation or new relevant input resumes accumulation and the next completed request can steer ongoing reasoning through the existing observation flow. Delivery carries the previous request and an explicit continuation relationship when JEV identified one. Delivered sentence revisions prevent repeated delivery of unchanged input. The two brief cues use media volume and are skipped at media mute.

Conversation state, handoff relationships, transcripts, published text, pending topics, and playback context persist on the laptop. Raw audio is held in memory during transcription and is not retained by the feature.

## Assistant activation

Phone Settings > Assistant shortcuts requests Android's assistant role. Once selected, Android assistant gestures and compatible Bluetooth assistant buttons can activate Orchid. The voice-interaction service handles system sessions and locked-screen activation; `ACTION_ASSIST` and `ACTION_VOICE_COMMAND` also open the activation screen. Caller extras and screen context are ignored.

Activation starts the existing conversation service with the saved microphone preference. Repeated activation reuses its recording and connection. Listening stops after 60 seconds without JEV-confirmed relevant input; Phone Settings > Listening timeout changes the duration in seconds, including while listening. Each accepted block renews the window, including unfinished task context. Background speech, transcripts awaiting evaluation, replies, reconnection, and microphone selection do not renew it. Expiry releases the microphone and Bluetooth communication route while retaining the connection for replies. The Voice tab shows the countdown and stop reason. Only app or assistant activation resumes capture; dismissing the activation screen does not stop it. Speech passes JEV before accepted, completed input reaches the reasoning stages. The locked activation screen exposes listening controls, while opening settings or history requires unlocking.

Android requires recognition-service metadata for assistant registration. Orchid's registration component rejects generic Android speech-recognizer requests and is not selectable as a default dictation engine; Orchid continues using laptop transcription. The S21's existing Google dictation service remains selected. No custom wake phrase is registered. Garmin/H95 hardware invocation and Bluetooth routing require separate device-button verification.

## Phone history

The same enabled gateway serves authenticated read-only HTTP requests using the WebSocket bearer token:

- `GET /sessions`: main interaction, JEV, summarization, digest, output, and delegated session summaries, including nested delegations.
- `GET /sessions/{id}?before={invocationId}`: up to ten turns, with input, replies, status, and normalized display events. `nextBefore` loads the previous page. Unrelated sessions are rejected.
- `GET /status`: authenticated live capture, transcription, relevance/completion, held-input reason, and reasoning states. The phone sends `capture_status` telemetry once a second while recording; no audio samples are included.

History does not open another WebSocket, activate a microphone, dispatch agent work, or return raw provider transport payloads. It remains available while the phone's voice connection is active.
