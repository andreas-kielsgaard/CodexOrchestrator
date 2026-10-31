# Voice Sessions

An exploratory handsfree companion for Orchid. JEV filters speech before persistent summarization, routing, digest, and output sessions handle it. These sessions remain visible in Agent Sessions and phone history.

## Setup

1. Sign in to Tailscale on the laptop and Android phone. Leave the laptop awake with Orchid running on its usual Wi-Fi.
2. Run `npm run setup:voice` once on Windows. It installs a Python environment and the multilingual Whisper base model under `%LOCALAPPDATA%/Orchid/voice-speech`. Transcription runs locally on CPU. No speech API key is used.
3. Build Orchid with `npm run build -- --debug` and run the resulting executable with `TYPESAFE_API_KEY` available for JEV. In **Voice Sessions**, enter the laptop's numeric Tailscale address with port `4818`, select a laptop Capability Profile, and save. A working directory is optional.
4. Run `npm run build:android` and `npm run deploy:s21` while authenticated wireless ADB is available. The deploy helper verifies S21 serial `R3CRA0BAYXD`.
5. Copy the endpoint and access token into **Orchid Voice**. The app uses an installed offline English Android text-to-speech voice. Install one through Android's text-to-speech settings if prompted.
6. Start the desktop conversation, then press **Start listening** on the phone and approve microphone and notification permissions. Start while the activity is visible. The foreground notification keeps the active conversation available with the screen off.

Android builds require Java 17 or later and Android SDK 36. The wrapper pins Gradle 9.6.1 and the project uses Android Gradle Plugin 9.2.0. `scripts/android/build.ps1` discovers the installed Adoptium JDK and default Android SDK, respecting explicit `JAVA_HOME` and `ANDROID_HOME`.

## Manual phone debugging

Open **Settings → Debugging connection** in Orchid Voice. While authenticated ADB is available, grant the one-time permission shown on that screen: `adb shell pm grant dev.orchid.voice android.permission.WRITE_SECURE_SETTINGS`. Orchid also requests Nearby Wi-Fi permission (location on Android 10–12).

Keep Tailscale connected, turn the phone's Mobile Hotspot off, and press **Start debugging connection**. Keep Orchid visible. It reuses TCP 5555 if available; otherwise it discovers Android's TLS debugging endpoint and, without a joined Wi-Fi network, requests a temporary local hotspot and attempts the experimental debugging-setting workaround for up to two minutes. A setting value alone is never reported as success. The screen shows an endpoint only after finding a local listener; the computer must authenticate with `adb connect <endpoint>` to verify debugging access. Pair through Android developer options if necessary.

Closing or cancelling the screen releases Orchid's temporary hotspot and stops the attempt. After authenticating, `adb -s <endpoint> tcpip 5555` can establish the separate TCP listener for continued Tailscale/mobile-data access. This feature has no Tasker or Shizuku dependency and does not run automatically. Mobile-only startup is device-dependent and may fail on stock firmware.

S21 validation, 2026-10-10: permission setup and existing TCP 5555 discovery passed. With Wi-Fi disconnected and a separate TCP 5556 transport retained for inspection, Orchid created its local hotspot successfully, but Android rejected wireless debugging without a joined network. The attempt timed out after two minutes with no TLS listener. Cancellation released the hotspot while the authenticated TCP connection remained usable. Mobile-only bootstrap is therefore **not working on this S21**; TLS discovery after startup and recovery after reboot remain unverified.

For an authenticated connection and installed offline-voice check without microphone access, build with `powershell -File scripts/android/build.ps1 -ConnectionProbe`, install the generated Android test APK, and run `adb -s <verified-transport> shell am instrument -w dev.orchid.voice.test/dev.orchid.voice.ConnectionProbeInstrumentation`. The desktop gateway must already be enabled. This check uses the app's saved setup and does not verify recording or playback.

Add `-e reconnect true` to test the actual Android connection client's recovery. After the connected status appears, interrupt the desktop process, restart it. An enabled gateway resumes automatically; explicitly stopping the connection keeps it stopped on later launches. The probe waits up to 40 seconds for each loss/recovery stage and closes its connection afterward; it does not open the microphone.

## Use

Speak normally and address requests explicitly to Orchid. The laptop transcribes a continuous rolling audio buffer into revisable sentence blocks. JEV evaluates the recent sentence window every two new sentences or after a two-second voice pause. Change those thresholds and the one-minute context window under **Phone Settings > Input processing**. A brief listening cue marks the first accepted sentence; accepted relevance is sticky. Semantic completion dispatches immediately, even during capture. Listening continues for 30 seconds after delivery, adjustable in the same settings. JEV separately evaluates continuations and corrections so they can accumulate and be delivered again. Failed filtering reports an error and does not bypass JEV.

The summarizer first acknowledges receipt briefly. The router delegates research and execution, keeping dated handoffs across interactions. User input, summaries, progress, and results reach the digest, which decides when to request output and supplies a brief reason plus references. The output session retrieves relevant material and drafts the spoken response. Relevant work can continue in delegated sessions while you discuss another topic.

The agent announces available material before giving a full report. Invite the explanation. Speaking during playback stops current and queued phone speech; the next reasoning input includes the interruption position and relevant material. Playback positions are approximate elapsed milliseconds, not word alignment.

The phone opens on **Listen**, with Start/Stop controls and unfiltered, revisable transcript text. Start listening returns here from **Status**. Swipe inward from the left edge or tap the menu button to open the drawer containing **Listen**, **Sessions**, **Settings**, and **Status**. Sessions separates the main interaction, reasoning stages, and delegated workers, including further delegations. Open a session to see its input, replies, status, and expandable activity; load earlier turns for older history. History refreshes while visible and does not start the microphone. Desktop Voice Sessions also shows the accepted buffer and recent JEV decisions. Enable Voice Sessions once on the laptop; the gateway restores that choice after restart. The phone microphone starts only after an authenticated connection and explicit phone activation.

**Status → Live status** has a Phone / External microphone switch that applies immediately. The actual recording route and signal appear below it. The phone and desktop show audio packets reaching transcription, JEV decisions, why accepted input is held, and reasoning states/errors. Open **Input and reasoning details** on the phone for more detail.

**Settings → Hey Google handoff** contains an experimental automatic bridge. With Google as the default assistant, Hey Google enabled, Orchid's handoff switch on, and **Orchid Google handoff** enabled in Android Accessibility, it dismisses recognized Google assistant overlays and opens Orchid listening. Ordinary Google app windows are excluded. Google overlay recognition and locked-screen activation require device verification; this bridge is off by default.

Listening stops after one minute without JEV-confirmed relevant input. Change the duration under **Settings → Listening timeout**. Voice shows the countdown and stop reason. Start listening or activate the assistant to resume. Replies can still arrive with capture stopped. A received audio segment that produces no words is displayed separately and never reaches JEV.

The assistant activation screen keeps Start listening, Stop listening, Open Orchid, and Dismiss above a rolling view of the last 20 unfiltered transcript blocks. Sentence text updates during speech and can be revised after classification and delivery. Pending input is neutral; JEV colors requests green, task context blue, and unaddressed speech gray. It follows new input while at the bottom and lets you scroll back. Diagnostic and explanatory text remains in the main app.

Phone speech follows Android media volume and normal media routing. At media mute, speech is skipped with a zero-duration `muted` receipt; its text remains in history. The app's volume buttons control media volume.

**Stop** on the phone stops microphone capture, playback, and its connection. **Stop connection** on the desktop also closes the gateway and local transcription worker. Independent agent work retains its normal session lifetime. Resume reconnects the persistent conversation and its reasoning stages; it does not replay audio. **New conversation** starts new stage sessions.

Disconnected microphone audio is discarded. Keep Tailscale enabled on mobile data. Use the phone speaker for the initial test; headset routing, wind noise, echo cancellation, and call interruption require testing with the actual audio setup.

## Instructions and tools

The tab owns editable [routing preferences](../../resources/voice-sessions/instructions.md). Stage instructions accompany deliveries and active-turn steering: [router](../../resources/voice-sessions/input-instructions.md), [summarizer](../../resources/voice-sessions/summary-instructions.md), [digest](../../resources/voice-sessions/digest-instructions.md), and [output](../../resources/voice-sessions/output-instructions.md). JEV questions and the dispatch gate belong to `voice_sessions/prefilter.rs`; reasoning behavior belongs to these instructions. See the [implementation shape](../architecture/voice-prefilter-pipeline-implementation-shape.md).

In desktop **Voice Sessions > Phone setup > Voice prompts**, select a stage or shared tool discovery guidance to edit its prompt. Stop the connection before editing, then **Save setup**. Changes persist and apply to subsequent JEV evaluations and reasoning turns; **Reset this prompt** restores its shipped default before saving. JEV choice labels and publication permissions remain application contracts.

The selected profile supplies the runtime and normal capabilities. Conversation creation explicitly adds `orchid_control` and `voice_session` product tools to its pinned Session Profile. Managed MCP requests use Harness-supplied caller identity. `orchid_control` discovers and operates sessions across the same Orchid instance; `voice_session` records handoffs, observations, pending topics, and deliberate speech.

Existing runtime approval policies still apply. Worker requests are delivered as observations and can be answered by reasoning using the existing typed request tool. A request that blocks the main reasoning invocation may require desktop intervention in this prototype.

## Speech choice

The native Codex 0.159.0 realtime probe returned `realtime conversation requires API key auth` with the current ChatGPT sign-in. OpenAI speech API use was declined. Claude Code dictation requires Claude.ai authentication and a local microphone; its documented interface does not provide this phone audio bridge. The Job Agent's Anthropic API key cannot enable that dictation service.

This slice therefore uses one speech path: Android WebRTC voice activity detection, local faster-whisper transcription, and offline Android text-to-speech. Audio stays between the phone and laptop over Tailscale; transcripts reach the selected reasoning provider as ordinary session input.

See [protocol](protocol.md) and [validation evidence](validation.md).
