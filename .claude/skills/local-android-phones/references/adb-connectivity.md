# ADB connection capabilities

## Transport evidence

| Observation | Available capability |
|---|---|---|
| `adb devices -l` shows a ready hardware serial | USB targeting through `adb -s <serial>` is available. |
| A numeric `ip:port` and an `adb-..._adb-tls-connect...` entry return the same hardware serial | Both entries expose the same phone through different wireless transport names. |
| `more than one device/emulator` | `adb -s <transport>` or a phone-specific helper provides device selection. |
| `unauthorized` | Android can display the ADB-key approval prompt after the phone is unlocked. |
| Windows shows a Samsung ADB interface while ADB omits the S21 | `adb kill-server` and `adb start-server` refresh local ADB discovery and authorization state. |
| A remembered numeric endpoint is refused or offline | `adb mdns services`, the reconnect helper, or USB can expose the current transport. |
| `_adb-tls-pairing._tcp` appears | `adb pair <endpoint>` can establish the PC's authenticated pairing while the pairing service is active. |
| `_adb-tls-connect._tcp` appears | `adb connect <endpoint>` can establish the recurring wireless transport. |
| mDNS is empty while a ready wireless transport exists | The ready transport remains available through its current ADB device entry. |
| A candidate's `ro.serialno` differs from the intended phone | Other candidates can be enumerated and tested by hardware identity. |
| A phone's Tasker listener answers while Wireless debugging is off | Its on-demand launcher can enable the setting before mDNS and serial verification. |

## Useful probes

The bundled ADB exposes these probes:

```powershell
& '.\Phone Hotspot Automation\platform-tools\adb.exe' devices -l
& '.\Phone Hotspot Automation\platform-tools\adb.exe' mdns services
& '.\Phone Hotspot Automation\platform-tools\adb.exe' -s <transport> shell getprop ro.serialno
& '.\Phone Hotspot Automation\platform-tools\adb.exe' -s <transport> shell getprop ro.product.model
```

The S21 helper can rediscover a connection after `adb disconnect <endpoint>` or an ADB-server restart. The Nothing toolbox exposes `/api/status` and screenshot endpoints for live transport checks.

`Enable-Nothing-WirelessDebugging.cmd` and `Enable-S21-WirelessDebugging.cmd` reach separate Tasker HTTP listeners over the home Wi-Fi. The listeners do not depend on an active ADB transport; each launcher enables Wireless debugging, discovers the new dynamic endpoint, and verifies the phone's hardware serial.

Wireless debugging operates on the local network. Android advertises dynamic pairing and connection ports through mDNS; a DHCP reservation can stabilize the IP address but not the debugging port.

The **Pair device with pairing code** screen exposes a temporary `_adb-tls-pairing._tcp` endpoint and code. After pairing, `_adb-tls-connect._tcp` exposes the main connection endpoint. The S21 helper implements both paths.
