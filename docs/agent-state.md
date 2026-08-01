# AnalogConnect Agent State

## Current milestone
Milestone 0

## Current phase
Phase 5D — Incoming-Call Test BLOCKED

## Current objective
Run controlled incoming-call test to verify HFP SCO audio. Pre-test requirements not met.

## Current classification
`BLOCKED` — HFP RFCOMM cannot be established after disconnect/reconnect

## Last completed action
Phase 5D incoming-call test BLOCKED at pre-test verification:
1. After bluetoothctl disconnect/reconnect, device appears in PipeWire (`bluez5.profile = "off"`)
2. `ServicesResolved = true`
3. **No NewConnection in journal** — BlueZ never sends RFCOMM channel 8 connection to WirePlumber
4. **No Profile1 registered for HFP** — WirePlumber's native backend fails to register
5. **D-Bus method_return rejection persists**: WirePlumber → bluetoothd
6. A2DP `SET_CONFIGURATION rejected: Configuration not supported (41)`
7. Two disconnect/reconnect cycles attempted, same result both times
8. Phase 5A lifecycle documented, Phase 5B call-state indicators decoded, Phase 5C confirmed `bluez5.disable-dummy-call` does not exist in PipeWire 1.4.2

## Evidence

### Established

- `VERIFIED_HARDWARE`: MAP works (with reconnection)
- `VERIFIED_HARDWARE`: PBAP works (with reconnection)
- `VERIFIED_AUTOMATED`: The iPhone advertises HFP Audio Gateway UUID `111f`
- `VERIFIED_AUTOMATED`: `ServicesResolved` was true after manual disconnect/reconnect
- `VERIFIED_AUTOMATED`: A previous WirePlumber instance registered `/Profile/HFPHF`
- `VERIFIED_AUTOMATED`: BlueZ delivered NewConnection to `/Profile/HFPHF` during the earlier test
- `VERIFIED_AUTOMATED`: The earlier WirePlumber instance received the RFCOMM file descriptor
- `VERIFIED_AUTOMATED`: The Pi sent HFP Hands-Free AT commands
- `VERIFIED_AUTOMATED`: The iPhone responded as HFP Audio Gateway
- `VERIFIED_AUTOMATED`: HFP service-level negotiation completed in the earlier test
- `VERIFIED_AUTOMATED`: `headset-head-unit` was absent from PipeWire EnumProfile during the earlier test
- `VERIFIED_AUTOMATED`: Phase 7d btmon shows RFCOMM SABM sent by LOCAL Pi (TX), UA received — Pi initiated RFCOMM
- `VERIFIED_AUTOMATED`: Phase 7d btmon shows full AT negotiation: AT+BRSF, AT+BAC, AT+CIND, AT+CMER, AT+CHLD, AT+CLIP, AT+CCWA, AT+CMEE, AT+CLCC — all with OK responses
- `VERIFIED_AUTOMATED`: Phase 7d btmon shows A2DP (AVDTP) and AVRCP (AVCTP) also established alongside HFP RFCOMM
- `VERIFIED_AUTOMATED`: D-Bus journal shows `Rejected send message, 0 matched rules; type="method_return"` from WirePlumber to bluetoothd at 19:00:49 — may indicate D-Bus policy issue preventing Profile1 reply delivery
- `VERIFIED_AUTOMATED`: Phase 4 restart: `spa.bluez5.sink.sco: failed to write data: -104 (Connection reset by peer)` — headset-head-unit DID appear and SCO transport was created, but SCO link was reset by iPhone
- `VERIFIED_AUTOMATED`: Phase 4 restart: `ServicesResolved` false for 30+ seconds after WirePlumber restart, only became true after manual disconnect/reconnect via bluetoothctl
- `VERIFIED_AUTOMATED`: Phase 4 restart: A2DP `SET_CONFIGURATION request rejected: Configuration not supported (41)` — iPhone rejected initial A2DP codec negotiation

### Resolved by source code analysis

- `VERIFIED_AUTOMATED`: `headset-head-unit` EnumProfile is gated on `connected_profiles & SPA_BT_PROFILE_HEADSET_HEAD_UNIT` (`bluez5-device.c:1977-1983`). It only appears AFTER HFP RFCOMM connects — not a config issue.
- `VERIFIED_AUTOMATED`: `bluez5.roles` property controls which profiles `register_profile()` registers with BlueZ (`backend-native.c:3533`), NOT which EnumProfiles appear. The config `bluez5.roles = [ hfp_hf, hfp_ag, ... ]` was correctly parsed.
- `VERIFIED_AUTOMATED`: `spa_bt_profiles_from_json_array()` parses SPA-JSON arrays (`bluez5-dbus.c:6276`). Format `[ a2dp_sink, hfp_hf ]` (unquoted) is correct.
- `VERIFIED_AUTOMATED`: `parse_headset_roles()` (`backend-native.c:3914`) reads `bluez5.roles` and filters with `SPA_BT_PROFILE_HEADSET_AUDIO`. Default `DEFAULT_ENABLED_PROFILES = HFP_HF | HFP_AG` when not set.
- `VERIFIED_AUTOMATED`: `register_profile()` for HFP HF does NOT set `AutoConnect` property. HSP HS explicitly sets `AutoConnect=0`. Neither has auto-connect enabled.
- `VERIFIED_AUTOMATED`: iPhone SDP shows HFP AG (UUID `0x111f`) on RFCOMM channel 8. No HFP HF service (iPhone IS the gateway).
- `VERIFIED_AUTOMATED`: `ConnectProfile("0x111e")` fails with `br-connection-profile-unavailable` because iPhone doesn't offer HF service.
- `VERIFIED_AUTOMATED`: `monitor.bluez.properties` section in `wireplumber.conf` is passed to `SpaDevice("api.bluez5.enum.dbus", config.properties)` (`bluez.lua:406`).

### Corrected (previous claim withdrawn)

- ~~`VERIFIED_AUTOMATED`: `ConnectProfile("0x111f")` returns 0 at D-Bus level but BlueZ never opens RFCOMM channel 8.~~ — **WITHDRAWN**. Phase 7d btmon proves RFCOMM SABM WAS sent by local Pi (TX) at 19.313s, UA received at 19.321s, and full AT negotiation completed. RFCOMM channel 8 (DLCI 0x10) was established successfully. The earlier conclusion was based on incomplete btmon analysis.
- ~~`VERIFIED_AUTOMATED`: After adapter power cycle (`hciconfig down/up`), iPhone reconnects A2DP only, never initiates HFP RFCOMM.~~ — **DOWNJECTED**. Adapter power cycle was a mistake per user instructions. Phase 7d shows locally-initiated RFCOMM DID establish. iPhone HFP reconnection behavior after power cycle remains UNKNOWN.

### Unknown (insufficient evidence)

- `UNKNOWN`: Whether modifying `register_profile()` to set `AutoConnect=true` for HFP HF would fix RFCOMM initiation
- `UNKNOWN`: Whether disconnecting iPhone from iOS Bluetooth settings forces HFP reconnection on next connect
- `UNKNOWN`: Whether the iPhone re-initiates HFP after a phone call is placed while connected

### Superseded (do not cite as current)

- ~~`PIPEWIRE_HFP_BACKEND_NOT_CONFIGURED`~~ — withdrawn. Default is already `native`.
- ~~"WirePlumber registers 0 Profile1 interfaces after restart"~~ — withdrawn. ObjectManager on `org.bluez` does not list client-owned objects. Wrong D-Bus service was inspected.
- ~~"Stale UUID registration persists across bluetoothd restart"~~ — withdrawn. One-shot busctl RegisterProfile is not persistent; registration disappears when the calling process exits.
- ~~"D-Bus rejects WirePlumber method_returns"~~ — withdrawn. "0 matched rules" log does not by itself prove the actual method reply was rejected.
- ~~`bluez5.roles=[hfp_hf]` causes device disappearance~~ — changed to INCONCLUSIVE. Device absence before a verified fresh HFP connection is expected behavior, not proof of configuration failure.

### Current classification

- `HFP_RFCOMM_WORKS_ENUMPROFILE_MISSING_AFTER_SLC` — RFCOMM connection established by local Pi, AT negotiation completes successfully (SLC established), but `headset-head-unit` EnumProfile never appears in PipeWire. Issue is in spa-bluez5 profile state management after NewConnection.

## Current blockers

1. **HFP RFCOMM cannot be re-established** — After disconnect/reconnect, no NewConnection for HFP, no Profile1 registered, `bluez5.profile` stays "off". This BLOCKS all HFP testing.
2. **D-Bus `method_return` rejection** — WirePlumber → bluetoothd (`Rejected send message, 0 matched rules`). Likely preventing Profile1 handshake completion.
3. **A2DP codec negotiation rejected** — `SET_CONFIGURATION request rejected: Configuration not supported (41)` — repeated across all sessions
4. **SCO connection reset by iPhone** (from Phase 4) — `spa.bluez5.sink.sco: failed to write data: -104 (Connection reset by peer)` — untestable until RFCOMM is restored

## Approved system changes
- Removed malformed system fragment `/etc/wireplumber/wireplumber.conf.d/51-bluez-hfp.conf` → `.invalid-disabled`
- Removed user-level isolation fragment `90-analogconnect-hfp-isolation.conf`

## Pending user actions
- None

## Next action
Investigate D-Bus method_return rejection preventing Profile1 registration. Possible options:
1. Restart bluetoothd (would clear all pairings — high cost)
2. Investigate D-Bus policy rules for WirePlumber → bluetoothd method_return
3. Check if WirePlumber's native backend is attempting RegisterProfile and failing silently

## Tests
- test-diagnostics.sh: 31/31 passing
- MH-MAP-001: PASS (MAP listing, retrieval working)
- MH-PBAP-001: PASS (PBAP listing working after reconnection)
- HFP ConnectProfile: PASS (RFCOMM established, AT negotiation successful — Phase 7c, 7d, 4b)
- HFP NewConnection callback: PASS (earlier instance)
- HFP SLC completion: PASS (Phase 7d and 4b)
- HFP SCO transport: PARTIAL (SCO sink created but reset by iPhone — error -104)
- HFP profile activation: FAIL (headset-head-unit appears briefly then disappears after SCO failure)
- Phase 4 WirePlumber restart: PARTIAL (RFCOMM+AT works, but ServicesResolved delayed, SCO fails)
- Phase 5D incoming-call test: BLOCKED (HFP RFCOMM cannot be re-established after disconnect/reconnect)

## Important decisions
- imsg works without bluez-obexd — uses own OBEX implementation
- iOS requires manual permission grant before first MAP/PBAP connection
- Re-pairing not justified — remote UUIDs are correct, service discovery complete
- Native backend and `hfp_hf` role are WirePlumber 0.5 defaults — no explicit config needed
- Profile1 objects are client-owned, not BlueZ-owned — cannot be found via BlueZ ObjectManager
- One-shot busctl RegisterProfile does not persist — registration removed when caller exits
- Registration ownership is connection-specific — UnregisterProfile from another client is not an existence test

## Unresolved questions
- Why does the D-Bus method_return rejection persist across WirePlumber restarts?
- Why doesn't WirePlumber's native backend register Profile1 for HFP after reconnect?
- Is there a D-Bus policy rule missing that would allow WirePlumber to reply to bluetoothd?
- Would a bluetoothd restart clear the D-Bus state? What is the cost (re-pairing)?
- Why does A2DP SET_CONFIGURATION keep failing with "Configuration not supported (41)"?
