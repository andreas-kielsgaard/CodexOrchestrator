# Listening inactivity timeout

The phone stops its microphone after 60 seconds without JEV-confirmed relevant input. Phone Settings > Listening timeout saves a positive duration in seconds and applies it to the current window. Accepted unfinished input renews the window; unrelated speech and agent replies do not. The connection remains available for output. App Start listening and assistant activation resume capture; reconnection and microphone selection do not.

## Validation

Eleven focused Rust tests and Android debug build/lint passed. The installed S21 instrumentation probe tested the monotonic default deadline, relevant-input renewal, ignored transport/output events, duration changes, and late input after stop. With a five-second setting, actual recording stopped twice, stayed stopped, and resumed through both the app's Start listening control and the assistant activation activity. Microphone selection while inactive did not reopen capture. The probe restored the previous timeout setting.

The assistant activity test does not establish Garmin or headset hardware-button invocation.
