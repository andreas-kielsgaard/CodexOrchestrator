# Live voice status and microphone switching

The reported silence had two distinct states: Android was recording and not silencing Orchid, but Phone was the selected microphone; the received “hello, can you hear me? orchid” input was relevant and held because JEV returned `continue`. Connection status alone did not explain either state.

## Changes

- Voice on the phone now has a Phone / External microphone switch. Changing it restarts capture immediately while preserving the connection. External covers Bluetooth, wired, and USB input; the actual route is displayed separately from the requested choice.
- Local capture reports recording, Android silencing, actual route, input level, sample progress, recorded segments, successful WebSocket sends, and capture errors. No audio samples are retained by this telemetry. Reconnection resends current capture and speaking state.
- Authenticated `GET /status` exposes capture telemetry, received/transcribed counts, latest audio transcript, input relevance/completion and the specific held-input reason, queued observations, reasoning states, and stage errors. Capture reports remain transient rather than rewriting durable conversation state every second.
- Desktop Voice Sessions and phone Voice show live status. Phone input and reasoning details remain in a separate expandable section. Stale capture reports cannot continue claiming active recording.
- Completion instructions allow a self-contained question to request an answer, while retaining semantic completion and the gate against continued speaking or pending input. No trigger phrases or silence timeout release input.

## Validation

Eleven focused Rust tests passed, including ignored input and the held-input/filtering/transcription/error distinction. The frontend test for microphone fallback and failed reasoning passed. Frontend build and Android debug build/lint passed.

S21 UI clicks selected External while listening. Actual capture reported `External: soundcore Sport X20`, advancing sample counts, `silenced=false`, and one recorded/sent segment. Clicking Phone restored `Phone: SM-G991B` while the connection remained active. Screenshots showed the switch and actual route. This verifies routing and read activity, not the user's hearing or speech recognition accuracy.

Receipts are retained under the main checkout's `.dev/voice-pipeline-main/` and the feature worktree's `.dev/voice-status-*` logs.

The main release build from source `c52d439f` passed and was relaunched with the existing conversation, stage identities, phone settings, and custom prompt values preserved. Only the unchanged shipped completion prompt was upgraded. Main and phone screenshots agreed on the actual phone route and held-input reason. Authenticated `/status` returned 200, unauthenticated access returned 401, and successive reports showed advancing sample counts and timestamps.

Real JEV queries classified the exact previously held question as `complete` (confidence 0.84), and a promise to add more detail as `continue` (confidence 0.98). This is a two-case check, not an accuracy benchmark. During the route test, a real headset audio segment reached transcription and was classified as unaddressed.

Desktop executable SHA-256: `7E17E1865C5453398C776BB24F2D44AB77311CF990893F1D5EE2FE39C1B613BA`. Installed Android APK SHA-256: `9E2169E9F8838636AD0A7607F638B0C69C86B3A2A217AF7F1A69457B8CB92E14`.

A controlled text question in the relaunched main app passed relevance and completion, released the old held question, and produced the brief summary-stage publication “Yes, your message came through.” Digest avoided a redundant reply. No additional worker was created; all reasoning stages completed and observation queues emptied. This checks the reasoning path with text, separately from the real headset audio/transcription observation. The gateway and phone were left connected with the original Phone microphone selection; the External switch remains available.
