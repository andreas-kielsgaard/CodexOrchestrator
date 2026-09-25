# Phone operation capabilities

## Remote shell and configuration

- `adb shell` exposes Android shell commands on either verified phone.
- `settings get`, `settings list`, and `settings put` expose global, secure, and system settings available to the shell user.
- `dumpsys` exposes activity, window, input, notification, package, Wi-Fi, power, and service state.
- `pm` and `cmd package` expose installed-package inventory and supported per-user package operations.
- `am start` launches activities, Settings pages, and deep links.
- `input` exposes key events, text, taps, and swipes.
- `uiautomator dump` exposes the accessible UI hierarchy and bounds.

Some One UI switches update only through their visible Settings control. UI hierarchy bounds plus `input tap` expose that control path.

## Screen and input

- `screencap -p` captures ordinary Android windows.
- Phone Toolbox renders screenshots, maps viewport coordinates to Android coordinates, and supports taps, direct drags, and two-click point-to-point swipes.
- `dumpsys input` exposes the active `Viewport INTERNAL logicalFrame` used for coordinate mapping after rotation; `wm size` exposes the physical display size.
- Windows marked `secure=true` block screenshot pixels. The semantic accessibility overlay can still expose labels, bounds, and supported actions published by the app.
- The DPAPI-backed automation can unlock the phone and enter the stored PIN into a focused numerical interface.

## Files and media

`adb pull`, `adb push`, and remote shell file commands expose copying and file management. `stat`, file counts, and hashes expose transfer verification.

Android MediaStore exposes photo metadata. On the S21 the camera-image column is `datetaken`; useful fields include:

```text
_display_name:_data:_size:datetaken:mime_type:relative_path
```

`relative_path=DCIM/Camera/` identifies camera images separately from screenshots, downloads, and app media. `content query --where` accepts Unix-millisecond time windows. `[Environment]::GetFolderPath('MyPictures')` resolves the Windows photo destination.

`adb pull` copies files while leaving the phone originals in place. Local and remote counts, `_size`, `stat`, and hashes can confirm transfer completeness.

## Inventory and migration

- `pm list packages`, package metadata, Android settings, `dumpsys`, screenshots, and UI hierarchy expose installed apps and user-visible configuration on both phones.
- Nothing launcher pages, dock, folders, widgets, and drawer state can be observed through screenshots and UI automation, then reproduced through S21 launcher input.
- Launcher databases remain inside each launcher's app-private sandbox under standard non-root ADB.
- `dumpsys notification --noredact` exposes application notification enablement, channels, channel groups, badges, importance, and user-locked fields for comparison between phones.
- Samsung One UI 7 exposes per-app notification categories after `settings put secure show_notification_category_setting 1`; the corresponding Settings control is **Manage notification categories for each app**.
- `phone-migration-captures` contains prior structured notification and migration inventories.
- Package enable/disable state, default applications, Samsung services, notification choices, and supported launcher organization can be configured through shell commands, Settings activities, and UI input.

## Technical limits exposed by the devices

- Standard ADB cannot read another app's private database without that app exposing an interface or the device providing elevated access.
- Android `FLAG_SECURE` prevents screenshot and screen-mirroring pixels for protected windows.
- Accessibility exposes only the labels, bounds, text, and actions published by the active app.
- Authentication flows can expose text fields, approval screens, deep links, notifications, SMS delivery, Bluetooth state, and accessibility controls; the exact exposed surface varies by app.
