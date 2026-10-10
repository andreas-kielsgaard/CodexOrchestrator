# Phone activation follow-through

Keep Google wake-word detection and Android permissions in phone settings. Orchid's existing `GoogleAssistantHandoff` owns the assistant-overlay handoff; `AssistantActivity` owns activation and reuses `ConversationService` for capture and connection.

Adapt `MainActivity` so Listen has fixed controls and its own scrolling transcript. Keep the existing drawer and shared `ListeningTranscriptView`. Other pages retain their scrolling container. This removes the nested-scroll behavior that hides Start/Stop after visiting Status.

Refresh Listen's connection label alongside its transcript, avoiding the diagnostics page's three-second refresh delay. Scroll the shared transcript directly to its latest text without moving keyboard focus.

The installed Gemini overlay uses `assistant_robin_floaty_root_container`. Extend the existing handoff recognizer to that observed marker and focused application windows, retaining the Google package check so normal Google settings and search remain available.

Verify the installed phone UI, authenticated capture, repeated activation, reconnect, classified transcript, and Google handoff on unlocked and locked screens. Preserve the separate worktree's debugging experiment. No phone-maintenance tooling belongs in this change.

## Verification on the S21, 2026-10-10

- Android build and lint passed. The updated APK was installed through the existing authenticated ADB transport.
- Final APK SHA-256: `009D48EB3D4FF0A92557F67C2D34DAD17319BE5759F4822DB9CB90DF62A93973`, matching the installed package. A retained copy and phone screenshots are under the main checkout's `.dev/voice-activation-final/`.
- Start from Status opened Listen, connected, and captured through the unsilenced phone microphone. Repeated Start retained capture. Start/Stop stayed visible in the reviewed landscape layout.
- The installed connection client recovered from gateway interruption in 4,140 ms, receiving two authenticated version-2 hellos. The existing conversation and reasoning session identities were retained.
- Live microphone speech produced revisable transcript blocks and JEV classifications. Ambient speech remained unaddressed. The transcript probe passed revisions, removals, sticky colors and the rolling limit.
- With Google selected as default assistant and Orchid's handoff/accessibility switches enabled, Android's assistant key opened Gemini and automatically handed off to Orchid. This also passed with keyguard showing and input restricted; capture connected and was not silenced. This exercises assistant activation, not acoustic wake-word recognition or a physical Garmin button.
- The final installed build reached fresh, connected, unsilenced recording in 1.7 seconds after assistant activation while locked. Capture-state reads and screenshots verified this without UI automation. Android UI automation [suppresses accessibility services by default](https://developer.android.com/reference/android/app/UiAutomation#FLAG_DONT_SUPPRESS_ACCESSIBILITY_SERVICES); an earlier UI dump interfered with the handoff test.
- A previously failed JEV evaluation passed when replayed. Its old error cleared when the gateway was restarted. No provider validation changes were made; the cause of the original invalid answer remains undetermined.

Hey Google was off. Google requires four Voice Match recordings before enabling it; enrollment and a spoken wake-word test remain pending. The path on this phone is **Gemini settings → Talk to Gemini hands-free → Hey Google and Voice Match → Hey Google**. Long-window CPU transcription latency remains unchanged.
