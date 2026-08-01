# AnalogConnect Agent State

## Current milestone
Milestone 0

## Current phase
Phase F — PipeWire HFP Profile State Inspection

## Current objective
Determine the current PipeWire EnumProfile and active Profile state after the Phase E HFP SLC, and correlate the NewConnection callback.

## Current classification
`HFP_SLC_NOT_REFLECTED_IN_CONNECTED_PROFILES`

## Last completed action
Phase F1-F4: Inspected current PipeWire device without restart. EnumProfile contains only `off` and `audio-gateway` — `headset-head-unit` is absent. Active Profile is `audio-gateway` (Pi-as-AG). RFCOMM alive, SLC complete, ServicesResolved true. NewConnection not captured in Phase E D-Bus monitor (monitor started after RFCOMM establishment). Classification: `HFP_SLC_NOT_REFLECTED_IN_CONNECTED_PROFILES`.

## Evidence

### Established

- `VERIFIED_HARDWARE`: MAP works (with reconnection)
- `VERIFIED_HARDWARE`: PBAP works (with reconnection)
- `VERIFIED_AUTOMATED`: The iPhone advertises HFP Audio Gateway UUID `111f`
- `VERIFIED_AUTOMATED`: `ServicesResolved` is true (D-Bus property, current session)
- `VERIFIED_AUTOMATED`: WirePlumber (PID 298021, sender `:1.885`) registered `/Profile/HFPHF` (UUID 0x111e) and `/Profile/HFPAG` (UUID 0x111f) — Phase E D-Bus capture
- `VERIFIED_AUTOMATED`: Phase E btmon shows RFCOMM SABM sent by LOCAL Pi (TX), UA received — Pi initiated RFCOMM to iPhone AG service
- `VERIFIED_AUTOMATED`: Phase E btmon shows full AT negotiation: AT+BRSF=695→+BRSF:4079, AT+BAC=1,2,3, AT+CIND=?, AT+CIND?=1,0,0,5,2,0,0, AT+CMER=3,0,0,1, AT+CHLD=?, AT+CLIP=1, AT+CCWA=1, AT+CMEE=1, AT+CLCC — all OK
- `VERIFIED_AUTOMATED`: Phase E btmon shows A2DP (AVDTP PSM 25) and AVRCP (AVCTP PSM 23) also established alongside HFP RFCOMM
- `VERIFIED_AUTOMATED`: `/sys/kernel/debug/bluetooth/rfcomm` shows active RFCOMM session to `<REDACTED_BLUETOOTH_ADDRESS>` channel 8, dlci 16, mtu 1015
- `VERIFIED_AUTOMATED`: `/sys/kernel/debug/bluetooth/l2cap` shows active L2CAP PSM 3 (RFCOMM) to iPhone — CID 0x0041/0x0909, MTU 1021/2582
- `VERIFIED_AUTOMATED`: Phase E btmon shows NO RFCOMM disconnect frames — RFCOMM channel remains alive
- `VERIFIED_AUTOMATED`: Phase F1: `pw-cli enum-params 41 EnumProfile` returns only `off` (index 0) and `audio-gateway` (index 65536) — `headset-head-unit` is absent
- `VERIFIED_AUTOMATED`: Phase F1: Active Profile parameter is `audio-gateway` (index 65536) — Pi is configured as Audio Gateway (A2DP Source & HSP/HFP AG)
- `VERIFIED_AUTOMATED`: Phase F1: No HFP transport objects exist (`pw-cli ls Transport` returns empty)
- `VERIFIED_AUTOMATED`: Phase F1: No SCO sink or source nodes exist
- `VERIFIED_AUTOMATED`: Phase F1: `EnumRoute` and `Route` are empty
- `VERIFIED_AUTOMATED`: Phase F1: `bluetoothAudioCodec = sbc`, `bluetoothOffloadActive = false`
- `VERIFIED_AUTOMATED`: Phase 4 restart: `spa.bluez5.sink.sco: failed to write data: -104 (Connection reset by peer)` — headset-head-unit DID appear and SCO transport was created, but SCO link was reset by iPhone
- `VERIFIED_AUTOMATED`: Phase 4 restart: A2DP `SET_CONFIGURATION request rejected: Configuration not supported (41)` — iPhone rejected initial A2DP codec negotiation

### Corrected (previous claim withdrawn)

- ~~"PipeWire/WirePlumber never transitions the EnumProfile to connected because bluez5.profile remains off"~~ — **WITHDRAWN**. `bluez5.profile` is a device property describing an initial profile preference. It is NOT the authoritative active-profile state. The authoritative parameters are `EnumProfile` and `Profile` queried directly from the PipeWire Bluetooth Device object. After Phase E SLC, the active Profile is `audio-gateway` (index 65536), not `off`.
- ~~`VERIFIED_AUTOMATED`: `ConnectProfile("0x111f")` returns 0 at D-Bus level but BlueZ never opens RFCOMM channel 8.~~ — **WITHDRAWN**. Phase 7d and Phase E btmon prove RFCOMM SABM WAS sent by local Pi (TX), UA received, and full AT negotiation completed.
- ~~`VERIFIED_AUTOMATED`: After adapter power cycle (`hciconfig down/up`), iPhone reconnects A2DP only, never initiates HFP RFCOMM.~~ — **WITHDRAWN**. Adapter power cycle was a mistake per user instructions.
- ~~`VERIFIED_AUTOMATED`: `ConnectProfile("0x111e")` fails with `br-connection-profile-unavailable` because iPhone doesn't offer HF service.~~ — **WITHDRAWN**. BlueZ `Device1.ConnectProfile()` takes the remote service UUID. The correct remote UUID is `0000111f` (HFP Audio Gateway).

### Unknown (insufficient evidence)

- `UNKNOWN`: Whether the NewConnection callback was delivered to the current WirePlumber process (PID 298021) — RFCOMM and AT negotiation prove it happened, but the D-Bus monitor was not running at the time
- `UNKNOWN`: Whether the `connected_profiles` bitmask includes `SPA_BT_PROFILE_HEADSET_HEAD_UNIT`
- `UNKNOWN`: Whether `headset-head-unit` EnumProfile would appear after a fresh WirePlumber restart with proper logging
- `UNKNOWN`: Whether an incoming call would cause profile activation and EnumProfile update
- `UNKNOWN`: Whether the `audio-gateway` active Profile would interfere with HFP HF functionality

### Superseded (do not cite as current)

- ~~`PIPEWIRE_HFP_BACKEND_NOT_CONFIGURED`~~ — withdrawn. Default is already `native`.
- ~~"WirePlumber registers 0 Profile1 interfaces after restart"~~ — withdrawn. ObjectManager on `org.bluez` does not list client-owned objects.
- ~~"Stale UUID registration persists across bluetoothd restart"~~ — withdrawn. One-shot busctl RegisterProfile is not persistent.
- ~~"D-Bus rejects WirePlumber method_returns"~~ — withdrawn. "0 matched rules" log does not by itself prove the actual method reply was rejected.
- ~~`bluez5.roles=[hfp_hf]` causes device disappearance~~ — changed to INCONCLUSIVE.
- ~~`HFP_CURRENT_REGISTRATION_AND_CONNECT_STATE_UNRESOLVED`~~ — superseded. Registration is now verified, RFCOMM is established, SLC is complete.
- ~~`HFP_CONTROL_CONNECTION_DROPPED`~~ — superseded. RFCOMM is alive.

### Current classification

- `HFP_SLC_NOT_REFLECTED_IN_CONNECTED_PROFILES` — The HFP service-level control negotiation is verified at the HCI level (RFCOMM channel 8 established, full AT negotiation completed, no disconnect observed). However, `headset-head-unit` is absent from the PipeWire EnumProfile. The active Profile parameter is `audio-gateway` (Pi-as-AG), which does not provide HFP hands-free audio.

## Current blockers

1. **`headset-head-unit` EnumProfile absent** — The EnumProfile contains only `off` and `audio-gateway`. No HFP hands-free profile is available. This blocks the incoming-call test.
2. **No HFP transport objects** — No BlueZ HFP transport exists in PipeWire
3. **No SCO nodes** — No SCO sink or source nodes exist
4. **A2DP codec negotiation rejected** — `SET_CONFIGURATION request rejected: Configuration not supported (41)` — repeated across all sessions
5. **SCO connection reset by iPhone** (from Phase 4) — `spa.bluez5.sink.sco: failed to write data: -104 (Connection reset by peer)` — untestable until HFP profile is activated

## Approved system changes
- Removed malformed system fragment `/etc/wireplumber/wireplumber.conf.d/51-bluez-hfp.conf` → `.invalid-disabled`
- Removed user-level isolation fragment `90-analogconnect-hfp-isolation.conf`

## Pending user actions
- None

## Next action
Enable temporary detailed SPA Bluetooth logging for one controlled reconnection to determine why `headset-head-unit` is absent from EnumProfile despite the completed SLC. The exact logging method, risk, and rollback must be shown before execution.

## Tests
- test-diagnostics.sh: 31/31 passing
- MH-MAP-001: PASS (MAP listing, retrieval working)
- MH-PBAP-001: PASS (PBAP listing working after reconnection)
- HFP RFCOMM establishment: PASS (Phase E — SABM TX, UA received, channel 8, dlci 16)
- HFP AT negotiation: PASS (Phase E — all AT commands return OK)
- HFP SLC alive: PASS (Phase F2 — RFCOMM in debugfs, no disconnect in btmon)
- HFP NewConnection callback: NOT_CAPTURED (Phase E D-Bus monitor started after RFCOMM)
- HFP EnumProfile: FAIL — `headset-head-unit` absent; only `off` and `audio-gateway`
- HFP transport: FAIL — no HFP transport objects
- HFP SCO: FAIL — no SCO nodes
- Phase 4 WirePlumber restart: PARTIAL (RFCOMM+AT works, but ServicesResolved delayed, SCO fails)
- Phase 5D incoming-call test: BLOCKED (EnumProfile absent — no HFP audio pathway)

## Important decisions
- imsg works without bluez-obexd — uses own OBEX implementation
- iOS requires manual permission grant before first MAP/PBAP connection
- Re-pairing not justified — remote UUIDs are correct, service discovery complete
- Native backend and `hfp_hf` role are WirePlumber 0.5 defaults — no explicit config needed
- Profile1 objects are client-owned, not BlueZ-owned — cannot be found via BlueZ ObjectManager
- One-shot busctl RegisterProfile does not persist — registration removed when caller exits
- `bluez5.profile` is NOT the authoritative active-profile state — EnumProfile and Profile parameters are authoritative

## Unresolved questions
- Why is `headset-head-unit` absent from EnumProfile despite the SLC being established?
- Was the NewConnection callback delivered to the current WirePlumber process?
- Does the `connected_profiles` bitmask include `SPA_BT_PROFILE_HEADSET_HEAD_UNIT`?
- Would an incoming call cause profile activation and EnumProfile update?
- Why does A2DP SET_CONFIGURATION keep failing with "Configuration not supported (41)"?
