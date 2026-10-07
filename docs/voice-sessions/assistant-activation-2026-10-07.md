# Android assistant activation

Implemented on `feature/android-assistant-activation` for the Samsung S21 (`SM-G991B`). Orchid can hold Android's assistant role, handle assistant and voice-command intents, and receive voice-interaction sessions and locked-screen launches. Settings includes assistant selection. The activation screen starts the existing service and offers stop, dismiss, and unlock-to-open controls; it does not expose history or credentials.

## Verification

- Android debug build and lint passed.
- Installed on the S21 using its existing Tailscale ADB transport.
- Android accepted Orchid as assistant without bypassing role qualification, bound `OrchidAssistantService`, and registered `OrchidAssistantSessionService`.
- System `KEYCODE_ASSIST` invocation launched the activation screen and reached `Listening · connected` while device trust diagnostics reported `deviceLocked=1`.
- Explicit `ACTION_ASSIST` and `ACTION_VOICE_COMMAND` reached the same activation screen. Repeated activation retained one foreground conversation service.
- The installed `assistant` instrumentation probe passed: one unsilenced recording, authenticated connection, repeated activation retained the same recording session, microphone preference and media volume unchanged, stop released recording and reached `Stopped`.
- Screenshot review confirmed readable padding, separate controls, and no credentials/history on the locked activation screen.
- The phone's assistant is now Orchid; its existing Google dictation provider remains selected. Microphone remains `phone` and speech ducking remains `lower`.

Local evidence: `.dev/phone-history/assistant-system-service.txt`, `assistant-key-event-service.txt`, `assistant-system-activation.png`, and `assistant-probe.txt`.

## Limits

The Garmin model is unspecified. Garmin documents Phone Assistant as communication with the phone assistant via Bluetooth, but third-party Orchid compatibility has not been established by a physical watch-button test. The H95 button also needs physical verification, including whether the accessory starts a Bluetooth call-audio route. This implementation keeps the saved microphone setting; it does not automatically select the watch microphone.

`KEYCODE_VOICE_ASSIST` injection was inconclusive on this Samsung while locked; successful system-session verification used `KEYCODE_ASSIST`. Custom hotword activation and generic Android dictation are not implemented. Only this S21 has been live-tested.

References: [Android voice interaction sessions](https://developer.android.com/reference/android/service/voice/VoiceInteractionSession), [locked-screen assistant activation](https://developer.android.com/reference/android/service/voice/VoiceInteractionService#onLaunchVoiceAssistFromKeyguard()), [Garmin Phone Assistant](https://www8.garmin.com/manuals/webhelp/GUID-C144B465-A0C8-4FE9-AFE6-41A3FE3F1D9A/EN-US/GUID-08BEA35F-C88E-48BC-A814-2AFD3E4A4527.html).
