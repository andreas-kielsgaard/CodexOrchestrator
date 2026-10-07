# Voice Sessions usability test, 7 October 2026

Tested the running desktop prototype and installed S21 app, using rendered controls, Android permission dialogs, the notification action, screenshots, and application/runtime observations. The desktop uses isolated test storage, not the production controller database. USB provided device control; phone traffic used Tailscale over LTE with Wi-Fi disabled. The laptop stayed on its home network.

## Exercised

- Desktop setup rejected a wildcard address with a useful error. Correcting it and saving cleared the error. Resume restored the existing conversation after restart.
- A request entered through the desktop composer started two independent read-only agents. Reasoning acknowledged the accepted work, announced each available result, and retained the explanations until invited. The normal Agent Sessions screen exposed the main conversation and workers.
- On the phone, **Start listening** requested microphone and notification permissions, then reached **Listening · connected**. Desktop state independently confirmed the authenticated phone connection.
- A phrase played through the laptop speaker was captured by the actual phone microphone and transcribed as “Tell me the speech setup advice.” Reasoning published the requested explanation; Android reported 18.8 seconds of completed TTS playback. Audibility and voice quality have not been confirmed by the user.
- Speech detected during an availability announcement interrupted playback at about 1.3 seconds. Screen-off capture and interruption also worked while the phone was locked and using mobile data. The longer overlapping question was split and garbled; reasoning asked for clarification.
- The phone notification's **Stop** action was usable while locked. Capture stopped, the connection closed, the service and wake lock disappeared, and audio mode returned to normal. Desktop **Stop connection** also worked through its control.

## Findings and changes

1. **The desktop composer was buried by accumulated work.** With the retained test conversation, its top was 1,785 CSS pixels down. It now appears at 296 pixels. Long task instructions collapse behind a disclosure, and the latest exchange appears first. Rebuilt and rechecked through the running desktop UI, including another submitted request.
2. **The phone kept saying “Speaking” after interruption.** Both the activity and notification retained that status after audio stopped. The player now signals idle after interruption, and the service updates status only while connected. Updated APK installed; its physical interruption/UI retest awaits another unlock.
3. **Recognition during overlapping audio needs more testing.** The controlled longer question became two inputs, including an incorrect transcription of “restart Orchid.” A shorter phrase outside playback was accurate. This experiment does not establish human speech or headset performance.
4. **Acknowledgement has noticeable delay.** The two-worker request took approximately 19 seconds to receive its acknowledgement; the clear spoken report request took about 10 seconds after transcription. There is no immediate audible receipt cue.
5. **Already reported topics remained in “Ready to discuss.”** Topic retirement needs a prompt/interaction decision; it was not changed in this test.

## Evidence and limits

Evidence is retained in ignored `.dev/voice/usability-*` files. Representative captures: [original desktop](../../.dev/voice/usability-stopped.png), [phone setup](../../.dev/voice/usability-phone-launch.png), and [revised desktop](../../.dev/voice/usability-revised-stopped.png). Android hierarchy captures and native receipts record permission, capture, playback, and stop outcomes.

The desktop runnable build, Android assembly and lint, formatting, and diff checks passed. Installed updated APK SHA-256: `3E750D27C81527789035DD16F6A7E56AF50D2D7B853B80D58A42A1B484977F11`.

Outdoor walking, the user's natural speech and audio setup, transient-network reconnect, and audible sound quality remain unverified. Microphone capture was stopped after testing. The desktop prototype remains available with its retained conversation.
