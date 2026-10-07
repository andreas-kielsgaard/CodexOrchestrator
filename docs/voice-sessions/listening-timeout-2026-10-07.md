# Listening inactivity timeout

The phone stops its microphone after 60 seconds without JEV-confirmed relevant input. Phone Settings > Listening timeout saves a positive duration in seconds and applies it to the current window. Accepted unfinished input renews the window; unrelated speech and agent replies do not. The connection remains available for output. App Start listening and assistant activation resume capture; reconnection and microphone selection do not.

## Validation

Eleven focused Rust tests and Android debug build/lint passed. The installed S21 instrumentation probe tested the monotonic default deadline, relevant-input renewal, ignored transport/output events, duration changes, and late input after stop. With a five-second setting, actual recording stopped twice, stayed stopped, and resumed through both the app's Start listening control and the assistant activation activity. Microphone selection while inactive did not reopen capture. The probe restored the previous timeout setting.

The assistant activity test does not establish Garmin or headset hardware-button invocation.

The rebuilt main executable preserved conversation `59e0eaa4-0e68-452d-8fcb-0b61d7e4a1a0` and all reasoning-stage identities. With the 60-second setting, a real JEV `direct_request` decision renewed the phone countdown to 59 seconds. A text block addressed to Sam was `unaddressed` and did not renew it. Actual capture then stopped while the WebSocket stayed connected. Another accepted request and published acknowledgement while inactive did not reopen capture. These controlled requests used text to isolate the timeout from speech recognition; receipts are in `.dev/voice-pipeline-main/timeout-integration.json`.

The subsequent phone speech check exposed poor capture: one segment reached the laptop but produced an empty transcript, followed by an attempt with no detected segments. Android reported neither microphone muting nor privacy silencing. Phone capture now uses the recognition source without the additional noise suppressor, and normal WebRTC detection replaces its most aggressive mode. The assistant screen shows actual route, a signal meter, detected speech, sent segments, and remaining time. The Voice tab distinguishes empty transcription and transcription errors from JEV decisions. Signal telemetry uses the strongest frame RMS within each reporting interval.

The user confirmed that the new meter moved and sent-segment count increased. Actual phone audio containing the test question reached transcription and JEV `direct_request`/`complete`, and the summary stage published “Yes, your test message came through.” Recognition rendered Orchid imperfectly as “Or could”; background speech was also captured, so this establishes the functioning path rather than transcription or relevance accuracy. The installed capture changes passed Android build/lint and the real recording inactivity/reactivation probe again. No raw audio was retained.

Main release executable was built from `f5e6e364` (later changes affect Android and documentation), SHA-256 `A5956D2C1C731F1A8CAE6FD14F436659C9C81A8027CCDEAA3FA3F5EA7E038C81`. Installed Android APK SHA-256: `04AC715BA40851A99B0117DDF7EF338BC4BCF528D940C0A506230C72B434E578`. The current timeout remains 60 seconds with Phone selected.
