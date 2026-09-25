# Local capability inventory

## Workspace

- Phone automation root: `C:\Users\user\Documents\Random Codex Stuff\Phone Hotspot Automation`
- Bundled ADB: `Phone Hotspot Automation\platform-tools\adb.exe`
- Migration evidence: `C:\Users\user\Documents\Random Codex Stuff\phone-migration-captures`
- Windows photo library: resolve `[Environment]::GetFolderPath('MyPictures')`; it currently points to `C:\Users\user\Pictures`.

The helpers call the bundled ADB. Commands using that binary share its server version and ADB key environment.

## Phone identities

| Phone | Stable identity | Known software |
|---|---|---|
| Nothing A142 / Pacman | Serial `00064141K002008` | Android 14; launcher `com.nothing.launcher/com.android.searchlauncher.SearchLauncher` |
| Samsung Galaxy S21 5G | Model `SM-G991B` / device `o1s`; serial `R3CRA0BAYXD` | Android 15 / One UI 7; launcher `com.sec.android.app.launcher/.activities.LauncherActivity` |

Hardware serial is stronger evidence than a USB transport label, mDNS instance name, IP address, or port.

## Nothing Phone Toolbox capabilities

- Launcher: `Phone Hotspot Automation\Run-Phone-Toolbox.cmd`
- Local UI: `http://127.0.0.1:8800`; `PHONE_TOOLBOX_PORT` can override the port.
- Server: `Phone Hotspot Automation\Phone Toolbox\server.js`
- Hotspot/unlock automation: `Enable-PhoneHotspot.ps1`; `-StatusOnly` exposes a connection and hotspot-state check.
- Setup stores the device PIN through Windows DPAPI under `%LOCALAPPDATA%\PhoneHotspotAutomation`.
- `%LOCALAPPDATA%\PhoneHotspotAutomation\config.json` selects the configured phone and currently contains the Nothing serial.

The toolbox can:

- Resolve ready USB and wireless transports by hardware serial.
- Discover `_adb-tls-connect._tcp` services and connect to the matching phone.
- Display live screenshots and a semantic accessibility overlay.
- Translate viewport clicks and drags into phone input.
- Send taps, two-point swipes, text, Enter, clear, Home, Back, multitasking, notifications, wake, and sleep actions.
- Apply the DPAPI-protected PIN to unlock the device or enter digits into the focused UI.
- Inspect status and control the Wi-Fi hotspot.

## Samsung S21 connection capabilities

- Reconnect helper: `Phone Hotspot Automation\Connect-S21Wireless.ps1`
- Double-click launcher: `Phone Hotspot Automation\Connect-S21Wireless.cmd`
- The helper discovers `_adb-tls-connect._tcp` endpoints, connects candidates, and verifies serial `R3CRA0BAYXD`.
- `-PairEndpoint <ip:pairing-port>` invokes ADB pairing and prompts for the short-lived pairing code.
- `-Endpoint <ip:port>` exposes direct connection to a supplied main debugging endpoint.
- `-TimeoutSeconds <seconds>` controls the discovery window.

Once connected, the bundled ADB exposes the same shell, package, activity, settings, input, screenshot, and file functions available on the Nothing phone.

## On-demand Wireless debugging

- Nothing launcher: `Phone Hotspot Automation\Enable-Nothing-WirelessDebugging.cmd`; Tasker listens on port 1821.
- S21 launcher: `Phone Hotspot Automation\Enable-S21-WirelessDebugging.cmd`; Tasker listens on port 1822 and delegates transport verification to `Connect-S21Wireless.ps1`.
- Both phones use a Tasker profile named `WD Manual Push` on the home SSID and a task named `Ensure Wireless Debugging` that writes global setting `adb_wifi_enabled=1`.
- Each Windows helper sends a protected local request, validates Tasker's response, caches the phone's Wi-Fi address, and scans the local `/24` if that address changes.
- After Tasker enables the setting, the helpers discover Android's current Wireless-debugging port and accept only the configured hardware serial.
