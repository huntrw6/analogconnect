# Phase 5A — Profile Lifecycle from Phase 4 Traces

> Historical investigation record. Its open questions were resolved or superseded
> by Phases G, H, and I. See `docs/current-state.md`.

## Evidence labels
- `VERIFIED_AUTOMATED` — btmon btsnoop capture, PipeWire journal, D-Bus trace
- `INFERRED` — timeline reconstructed from multiple independent captures

## Timeline

### Phase 4 Restart Test (WirePlumber restart)

| Time (journal) | Event | Evidence | Present/Absent |
|---|---|---|---|
| T+0 | WirePlumber restarted | `wpctl status` after restart | PRESENT |
| T+~30s | Device appears in PipeWire | `wpctl status`: device 75 `illuminary-cinema` [bluez5] | PRESENT |
| T+~30s | `bluez5.profile = "off"` | PipeWire dump after restart | PRESENT |
| T+~30s | `ServicesResolved = false` | bluetoothctl info: ServicesResolved: no | PRESENT |
| T+~30s | `headset-head-unit` EnumProfile | PipeWire source code gate: `connected_profiles & SPA_BT_PROFILE_HEADSET_HEAD_UNIT` | INFERRED |
| T+~30s | SCO transport created | Journal: `spa.bluez5.sink.sco` node attempted write | PRESENT |
| T+~30s | SCO write failed: `-104 (Connection reset by peer)` | Journal at 22:27:13 | PRESENT |
| T+~30s | Profile reverts to "off" | SCO failure → profile deactivation | INFERRED |
| T+~30s | `headset-head-unit` disappears | EnumProfile no longer in PipeWire | INFERRED |

### Phase 4b Reconnect Test (manual disconnect/reconnect via bluetoothctl)

| Time (btmon) | Frame | Event | Direction | Evidence | Present/Absent |
|---|---|---|---|---|---|
| 2.351640 | #1 | HCI Create Connection | TX | btmon | PRESENT |
| 3.077575 | — | MGMT Device Connected | RX | btmon | PRESENT |
| 3.906607 | #59 | RFCOMM SABM (dlci 0) | TX | btmon | PRESENT |
| 4.071403 | #61 | RFCOMM UA (dlci 0) | RX | btmon | PRESENT |
| 4.111510 | #64 | RFCOMM SABM (dlci 0x10) | TX | btmon | PRESENT |
| 4.216390 | #66 | RFCOMM UA (dlci 0x10) | RX | btmon | PRESENT |
| 4.233954 | #76 | AT+BRSF=695 | TX | btmon | PRESENT |
| 4.241458 | #80 | +BRSF:4079 | RX | btmon | PRESENT |
| 4.279689 | #83 | AT+BAC=1,2,3 | TX | btmon | PRESENT |
| 4.293367 | #90 | AT+CIND=? | TX | btmon | PRESENT |
| 4.298957 | #92 | +CIND: ("service",(0-1)),("call",(0-1)),("callsetup",(0-3)),("battchg",(0-5)),("signal",(0-5)),("roam",(0-1)),("callheld",(0-2)) | RX | btmon | PRESENT |
| 4.386596 | #96 | AT+CIND? | TX | btmon | PRESENT |
| 4.397756 | #99 | +CIND: 1,0,0,5,3,0,0 | RX | btmon | PRESENT |
| 4.435608 | #102 | AT+CMER=3,0,0,1 | TX | btmon | PRESENT |
| 4.455122 | #107 | AT+CHLD=? | TX | btmon | PRESENT |
| 4.474330 | #113 | AT+CLIP=1 | TX | btmon | PRESENT |
| 4.549098 | #118 | AT+CCWA=1 | TX | btmon | PRESENT |
| 4.683338 | #126 | AT+CMEE=1 | TX | btmon | PRESENT |
| 4.786696 | #133 | AT+CLCC | TX | btmon | PRESENT |
| 4.829094 | #142 | OK (to AT+CLCC) — no +CLCC list = no active calls | RX | btmon | PRESENT |
| 4.829–4.889 | #134–#161 | SDP queries, AVRCP connection, L2CAP PSM 27 | — | btmon | PRESENT |
| 7.044464 | #221 | L2CAP Disconnect (SDP channel) | TX | btmon | PRESENT |
| — | — | **No SCO connection attempt in this trace** | — | btmon | ABSENT |

## Call-state indicators during Phase 4b reconnect

Decoded from `+CIND: 1,0,0,5,3,0,0`:

| Indicator | Value | Meaning |
|---|---|---|
| service | 1 | Service available |
| call | 0 | No active call |
| callsetup | 0 | No call setup in progress |
| battchg | 5 | Battery full |
| signal | 3 | Medium signal strength |
| roam | 0 | Not roaming |
| callheld | 0 | No call held |

**Key finding**: No active call, no pending call, no held call during the reconnect. AT+CLCC returned OK without +CLCC list (no active call list).

## SCO failure analysis

### Phase 4 restart: SCO created but reset
- **SCO transport created**: Journal confirms `spa.bluez5.sink.sco` attempted write
- **SCO write failed**: `-104 (Connection reset by peer)` — iPhone closed SCO link
- **No confirmed active call**: CIND shows call=0, callsetup=0, callheld=0
- **Possible cause**: Audio Gateway may reject SCO when no call is active (normal HFP behavior)
- **Classification**: `HFP_PROFILE_AND_SCO_CREATED_REMOTE_CLOSED_SCO`

### Phase 4b reconnect: No SCO attempt
- After manual disconnect/reconnect, HFP SLC completed but no SCO was attempted
- Profile remained "off" in PipeWire
- `headset-head-unit` did not reappear

## Root cause chain

```
WirePlumber restart
  → BlueZ delivers NewConnection to /Profile/HFPHF
  → RFCOMM SABM/UA succeeds, AT negotiation completes (SLC established)
  → PipeWire creates SCO sink/source nodes
  → SCO transport created, SCO write attempted
  → iPhone resets SCO link (no active call → normal AG behavior?)
  → SCO failure → profile reverts to "off"
  → headset-head-unit EnumProfile disappears
  → Manual disconnect/reconnect
  → RFCOMM + AT succeeds again
  → But profile stays "off", headset-head-unit does not reappear
```

## Open questions

1. Does the iPhone reject SCO because no call is active? (normal HFP AG behavior)
2. Why doesn't headset-head-unit reappear after reconnect?
3. Is the profile state stuck after SCO failure?
4. Would SCO succeed during an active call?
