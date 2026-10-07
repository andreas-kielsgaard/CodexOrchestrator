# Phone history and media volume verification

The voice input agent interprets speech fragments across turns and active-turn steering. It formulates work requests only after recognizing a complete request explicitly addressed to it. The feature commit `431803f8` was merged into main as `eddb6ef8`.

Phone changes in `84cbfe38` add Voice, Sessions, and Settings tabs using the desktop's dark surfaces and green accents. Sessions separates the main interaction from delegated workers. Details show requests, replies, status, errors, expandable activity, earlier turns, and the main conversation's original speech fragments and published output. Browsing history does not open the microphone. History uses the existing gateway token and is limited to the current voice conversation and its delegates.

Phone speech uses the media stream. Communication-mode speaker forcing was removed. At media volume zero, queued speech is suppressed and reports zero playback instead of claiming it was heard.

## Passed checks

- Android debug APK and test APK compilation; Android lint.
- Native release build, including frontend typecheck/build; Rust check and all four focused voice-session tests.
- Real input-agent test: ambient speech and an incomplete addressed request produced no delegation. The continuation produced exactly one independently executing worker, whose completed result returned to the input agent.
- Installed S21 audio probe: media routing, zero-volume suppression, zero playback receipt, and unchanged media volume.
- Installed S21 history probe over Tailscale/mobile data: main and worker histories loaded from an isolated Orchid instance; missing credentials and unrelated session requests were rejected.
- Installed S21 history probe against the final primary executable: saved endpoint/token retained; current main history loaded; missing credentials and unrelated session requests rejected.
- Root executable launched with normal application storage. Voice Sessions navigation and its reasoning-session link worked. All 42 stored sessions remained; SQLite quick check passed, schema 66. The existing main conversation retained 13 spoken-exchange entries.

Desktop executable SHA-256: `A16049C28A3A47B39D53229062528AD428EAF3A7F1DA591454FA52ED2EE7FE68`. Its production source is `84cbfe38`; `a83a0b1c` changes only probe targeting/synchronization. Installed APK SHA-256: `43E152D9201C8A99D7B38323430C367D3BD9957CC4CD5F724F00102032946266`.

## Remaining verification

An initial phone screenshot confirmed the dark layout and muted-volume status. The final APK is installed, but the S21 locked during testing. Final tab navigation, scrolling, section expansion, and layout review remain pending an unlocked phone. Audible speech at nonzero volume, headset routing, and acoustic microphone/echo behavior were not tested.

The normal desktop gateway is running at the phone's existing port 4818. The isolated test gateway/application were stopped. Database/executable backups, command receipts, and screenshots are retained under `.dev/phone-history/`; build logs are under `.dev/`.

## Bluetooth check at 10:11 local time

The S21 had a connected Beoplay H95, with both A2DP playback and a SCO headset microphone available. A test-only instrumentation probe exercised the installed production `SpeechPlayer` and `AudioCapture`; captured audio was discarded locally.

- Speech completed with media volume unchanged. Android's playback log recorded the test's Google TTS player using `USAGE_MEDIA`, routed to device 13073. Audio policy identified that device as the connected H95's Bluetooth A2DP output.
- The production recorder returned routed device 15, type 15 (`TYPE_BUILTIN_MIC`), product `SM-G991B`. Recording was not silenced. Audio policy independently showed the active Orchid recording client using `AUDIO_DEVICE_IN_BUILTIN_MIC`, address `bottom`.

Therefore Bluetooth speech playback works with this headset, but Orchid currently uses the phone microphone rather than the headset microphone. This check does not establish acoustic intelligibility or reconnection behavior. The probe stopped playback and capture afterward; no production code or volume settings changed. Evidence: `headset-probe.txt`, `headset-playing-audio.txt`, and `headset-playing-policy.txt` under `.dev/phone-history/`.
