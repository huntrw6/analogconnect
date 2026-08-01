# AnalogConnect Agent State

## Current milestone
Milestone 0

## Current phase
Phase G — Current-Owner Connection Test (complete)

## Current objective
Determine whether the current WirePlumber process can receive a fresh NewConnection and add `headset-head-unit` to PipeWire EnumProfile. The answer is: WirePlumber receives and accepts NewConnection, but the HFP pipeline does not activate.

## Current classification
`HFP_CURRENT_OWNER_ACCEPTS_BUT_PIPELINE_INACTIVE`

## Last completed action
Phase G1-G7: Complete current-owner connection test. DisconnectProfile removed old RFCOMM, ConnectProfile created fresh RFCOMM, NewConnection delivered to `:1.885` (current WirePlumber PID 298021) and accepted. RFCOMM active at kernel level but no AT SLC completed, no HFP transport created, no `headset-head-unit` EnumProfile. This refutes the earlier `HFP_RFCOMM_PRECEDES_CURRENT_WIREPLUMBER_REGISTRATION` hypothesis.

## Evidence

### Established

- `VERIFIED_HARDWARE`: MAP works (with reconnection)
- `VERIFIED_HARDWARE`: PBAP works (with reconnection)
- `VERIFIED_AUTOMATED`: The iPhone advertises HFP Audio Gateway UUID `111f`
- `VERIFIED_AUTOMATED`: `ServicesResolved` is true (D-Bus property, current session)
- `VERIFIED_AUTOMATED`: WirePlumber (PID 298021, sender `:1.885`) registered `/Profile/HFPHF` (UUID 0x111e) and `/Profile/HFPAG` (UUID 0x111f) — Phase E D-Bus capture
- `VERIFIED_AUTOMATED`: Phase E btmon shows RFCOMM SABM sent by LOCAL Pi (TX), UA received — Pi initiated RFCOMM to iPhone AG service
- `VERIFIED_AUTOMATED`: Phase E btmon shows full AT negotiation: AT+BRSF=695→+BRSF:4079, AT+BAC=1,2,3, AT+CIND=?, AT+CIND?=1,0,0,5,2,0,0, AT+CMER=3,0,0,1, AT+CHLD=?, AT+CLIP=1, AT+CCWA=1, AT+CMEE=1, AT+CLCC — all OK
- `VERIFIED_AUTOMATED`: `/sys/kernel/debug/bluetooth/rfcomm` shows active RFCOMM session to iPhone channel 8, dlci 16, mtu 1015
- `VERIFIED_AUTOMATED`: Phase G4: `DisconnectProfile("0000111f-...")` successfully removes RFCOMM session
- `VERIFIED_AUTOMATED`: Phase G6: `ConnectProfile("0000111f-...")` successfully creates fresh RFCOMM session
- `VERIFIED_AUTOMATED`: Phase G6: D-Bus monitor captured `NewConnection` delivered to `Destination=:1.885` (current WirePlumber) on `/Profile/HFPHF`, accepted with method_return success (3ms)
- `VERIFIED_AUTOMATED`: Phase G6: After NewConnection acceptance, RFCOMM active (DLCI 16, MTU 1015) but NO HCI traffic visible — AT SLC did not complete
- `VERIFIED_AUTOMATED`: Phase G7: PipeWire EnumProfile unchanged — `headset-head-unit` STILL ABSENT after fresh connection
- `VERIFIED_AUTOMATED`: Phase G7: No HFP transport objects, no SCO sink/source nodes
- `DOCUMENTED`: Profile mapping: `path_to_profile("/Profile/HFPHF")` → `SPA_BT_PROFILE_HFP_AG` → `SPA_BT_PROFILE_HEADSET_AUDIO_GATEWAY`. The `headset-head-unit` EnumProfile requires `SPA_BT_PROFILE_HEADSET_HEAD_UNIT = SPA_BT_PROFILE_HSP_HS | SPA_BT_PROFILE_HFP_HF`, which is only set when a remote device connects to our AG profile.
- `VERIFIED_AUTOMATED`: Phase 4 restart: `spa.bluez5.sink.sco: failed to write data: -104 (Connection reset by peer)` — headset-head-unit DID appear and SCO transport was created, but SCO link was reset by iPhone
- `VERIFIED_AUTOMATED`: Phase 4 restart: A2DP `SET_CONFIGURATION request rejected: Configuration not supported (41)` — iPhone rejected initial A2DP codec negotiation

### Corrected (previous claim withdrawn)

- ~~`HFP_RFCOMM_PRECEDES_CURRENT_WIREPLUMBER_REGISTRATION`~~ — **REFUTED**. Phase G proved the current WirePlumber DOES receive and accept NewConnection. The RFCOMM timing is not the cause.
- ~~`HFP_SLC_NOT_REFLECTED_IN_CONNECTED_PROFILES`~~ — **SUPERSEDED**. The SLC from Phase E is reflected in `connected_profiles` (the `audio-gateway` EnumProfile is active). The issue is that AT SLC does not complete on fresh connections, AND the profile mapping means `SPA_BT_PROFILE_HFP_AG` (Pi-as-HF) does not contribute to `SPA_BT_PROFILE_HEADSET_HEAD_UNIT`.
- ~~"PipeWire/WirePlumber never transitions the EnumProfile to connected because bluez5.profile remains off"~~ — **WITHDRAWN**. `bluez5.profile` is NOT the authoritative active-profile state.
- ~~`VERIFIED_AUTOMATED`: `ConnectProfile("0x111f")` returns 0 at D-Bus level but BlueZ never opens RFCOMM channel 8.~~ — **WITHDRAWN**.
- ~~`VERIFIED_AUTOMATED`: `ConnectProfile("0x111e")` fails with `br-connection-profile-unavailable`.~~ — **WITHDRAWN**. Correct remote UUID is `0x111f`.

### Unknown (insufficient evidence)

- `UNKNOWN`: Whether the AT SLC negotiation completed on the fresh RFCOMM (btmon missed the window)
- `UNKNOWN`: Whether WirePlumber's backend-native properly registered the RFCOMM FD with its event loop
- `UNKNOWN`: Whether `headset-head-unit` would appear if the iPhone initiated a connection to our AG profile (UUID 0x111f)
- `UNKNOWN`: Whether an incoming call would cause profile activation
- `UNKNOWN`: Why AT SLC does not complete on fresh connections (RFCOMM is established but silent)

### Superseded (do not cite as current)

- ~~`PIPEWIRE_HFP_BACKEND_NOT_CONFIGURED`~~ — withdrawn
- ~~"WirePlumber registers 0 Profile1 interfaces after restart"~~ — withdrawn
- ~~"Stale UUID registration persists across bluetoothd restart"~~ — withdrawn
- ~~"D-Bus rejects WirePlumber method_returns"~~ — withdrawn
- ~~`HFP_CURRENT_REGISTRATION_AND_CONNECT_STATE_UNRESOLVED`~~ — superseded
- ~~`HFP_CONTROL_CONNECTION_DROPPED`~~ — superseded

## Current blockers

1. **AT SLC does not complete on fresh connections** — RFCOMM is established at kernel level, WirePlumber accepts NewConnection, but no AT exchange is visible. This prevents HFP transport creation.
2. **Profile mapping prevents `headset-head-unit`** — Pi-as-HF maps to `SPA_BT_PROFILE_HFP_AG` which is part of `SPA_BT_PROFILE_HEADSET_AUDIO_GATEWAY`, not `SPA_BT_PROFILE_HEADSET_HEAD_UNIT`. The `headset-head-unit` EnumProfile requires `SPA_BT_PROFILE_HFP_HF` (Pi-as-AG).
3. **No HFP transport objects** — No BlueZ HFP transport exists in PipeWire
4. **No SCO nodes** — No SCO sink or source nodes exist
5. **A2DP codec negotiation rejected** — `SET_CONFIGURATION request rejected: Configuration not supported (41)`

## Approved system changes
- Removed malformed system fragment `/etc/wireplumber/wireplumber.conf.d/51-bluez-hfp.conf` → `.invalid-disabled`
- Removed user-level isolation fragment `90-analogconnect-hfp-isolation.conf`

## Pending user actions
- None

## Next action
Investigate why AT SLC does not complete on fresh RFCOMM connections. Enable detailed SPA Bluetooth logging (`SPA_DEBUG=1` or `PIPEWIRE_DEBUG=3`) for one controlled reconnection to capture the backend-native AT exchange. Alternatively, investigate whether the RFCOMM FD is properly registered with WirePlumber's event loop.

## Tests
- test-diagnostics.sh: 31/31 passing
- MH-MAP-001: PASS (MAP listing, retrieval working)
- MH-PBAP-001: PASS (PBAP listing working after reconnection)
- HFP RFCOMM establishment: PASS (Phase E — SABM TX, UA received, channel 8, dlci 16)
- HFP AT negotiation: PASS (Phase E — all AT commands return OK)
- HFP SLC alive: PASS (Phase F2 — RFCOMM in debugfs, no disconnect in btmon)
- HFP NewConnection callback: **VERIFIED** (Phase G — delivered to `:1.885`, accepted)
- HFP DisconnectProfile/ConnectProfile: **VERIFIED** (Phase G — RFCOMM successfully removed and re-created)
- HFP EnumProfile: FAIL — `headset-head-unit` absent; only `off` and `audio-gateway`
- HFP transport: FAIL — no HFP transport objects
- HFP SCO: FAIL — no SCO nodes
- HFP AT SLC on fresh connection: FAIL — RFCOMM established but AT exchange not visible
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
- Pi-as-HF → `SPA_BT_PROFILE_HFP_AG` → `SPA_BT_PROFILE_HEADSET_AUDIO_GATEWAY` (NOT `HEADSET_HEAD_UNIT`)

## Unresolved questions
- Why does AT SLC not complete on fresh RFCOMM connections?
- Is the RFCOMM FD properly registered with WirePlumber's event loop?
- Would `headset-head-unit` appear if the iPhone initiated a connection to our AG profile?
- Would an incoming call cause profile activation?
- Why does A2DP SET_CONFIGURATION keep failing with "Configuration not supported (41)"?
