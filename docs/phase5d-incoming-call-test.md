# Phase 5D — Controlled Incoming-Call Test

## Status: BLOCKED — Pre-test requirements not met

## Pre-test verification

| Requirement | Result | Evidence |
|---|---|---|
| iPhone Connected=true | ✅ | `bluetoothctl info` shows `Connected: yes` |
| ServicesResolved=true | ✅ | D-Bus Properties: `ServicesResolved b true` |
| HFP RFCOMM channel 8 connected | ❌ | No NewConnection in journal, no Profile1 registered |
| HFP service-level connection complete | ❌ | No AT commands observed after reconnect |
| headset-head-unit present or available | ❌ | `bluez5.profile = "off"`, no HFP sink/source in PipeWire |
| call=0 | N/A | RFCOMM not active |
| callsetup=0 | N/A | RFCOMM not active |
| callheld=0 | N/A | RFCOMM not active |

## Procedure attempted

1. Initial state: `ServicesResolved=false`, `bluez5.profile="off"` — HFP absent
2. Disconnect via `bluetoothctl disconnect <REDACTED_BLUETOOTH_ADDRESS>`
3. Reconnect via `bluetoothctl connect <REDACTED_BLUETOOTH_ADDRESS>`
4. Wait 30+ seconds: device appears in PipeWire (`wpctl status`: device 41 `illuminary-cinema` [bluez5]) but `bluez5.profile = "off"`
5. `ServicesResolved = true`
6. A2DP endpoints registered (`Endpoint sep1-sep6 NEW`), but A2DP `SET_CONFIGURATION rejected: Configuration not supported (41)`
7. **No NewConnection in journal** — BlueZ never sent RFCOMM channel 8 connection to WirePlumber
8. **D-Bus method_return rejection persists**: `Rejected send message, 0 matched rules; type="method_return", sender=":1.798" (WirePlumber) ... destination=":1.634" (bluetoothd)`
9. Second disconnect/reconnect cycle: same result
10. Attempted `ConnectProfile("0000111f-...")` — fails because iPhone is Audio Gateway, not HF

## Blockers

1. **HFP RFCOMM absent** — No RFCOMM channel 8 connection observed after disconnect/reconnect. Whether this is due to missing Profile1 registration, ConnectProfile not called, or BlueZ state is unresolved.
2. **A2DP SET_CONFIGURATION rejected** — `Configuration not supported (41)` — same as Phase 4

## Classification

`HFP_CURRENT_REGISTRATION_AND_CONNECT_STATE_UNRESOLVED` — Whether the current WirePlumber process registered /Profile/HFPHF is unknown. Whether ConnectProfile(111f) triggers RFCOMM is untested.

## Recommended next action

Phase A: Prove current HFP registration via WirePlumber restart with D-Bus capture. Then Phase B–E.
