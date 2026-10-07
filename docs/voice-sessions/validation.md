# Voice Sessions validation

Live capture/pipeline visibility and the immediate microphone switch are covered by [transparency validation](transparency-2026-10-07.md).

The JEV prefilter, separate reasoning stages, and editable desktop prompts are covered by [pipeline validation](pipeline-2026-10-07.md). Earlier evidence below describes the previous voice flow.

Status: desktop reasoning, S21 microphone capture, mobile-data connection, TTS playback telemetry, screen-off operation, and interruption exercised. Audible quality and walking acceptance remain pending. See [the actual-app usability test](usability-2026-10-07.md) for findings and changes.

## Confirmed

- Developed on `feature/reasoned-voice-workflow` in worktree `17c9f61d-635e-4e7f-9015-b917e07306ba`; fast-forwarded into `main` at `ea55402d`.
- Native Codex 0.159.0 accepted the experimental realtime request, then rejected the current account with `realtime conversation requires API key auth`.
- The user declined OpenAI speech API use. Claude Code's documented dictation does not accept the Job Agent API authentication.
- Tailscale laptop `100.105.52.29` and S21 `100.90.249.49` are signed in and reachable. Home Wi-Fi Tailscale ping succeeded; actual S21 app traffic over mobile data was subsequently exercised in the usability test.
- S21 serial `R3CRA0BAYXD`, model `SM-G991B`, Android 15/API 35. Debug APK installed and its activity launched through authenticated wireless ADB without USB.
- Android project builds with Gradle 9.6.1, AGP 9.2.0, Java 21, and Android SDK 36.
- Desktop frontend typecheck/build and native runnable debug build passed. Voice persistence/private-address checks passed (2 tests). Related Agent Session checks passed (66 tests; 1 pre-existing ignored test), including pinned delivery behind an active invocation.
- Navigation/composition frontend checks: 3 files, 23 tests passed.
- Local CPU transcription of synthesized test speech returned the exact sentence. Cold request about 3.46 seconds; warm request about 1.26 seconds. This is a controlled test, not a walking/noise benchmark.
- The installed Android app authenticated to the desktop over Tailscale and received the version-1 WebSocket hello. A custom Android instrumentation probe used the app's context and saved credentials with the phone locked. The same probe found 17 installed offline English voices; it did not play audio or open the microphone. APK SHA-256: `48B47BD595836EA9DAE37DFE29A0E93873AF01D2962E11F63AD4D2E1C5F19537`.
- An invalid WebSocket token was rejected with HTTP 401. Temporary Tailscale Serve experiments were removed; the final connection uses the gateway bound directly to the laptop's Tailscale address.
- Live Codex reasoning started two independent workers. A protocol worker delegated a third transcription-script inspection. The main conversation answered another question while work continued. Runtime histories preserve each independent invocation and application-provenance returns.
- After an explicit direction change, reasoning steered the protocol worker and discovered/observed an independent session outside the delegation relationships. That session's result triggered a readiness announcement and retained material.
- A second nested handoff exercised the complete continuation path: parent invocation `666069c3-5402-4741-8be8-e1ea87cde94f` yielded with progress, nested invocation `9220578d-f283-46c4-8d3e-06d83e3dec9e` completed, and parent invocation `d7761176-e626-4469-b400-3fe03a75e641` resumed with observations and returned combined findings. Main reasoning announced readiness without reading the full report automatically.
- A controlled WAV invitation crossed the WebSocket, local transcription, reasoning, and deliberate publication path. It produced the transcript “Please tell me the readme finding that is ready.” and a concise explanation. This validates publication, not phone playback or user hearing.
- Desktop restart retained the conversation, pending topics, and handoff relationships. Explicit resume restored the gateway and real reasoning continued with the refreshed managed MCP host.
- Screenshot review covered empty setup and active conversation in the dedicated tab. Source material is available on demand under each ready topic.
- The main release executable launched with normal application storage; migration and native/rendered checks passed. The S21 authenticated over LTE using saved settings. Live main-instance reasoning, independent workers, concurrent input, readiness announcements, and invited publication passed. A fresh conversation was left ready for use. See [main integration](main-2026-10-07.md).

## Remaining

- User confirmation of audible output and voice quality; physical retest of the corrected interruption status.
- Foreground microphone-service recovery during a network drop; the Android connection client itself passed abrupt desktop-loss/recovery testing.
- Audible outdoor walking test with the user's actual microphone/headset setup.

Development evidence is retained under the ignored `.dev/voice/` directory. Development app launches use explicit executable, PID, debug endpoint, and isolated storage. A copied test database is not the live controller's current state.

The exploratory desktop instance was closed when the user stopped usability testing. Phone capture remains stopped. The main release executable and desktop smoke verification are recorded in [main integration](main-2026-10-07.md). No further phone usability testing was performed.
