---
name: local-android-phones
description: Provides the local tools and known capabilities for connecting to, inspecting, controlling, configuring, and transferring data with the user's Nothing A142 and Samsung Galaxy S21 from this Windows PC. Use for Android ADB, Phone Toolbox, hotspot, screenshots, UI automation, settings, migration, and media tasks.
---

# Local Android Phone Capabilities

The phones are available through USB ADB, authenticated Wireless debugging, and the local Phone Toolbox. Hardware identity connects changing transports to a stable device.

## Capability references

- [references/local-setup.md](references/local-setup.md): phone identities, workspace paths, helper programs, and their exposed functions.
- [references/adb-connectivity.md](references/adb-connectivity.md): USB and wireless transport discovery, pairing, reconnection, selection, and status clues.
- [references/phone-work.md](references/phone-work.md): remote shell, UI control, screenshots, settings, files, media, and migration capabilities.

## Shared capability model

- `adb devices -l`, `adb mdns services`, `getprop ro.serialno`, and `getprop ro.product.model` resolve a usable transport to a phone.
- `adb -s <transport>` exposes shell, package, settings, activity, input, screenshot, and file-transfer functions.
- The Phone Toolbox combines transport resolution with screen viewing, semantic UI inspection, tap, drag, key, text, unlock, and hotspot functions.
- The S21 reconnect helper combines mDNS discovery, ADB connection, and hardware-serial verification.
- Per-phone Tasker listeners and Windows launchers can enable Wireless debugging on demand from the home Wi-Fi before transport discovery.
- The stored DPAPI-protected PIN can be applied by the existing automation for device unlocking and focused application input without exposing the plaintext value.
