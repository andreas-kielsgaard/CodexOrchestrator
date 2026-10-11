# Independent phone audio devices

Settings > Audio devices and Status now select microphone and output independently. `AudioDevices` owns connected-device names and preference identity; `AudioDeviceControls` owns the shared selectors. `MicrophoneRouting` pins only capture. `SpeechPlayer` and `CuePlayer` own their AudioTrack output routes. USB keys omit transient ALSA card addresses.

On S21 R3CRA0BAYXD, the installed build passed production capture/playback probes for phone microphone + Beoplay H95 and phone microphone + speaker. Capture was not silenced, audio mode stayed normal, and media volume was unchanged. Disconnected USB preference correctly fell back to the phone while Bluetooth speech played; interruption also passed. Android build and lint passed (0 errors).

MediaPlayer accepted a speaker preference but continued routing through Bluetooth on this phone. AudioTrack honored both tested selections, so speech uses PCM from TTS synthesis callbacks instead. Temporary synthesis files are deleted after playback or interruption.

Android detected `USB-Audio - Wireless Microphone RX` initially, then reported its USB-C port disconnected. USB capture + Bluetooth playback remains unverified on hardware until the receiver reconnects. No product debugging/connection-management feature was added.
