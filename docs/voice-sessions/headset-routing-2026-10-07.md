# Headset microphone and speech focus

The earlier S21 check found Bluetooth playback but built-in microphone capture. `AudioCapture` now owns a headset communication route while listening. On Android 12 and later it selects a Bluetooth SCO or BLE headset with `setCommunicationDevice`; older supported versions use SCO. Connection callbacks update the selection. Stopping or failing capture releases the selection and Orchid's communication-mode request. Speech still uses media-volume attributes.

Speech now requests transient audio focus only while its queue plays. Settings offers Lower other audio, Pause other audio, and Keep other audio unchanged. Lower is the default. The requested 50% adjustable reduction cannot be supplied by Android's ordinary audio-focus API: Android chooses attenuation, and other apps can pause instead. The UI describes that limit rather than showing an ineffective percentage slider.

## S21 verification

- Android app/test APK builds and lint passed.
- The installed production capture selected Beoplay H95, routed device 13071, type 7 (Bluetooth SCO), and was not silenced.
- Speech completed while that same microphone route stayed active. Android audio policy independently showed the H95 SCO output and headset input simultaneously active.
- During speech, Orchid held `GAIN_TRANSIENT_MAY_DUCK`. Android identified Spotify's active player as ducked. After speech, the ducked-player list was empty and Orchid no longer held focus.
- Capture stopped, communication mode returned to normal, and the probe's media-volume reading returned to its original value after Android's asynchronous route restoration.
- Probe audio was discarded locally; it was not sent to the laptop. The phone's endpoint/token were retained.

Evidence is retained under `.dev/phone-history/`: `headset-duck-probe.txt`, `headset-duck-playing-audio.txt`, `headset-duck-playing-policy.txt`, and `headset-duck-after-audio.txt`. Physical headset disconnect/reconnect and the visible settings layout remain unverified; the phone was locked during testing. No desktop production source changed.
