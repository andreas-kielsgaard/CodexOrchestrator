# Phone protocol v2

Connect to `ws://<laptop Tailscale IPv4>:4818/phone` with `Authorization: Bearer <token>`. The server binds to that exact Tailscale address. Loopback is permitted for desktop tests; wildcard, LAN, and public bindings are rejected. Tailscale supplies encrypted device transport. The newest authenticated phone socket replaces the previous socket. Only that socket can clear connected state when it closes.

Each activation allows the initial connection attempt and three automatic retries, one second apart. TCP connection has a five-second timeout; the protocol hello must arrive within ten seconds. Exhausting retries stops the phone service and releases its resources until Start listening or an assistant activation starts again. A successful hello resets the retry budget. Rejected credentials and incompatible protocol versions stop immediately and retain their diagnostic message in the app.

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

Phone Settings > Audio devices and Status offer independent microphone and output selectors, listing connected devices by name. Defaults are Phone microphone and Automatic Android media output. Selections apply immediately; device callbacks reapply them on reconnect. A disconnected microphone falls back to the phone; a disconnected output falls back to Android media routing. Preferences retain device type, address and name rather than transient port IDs. Status displays actual capture and current or last playback routes.

Phone, USB and wired capture use Android's `VOICE_RECOGNITION` source without an additional noise suppressor. Only Bluetooth microphones use `VOICE_COMMUNICATION` and communication-device routing, released on stop. Classic Bluetooth microphone use requires call audio and can affect output routing and quality. USB/phone recording leaves media mode intact. WebRTC uses normal voice detection. Signal telemetry reports peak frame RMS since the previous update. See [recording audio](https://developer.android.com/media/platform/mediarecorder).

Speech uses the offline Android TTS engine's synthesis callbacks to collect PCM, then AudioTrack plays it with media attributes and the selected preferred device. The engine also writes a temporary WAV; completion and interruption release the track and delete that file. Playback receipts measure actual playback rather than synthesis. Short listening/processing cues use AudioTrack with the same output preference. See [AudioTrack](https://developer.android.com/reference/android/media/AudioTrack), [TextToSpeech](https://developer.android.com/reference/android/speech/tts/TextToSpeech).

Phone Settings offers Lower other audio (default), Pause other audio, and Keep other audio unchanged. The player holds transient audio focus only across queued speech, and releases it on completion, interruption, failure, or shutdown. Android controls ducking attenuation and other apps' response; an exact adjustable percentage is unavailable through the ordinary audio-focus API. See [Android audio focus](https://developer.android.com/media/optimize/audio-focus).

The phone sends consecutive audio packets, including silence, approximately once per second. Voice detection controls speaking indicators and playback interruption, not which samples reach transcription. The laptop retains a continuous rolling audio buffer (60 seconds by default). A single local transcriber revisits that whole buffer as audio arrives; superseded decoding requests can be skipped without dropping their samples. Sentence text remains revisable after classification and delivery. A stream gap requires reactivation rather than silently joining unrelated audio.

Phone Settings > Input processing controls sentences per JEV query (default 2), voice pause before a query (2 seconds), recent audio/text context (60 seconds), and listening after delivery (30 seconds). Each query evaluates every sentence in the recent window for relevance and completion, with a separate continuation question when a recent request was delivered. Relevance is sticky: a previously accepted sentence remains accepted. A pause triggers evaluation but does not itself establish semantic completion.

JEV completion releases accepted input immediately, including during active capture. The delivery event starts the follow-up listening window. A continuation or new relevant input resumes accumulation and the next completed request can steer ongoing reasoning through the existing observation flow. Delivery carries the previous request and an explicit continuation relationship when JEV identified one. Delivered sentence revisions prevent repeated delivery of unchanged input. The two brief cues use media volume and are skipped at media mute.

Conversation state, handoff relationships, transcripts, published text, pending topics, and playback context persist on the laptop. Raw audio is held in memory during transcription and is not retained by the feature.

## Assistant activation

Phone Settings > Assistant shortcuts requests Android's assistant role. Once selected, Android assistant gestures and compatible Bluetooth assistant buttons can activate Orchid. The voice-interaction service handles system sessions and locked-screen activation; `ACTION_ASSIST` and `ACTION_VOICE_COMMAND` also open the activation screen. Caller extras and screen context are ignored.

Activation starts the conversation service with the saved microphone preference. Repeated activation reuses its recording and connection while active. Listening stops after 60 seconds without JEV-confirmed relevant input; Phone Settings > Listening timeout changes the duration in seconds, including while listening. Each accepted block renews the window, including unfinished task context. Background speech, transcripts awaiting evaluation, replies, reconnection, and microphone selection do not renew it. Inactivity or follow-up expiry closes capture, playback, the WebSocket and foreground service, removes the notification, and releases the CPU wake lock. Idle Orchid does not ping or retry the laptop; late callbacks cannot restore its notification. The Voice tab retains the stop reason. Only app or assistant activation reconnects and resumes capture; dismissing the activation screen does not stop it. Agent work continues independently, but idle phone announcements need a separate delivery mechanism. Speech passes JEV before accepted, completed input reaches the reasoning stages. The locked activation screen exposes listening controls, while opening settings or history requires unlocking.

Android requires recognition-service metadata for assistant registration. Orchid's registration component rejects generic Android speech-recognizer requests and is not selectable as a default dictation engine; Orchid continues using laptop transcription. The S21's existing Google dictation service remains selected. No custom wake phrase is registered. Garmin/H95 hardware invocation and Bluetooth routing require separate device-button verification.

## Phone history

The same enabled gateway serves authenticated read-only HTTP requests using the WebSocket bearer token:

- `GET /sessions`: main interaction, JEV, summarization, digest, output, and delegated session summaries, including nested delegations.
- `GET /sessions/{id}?before={invocationId}`: up to ten turns, with input, replies, status, and normalized display events. `nextBefore` loads the previous page. Unrelated sessions are rejected.
- `GET /status`: authenticated live capture, transcription, relevance/completion, held-input reason, and reasoning states. The phone sends `capture_status` telemetry once a second while recording; no audio samples are included.

History does not open another WebSocket, activate a microphone, dispatch agent work, or return raw provider transport payloads. It remains available while the phone's voice connection is active.
