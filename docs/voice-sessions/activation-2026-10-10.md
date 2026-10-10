# Listening activation

Implemented on `feature/reasoned-voice-workflow` and merged into main (`926d8fe2`, `8b4e59bb`).

The desktop persists connection intent separately from live gateway state. Normal shutdown retains intent; explicit Stop clears it. An enabled gateway restores on launch with the existing conversation and reasoning sessions. The phone waits for an authenticated hello before recording, retries failed connections promptly, ignores obsolete socket callbacks, and retains listening deadlines across reconnects. Repeated activation reuses an active recording.

The phone opens on Listen with Start/Stop controls and unfiltered, revisable, classified transcript text. Start from Status switches to Listen. A standard navigation drawer replaces visible tabs: swipe inward from the left edge or tap the menu button for Listen, Sessions, Settings and Status. Assistant activation shares the same transcript component.

An experimental Google handoff service recognizes Google assistant system overlays, dismisses them and opens Orchid's assistant activity. It is off by default. It requires Google as default assistant, Hey Google enabled, Orchid's handoff switch enabled and the Orchid Google handoff accessibility service enabled. Overlay recognition and locked-screen operation are unverified on the phone.

## Verified

- Seventeen focused Rust voice tests passed, including legacy connection-intent migration and explicit Stop persistence. Android debug build and lint passed.
- The release executable was launched from main and checked through Orchid native commands and the rendered Voice Sessions screen. Normal close/relaunch restored the gateway automatically. Explicit Stop followed by close/relaunch left it stopped. Conversation and pipeline identities were retained, with no runtime errors.
- Final desktop gateway is enabled on `100.105.52.29:4818`. Executable SHA-256: `03B3F4914BFF5107E930D41C28A341DDB0BC1E94C8B87DE124CA1DDE43127E2E`.
- The prepared APK is `apps/orchid-android/app/build/outputs/apk/debug/app-debug.apk`. SHA-256: `C97160F2473718C7D45084BE553E970D2B8D061E28BF2B80EC5C826072096372`.

## Earlier phone access blocker

The existing Tasker activation endpoint answered over Tailscale with HTTP 200 and its expected response. No authenticated ADB transport became available: the previous TCP port was unavailable, local mDNS found no endpoint, and bounded remote port discovery found none. Tasker's acknowledgement does not prove debugging started.

At that stage, APK installation, phone UI review, connection/capture recovery tests, Hey Google enablement and automatic/locked handoff tests remained pending. The phone's assistant and wake settings could not be inspected or changed. Local evidence is under `.dev/voice-start-20261010/` and the feature worktree's `.dev/`; unrelated reports were preserved.

Later on 2026-10-10, authenticated ADB access returned and the phone follow-through proceeded. See [current phone activation verification](phone-activation-final.md) for the installed fixes, unlocked/locked assistant handoff results, and remaining Voice Match enrollment.
