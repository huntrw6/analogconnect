# Phase G: Current-Owner Connection Test

> Historical investigation record. The classification below was superseded by
> Phase H, which captured the complete SLC, and Phase I, which captured active-call
> codec and eSCO behavior. See `docs/current-state.md`.

**Date**: 2026-08-01 13:44–13:52 PDT
**Classification**: `HFP_CURRENT_OWNER_ACCEPTS_BUT_PIPELINE_INACTIVE`
**Previous classification (refuted)**: `HFP_RFCOMM_PRECEDES_CURRENT_WIREPLUMBER_REGISTRATION`

## Summary

The current WirePlumber process (PID 298021, `:1.885`) **receives and accepts** a fresh NewConnection for HFP, but the PipeWire HFP pipeline does not activate. No `headset-head-unit` EnumProfile appears, no SCO transport is created, and no SCO sink/source nodes are emitted.

This refutes the earlier hypothesis that RFCOMM timing was the cause. The issue is deeper in the PipeWire/WirePlumber/BlueZ HFP pipeline.

## Test procedure

### Phase G4: Remove pre-existing HFP RFCOMM

1. Captured before-state:
   - EnumProfile: `off` (0), `audio-gateway` (65536) — no `headset-head-unit`
   - Active Profile: `audio-gateway` (65536)
   - RFCOMM: active session to iPhone on channel 8, DLCI 16, MTU 1015

2. Called `DisconnectProfile("0000111f-0000-1000-8000-00805f9b34fb")` — returned success (38ms)

3. Verified RFCOMM removal:
   - `/sys/kernel/debug/bluetooth/rfcomm`: iPhone session GONE (only local listeners remain)
   - `/sys/kernel/debug/bluetooth/rfcomm_dlc`: empty
   - L2CAP PSM 3 (RFCOMM) to iPhone: GONE
   - A2DP (PSM 25) and AVRCP (PSM 23) to iPhone: still active
   - `Device1.Connected`: still `true`

4. Post-disconnect PipeWire state: same as before (no `headset-head-unit`)

### Phase G6: Create fresh HFP connection

1. Called `ConnectProfile("0000111f-0000-1000-8000-00805f9b34fb")` — returned success (399ms)

2. Verified RFCOMM re-establishment (8s later):
   - RFCOMM: active session to iPhone on channel 8, DLCI 16, MTU 1015
   - `rfcomm_dlc`: local and remote addresses redacted; DLCI 16, MTU 1015

3. D-Bus monitor captured:
   - `NewConnection` delivered to `Destination=:1.885` (current WirePlumber, PID 298021)
   - Path: `/Profile/HFPHF`
   - UUID implicit: HFP (via `path_to_profile`)
   - Version: 264 (HFP 1.8), Features: 47
   - WirePlumber accepted: method_return success (3ms response time)
   - `BatteryProvider1.InterfacesAdded` signal appeared
   - `Device1.Connected = true`, `ServicesResolved = true`

### Phase G7: PipeWire state inspection

- EnumProfile: `off` (0), `audio-gateway` (65536) — **`headset-head-unit` STILL ABSENT**
- Active Profile: `audio-gateway` (65536)
- EnumRoute: empty. Route: empty.
- HFP transport objects: **NONE**
- SCO sink/source nodes: **NONE**
- btmon post-test: **NO HCI traffic** after RFCOMM establishment

## Source code analysis

### Profile mapping (`backend-native.c:3262`)

```
PROFILE_HFP_HF = "/Profile/HFPHF" (UUID 0x111e)
PROFILE_HFP_AG = "/Profile/HFPAG" (UUID 0x111f)

path_to_profile("/Profile/HFPHF") → SPA_BT_PROFILE_HFP_AG
path_to_profile("/Profile/HFPAG") → SPA_BT_PROFILE_HFP_HF
```

When NewConnection arrives on `/Profile/HFPHF`, PipeWire maps it to `SPA_BT_PROFILE_HFP_AG`. This means the Pi is acting as HF (hands-free) connecting to the iPhone's AG (audio gateway).

### EnumProfile gate (`bluez5-device.c:1977`)

```c
uint32_t profile = device->connected_profiles &
      SPA_BT_PROFILE_HEADSET_HEAD_UNIT;
if (profile == 0)
    return NULL;
```

`SPA_BT_PROFILE_HEADSET_HEAD_UNIT = SPA_BT_PROFILE_HSP_HS | SPA_BT_PROFILE_HFP_HF`

`SPA_BT_PROFILE_HFP_AG` (0x100) is NOT part of `SPA_BT_PROFILE_HEADSET_HEAD_UNIT` (0xa0). So `headset-head-unit` EnumProfile requires `SPA_BT_PROFILE_HFP_HF`, which is only set when a remote device connects to our AG profile.

### SCO node creation (`bluez5-device.c:1092`)

```c
case DEVICE_PROFILE_AG:
    if (this->bt_dev->connected_profiles & SPA_BT_PROFILE_HEADSET_AUDIO_GATEWAY) {
        t = find_transport(this, SPA_BT_PROFILE_HFP_AG);
        if (!t)
            t = find_transport(this, SPA_BT_PROFILE_HSP_AG);
        if (t) {
            emit_dynamic_node(this, t, 0, SPA_NAME_API_BLUEZ5_SCO_SOURCE, false);
            emit_dynamic_node(this, t, 1, SPA_NAME_API_BLUEZ5_SCO_SINK, false);
        }
    }
```

SCO nodes require BOTH `SPA_BT_PROFILE_HEADSET_AUDIO_GATEWAY` in `connected_profiles` AND a transport object. The transport is created by `rfcomm_new_transport()` when the AT SLC negotiation completes.

### AT SLC flow (`backend-native.c:3363`)

When `profile == SPA_BT_PROFILE_HFP_AG`:
1. Sends `AT+BRSF=<hf_features>` to remote AG
2. Awaits `+BRSF:<ag_features>` response
3. Sends `AT+BAC=1,2` (available codecs)
4. Awaits `OK`
5. Sends `AT+CIND=?` (indicator mapping)
6. Awaits `+CIND:...`
7. Sends `AT+CIND?` (indicator values)
8. Awaits `+CIND:...`
9. Sends `AT+CMER=3,0,0,1` (enable indicator reporting)
10. Awaits `OK`
11. Sends `AT+CHLD=?` (call hold)
12. Awaits `+CHLD:...`
13. SLC complete → `rfcomm_new_transport()` → `spa_bt_device_connect_profile(device, SPA_BT_PROFILE_HFP_AG)`

## Key findings

1. **NewConnection delivery to current WirePlumber**: `VERIFIED_AUTOMATED`
   - `Sender=:1.880 (bluetoothd) → Destination=:1.885 (WirePlumber PID 298021)`
   - Path: `/Profile/HFPHF`
   - WirePlumber accepted (method_return success, 3ms)

2. **RFCOMM re-establishment**: `VERIFIED_AUTOMATED`
   - SABM/UA exchange completed (RFCOMM DLCI 16, MTU 1015)

3. **AT SLC negotiation**: `UNKNOWN`
   - btmon started 3 minutes after NewConnection — missed the AT exchange
   - Post-test btmon shows NO HCI traffic — RFCOMM is silent
   - No HFP transport created → AT SLC likely did not complete

4. **PipeWire EnumProfile unchanged**: `VERIFIED_AUTOMATED`
   - `headset-head-unit` absent before and after
   - Only `off` and `audio-gateway` present

5. **Profile mapping insight**: `DOCUMENTED`
   - Pi acting as HF → `SPA_BT_PROFILE_HFP_AG` → `SPA_BT_PROFILE_HEADSET_AUDIO_GATEWAY`
   - `headset-head-unit` requires `SPA_BT_PROFILE_HFP_HF` (Pi acting as AG)
   - The `audio-gateway` composite profile IS the correct profile for Pi-as-HF

## Remaining unknowns

1. **Why did AT SLC not complete?** The RFCOMM is established at kernel level, WirePlumber accepted the FD, but no AT exchange is visible. Possible causes:
   - WirePlumber accepted the FD but didn't start the event loop for it
   - The AT+BRSF was sent but the iPhone didn't respond
   - WirePlumber's backend-native didn't call `rfcomm_event` for the new FD
   - The RFCOMM FD was not properly registered with the event loop

2. **Would `headset-head-unit` ever appear?** Only if the iPhone initiates a connection to our AG profile (UUID 0x111f → `/Profile/HFPAG` → `SPA_BT_PROFILE_HFP_HF`). But in the normal HFP model, the HF always initiates. So `headset-head-unit` may never appear in this configuration.

3. **Is the `audio-gateway` profile sufficient?** The `audio-gateway` composite profile includes A2DP Source + HFP AG. It should emit SCO nodes when the HFP transport is created. But without the transport, no SCO nodes appear.

## Evidence labels used

- `VERIFIED_AUTOMATED` — demonstrated by D-Bus monitor, rfcomm debugfs, pw-cli
- `DOCUMENTED` — supported by PipeWire 1.4.2 source code analysis
- `UNKNOWN` — insufficient evidence (AT SLC completion, WirePlumber event loop state)
- `FAILED` — attempted and did not produce expected result (headset-head-unit EnumProfile)
