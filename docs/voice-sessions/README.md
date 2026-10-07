# Voice Sessions

An exploratory handsfree companion for Orchid. A persistent reasoning session interprets speech, hands off independent work, and decides when to announce and explain results. These sessions remain visible in Agent Sessions.

## Setup

1. Sign in to Tailscale on the laptop and Android phone. Leave the laptop awake with Orchid running on its usual Wi-Fi.
2. Run `npm run setup:voice` once on Windows. It installs a Python environment and the multilingual Whisper base model under `%LOCALAPPDATA%/Orchid/voice-speech`. Transcription runs locally on CPU. No speech API key is used.
3. Build Orchid with `npm run build -- --debug` and run the resulting executable. In **Voice Sessions**, enter the laptop's numeric Tailscale address with port `4818`, select a laptop Capability Profile, and save. A working directory is optional.
4. Run `npm run build:android` and `npm run deploy:s21` while authenticated wireless ADB is available. The deploy helper verifies S21 serial `R3CRA0BAYXD`.
5. Copy the endpoint and access token into **Orchid Voice**. The app uses an installed offline English Android text-to-speech voice. Install one through Android's text-to-speech settings if prompted.
6. Start the desktop conversation, then press **Start listening** on the phone and approve microphone and notification permissions. Start while the activity is visible. The foreground notification keeps the active conversation available with the screen off.

Android builds require Java 17 or later and Android SDK 36. The wrapper pins Gradle 9.6.1 and the project uses Android Gradle Plugin 9.2.0. `scripts/android/build.ps1` discovers the installed Adoptium JDK and default Android SDK, respecting explicit `JAVA_HOME` and `ANDROID_HOME`.

For an authenticated connection and installed offline-voice check without microphone access, build with `powershell -File scripts/android/build.ps1 -ConnectionProbe`, install the generated Android test APK, and run `adb -s <verified-transport> shell am instrument -w dev.orchid.voice.test/dev.orchid.voice.ConnectionProbeInstrumentation`. The desktop gateway must already be enabled. This check uses the app's saved setup and does not verify recording or playback.

Add `-e reconnect true` to test the actual Android connection client's recovery. After the connected status appears, interrupt the desktop process, restart it, and resume its gateway. The probe waits up to 40 seconds for each loss/recovery stage and closes its connection afterward; it does not open the microphone.

## Use

Speak normally. Completed utterances are transcribed on the laptop and reach reasoning. No vocabulary of hardcoded commands is required. Relevant work can continue in delegated sessions while you discuss another topic.

The agent announces available material before giving a full report. Invite the explanation. Speaking during playback stops current and queued phone speech; the next reasoning input includes the interruption position and relevant material. Playback positions are approximate elapsed milliseconds, not word alignment.

**Stop** on the phone stops microphone capture, playback, and its connection. **Stop connection** on the desktop also closes the gateway and local transcription worker. Independent agent work retains its normal session lifetime. Resume reconnects the persistent conversation; it does not replay audio. **New conversation** creates a new ordinary reasoning session.

Disconnected microphone audio is discarded. Keep Tailscale enabled on mobile data. Use the phone speaker for the initial test; headset routing, wind noise, echo cancellation, and call interruption require testing with the actual audio setup.

## Instructions and tools

The tab owns an editable copy of [the default instructions](../../resources/voice-sessions/instructions.md). Changes apply to subsequent reasoning deliveries. Readiness, work division, and reactions to steering are agent decisions.

The selected profile supplies the runtime and normal capabilities. Conversation creation explicitly adds `orchid_control` and `voice_session` product tools to its pinned Session Profile. Managed MCP requests use Harness-supplied caller identity. `orchid_control` discovers and operates sessions across the same Orchid instance; `voice_session` records handoffs, observations, pending topics, and deliberate speech.

Existing runtime approval policies still apply. Worker requests are delivered as observations and can be answered by reasoning using the existing typed request tool. A request that blocks the main reasoning invocation may require desktop intervention in this prototype.

## Speech choice

The native Codex 0.159.0 realtime probe returned `realtime conversation requires API key auth` with the current ChatGPT sign-in. OpenAI speech API use was declined. Claude Code dictation requires Claude.ai authentication and a local microphone; its documented interface does not provide this phone audio bridge. The Job Agent's Anthropic API key cannot enable that dictation service.

This slice therefore uses one speech path: Android WebRTC voice activity detection, local faster-whisper transcription, and offline Android text-to-speech. Audio stays between the phone and laptop over Tailscale; transcripts reach the selected reasoning provider as ordinary session input.

See [protocol](protocol.md) and [validation evidence](validation.md).
