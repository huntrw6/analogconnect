# Phase F — PipeWire HFP Profile State Inspection

## Evidence labels
- `VERIFIED_AUTOMATED` — pw-cli, busctl, debugfs, btmon
- `CURRENT_NEWCONNECTION_NOT_CAPTURED` — D-Bus monitor started after RFCOMM establishment

## Phase F1 — PipeWire Device Object (no restart)

### Device identification
- **Current Device ID**: 41
- **Device name**: `bluez_card.20_1A_94_70_26_7C`
- **Device API**: `bluez5`
- **Device description**: `illuminary-cinema`
- **`api.bluez5.connection`**: `connected`
- **`api.bluez5.path`**: `/org/bluez/hci0/dev_20_1A_94_70_26_7C`

### EnumProfile entries
| Index | Name | Description | Available | Priority |
|---|---|---|---|---|
| 0 | `off` | Off | yes | 0 |
| 65536 | `audio-gateway` | Audio Gateway (A2DP Source & HSP/HFP AG) | yes | 256 |

**`headset-head-unit` EnumProfile**: **ABSENT**

### Active Profile parameter
- **Name**: `audio-gateway`
- **Index**: 65536
- **Description**: Audio Gateway (A2DP Source & HSP/HFP AG)
- **Available**: yes
- **Priority**: 256
- **Save**: false

### Device properties (corrected)
- `bluez5.profile = "off"` — **NOT authoritative** for active profile state. The authoritative state is the active Profile parameter queried via `pw-cli enum-params`.
- `bluez5.auto-connect = [ hfp_hf hsp_hs a2dp_sink hfp_ag hsp_ag a2dp_source ]`
- `bluetoothAudioCodec = sbc`
- `bluetoothOffloadActive = false`

### Other
- **EnumRoute**: empty
- **Route**: empty
- **HFP transport objects**: NONE
- **SCO sink/source nodes**: NONE

### Full pw-dump saved
`test-results/phaseF-current-pw-dump.json` (3391 lines)

## Phase F2 — Current HFP Control Connection

### RFCOMM status
- **`/proc/net/bluetooth/rfcomm`**: Does not exist (rfcomm module not in procfs)
- **`/sys/kernel/debug/bluetooth/rfcomm`**: Active session to `<REDACTED_BLUETOOTH_ADDRESS>` on channel 8
- **`/sys/kernel/debug/bluetooth/rfcomm_dlc`**: DLCI 16 (0x10), MTU 1015
- **`/sys/kernel/debug/bluetooth/sco`**: No SCO connections
- **L2CAP PSM 3 (RFCOMM)**: Active — CID 0x0041/0x0909, MTU 1021/2582

### BlueZ device state
- **Connected**: true (D-Bus `org.bluez.Device1.Connected`)
- **ServicesResolved**: true (D-Bus `org.bluez.Device1.ServicesResolved`)
- **UUIDs**: 22 UUIDs including `0000111f` (HFP Audio Gateway)

### btmon evidence
- Phase E btmon shows RFCOMM SABM TX at ~1.62s, UA received
- Full AT negotiation completed: BRSF→BAC→CIND→CMER→CHLD→CLIP→CCWA→CMEE→CLCC — all OK
- No RFCOMM disconnect frames in the entire btmon capture
- L2CAP disconnections at ~3s were for CIDs 1547, 1290, 1032 (NOT RFCOMM channel 2313/65)

### HFP SLC alive: YES

## Phase F3 — NewConnection Correlation

### Phase E D-Bus monitor
- **Monitor started**: 13:02:52 (time=1785614572.324368)
- **RegisterProfile captured**: 13:02:53 — `/Profile/HFPHF` (UUID 0x111e) and `/Profile/HFPAG` (UUID 0x111f) from sender `:1.885` (WirePlumber PID 298021)
- **NewConnection captured**: **NO** — zero NewConnection, ProfileChanged, or profile_connect method calls in the 3009-line log

### Timeline correlation
- **RFCOMM SABM**: btmon time ~1.62s (corresponds to ~13:02:15.6, based on btmon start at ~13:02:14)
- **D-Bus RegisterProfile**: 13:02:53 (37 seconds AFTER RFCOMM establishment)
- **Conclusion**: The D-Bus monitor was started 37 seconds after the RFCOMM was established. The NewConnection callback occurred before the monitor was running.

### Inference
The RFCOMM SABM is TX (Pi-initiated) and the full AT HF command sequence was sent by the local Pi. This requires an HFP handler to have accepted the RFCOMM file descriptor. In BlueZ's Profile1 model, this requires `NewConnection` to have been delivered to a registered `/Profile/HFPHF` object. The AT negotiation completing proves the callback happened, but we cannot confirm from the Phase E traces which process received it.

**`CURRENT_NEWCONNECTION_NOT_CAPTURED`**

## Phase F4 — Classification

### Classification
**`HFP_SLC_NOT_REFLECTED_IN_CONNECTED_PROFILES`**

### Rationale
1. The HFP SLC is established at the HCI level (RFCOMM alive, AT negotiation complete, no disconnect)
2. `headset-head-unit` is absent from PipeWire EnumProfile
3. The active Profile is `audio-gateway` (Pi-as-AG), not `headset-head-unit` (Pi-as-HF)
4. No HFP transport objects exist
5. No SCO sink/source nodes exist

### Corrected previous conclusion
**WITHDRAWN**: "PipeWire/WirePlumber never transitions the EnumProfile to connected because bluez5.profile remains off."

**REPLACEMENT**: `bluez5.profile` is a device property describing an initial profile preference. It is not the authoritative active-profile state. The authoritative parameters are `EnumProfile` and `Profile` queried directly from the PipeWire Bluetooth Device object. After the Phase E SLC, the active Profile is `audio-gateway` (index 65536), and `headset-head-unit` is absent from EnumProfile.

## Open questions
1. Why is `headset-head-unit` absent from EnumProfile despite the SLC being established?
2. Was the NewConnection callback delivered to the current WirePlumber process (PID 298021)?
3. Does the `connected_profiles` bitmask include `SPA_BT_PROFILE_HEADSET_HEAD_UNIT`?
4. Would an incoming call cause profile activation and EnumProfile update?
