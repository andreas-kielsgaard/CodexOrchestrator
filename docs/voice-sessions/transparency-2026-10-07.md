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
