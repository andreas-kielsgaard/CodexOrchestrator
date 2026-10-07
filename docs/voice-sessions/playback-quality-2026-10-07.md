# Playback quality during recording

The connected H95 exposes a classic Bluetooth SCO microphone, not a BLE Audio microphone. The S21's Bluetooth log reports wideband call audio at 16 kHz when that microphone is active. Its normal media route uses stereo A2DP. The earlier headset-microphone fix switched playback onto SCO along with capture, explaining the reported quality drop.

Settings now offers Phone microphone (preserve playback quality) and Headset microphone (Bluetooth call quality). Phone is the default. It explicitly selects the built-in microphone and leaves communication routing unchanged. Headset mode retains the existing headset selection and cleanup. Restart listening after changing the setting. Background-audio focus and media-volume behavior remain active in both modes.

The installed S21 phone-microphone probe passed: routed input was built-in device 15, capture was not silenced, and speech completed while input stayed on that device. Android policy showed active built-in microphone capture with Bluetooth A2DP playback rather than SCO. Spotify was still ducked while Orchid spoke. The user confirmed that audio sounded good in this test. Audio was discarded locally, and the probe restored the previous microphone preference and retained media volume.

App/test APK builds and lint passed. Evidence is retained under `.dev/phone-history/`: `quality-phone-probe.txt`, `quality-phone-playing-audio.txt`, `quality-phone-playing-policy.txt`, and `quality-bluetooth.txt`. This verifies the phone/headset combination used here; acoustic recognition quality with the phone in a pocket and physical headset reconnect behavior remain untested.

Android documents higher-quality two-way audio through [BLE Audio](https://developer.android.com/develop/connectivity/bluetooth/ble-audio/audio-recording). That route was not available on the connected H95, so selecting the phone microphone is the verified way to retain its media playback quality.
