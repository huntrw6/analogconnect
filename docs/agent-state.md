# AnalogConnect Agent State

## Current milestone
Milestone 3 — MAP incoming synchronization — SOFTWARE COMPLETE, HARDWARE VALIDATION PENDING

## Current phase
Hardware-free backend foundations and authenticated server control plane — COMPLETE

## Current objective
Validate PBAP parsing and MAP notification behavior against the real iPhone under
privacy-controlled procedures, then proceed to outgoing MAP and HFP call control.

## Current classification
`HFP_SCO_BIDIRECTIONAL_PACKET_FLOW_VERIFIED`

## Last completed action
Installed and launched the signed API-27 APK on the real Android 8.1 phone. ADB
reported successful installation and activity launch, and the app process remained
running. No device identifier or application data was retained.

## Evidence

### Established

- `VERIFIED_HARDWARE`: MAP works (with reconnection)
- `VERIFIED_HARDWARE`: PBAP works (with reconnection)
- `VERIFIED_AUTOMATED`: The iPhone advertises HFP Audio Gateway UUID `111f`
- `VERIFIED_AUTOMATED`: `ServicesResolved` is true (D-Bus property, current session)
- `VERIFIED_AUTOMATED`: WirePlumber (PID 298021, sender `:1.885`) registered `/Profile/HFPHF` (UUID 0x111e) and `/Profile/HFPAG` (UUID 0x111f) — Phase E D-Bus capture
- `VERIFIED_AUTOMATED`: Phase E btmon shows RFCOMM SABM sent by LOCAL Pi (TX), UA received — Pi initiated RFCOMM to iPhone AG service
- `VERIFIED_AUTOMATED`: Phase H: Fresh NewConnection reached current WirePlumber process (`:1.885`)
- `VERIFIED_AUTOMATED`: Phase H: WirePlumber accepted the RFCOMM descriptor
- `VERIFIED_AUTOMATED`: Phase H: AT+BRSF completed — `+BRSF:4079`
- `VERIFIED_AUTOMATED`: Phase H: AT+BAC=1,2,3 completed
- `VERIFIED_AUTOMATED`: Phase H: AT+CIND=? completed — 7 indicators mapped
- `VERIFIED_AUTOMATED`: Phase H: AT+CIND? completed — `+CIND: 1,0,0,4,2,0,0`
- `VERIFIED_AUTOMATED`: Phase H: AT+CMER=3,0,0,1 completed
- `VERIFIED_AUTOMATED`: Phase H: AT+CHLD=? completed — `+CHLD: (0,1,1x,2,2x,3)`
- `VERIFIED_AUTOMATED`: Phase H: AT+CLIP=1 completed
- `VERIFIED_AUTOMATED`: Phase H: AT+CCWA=1 completed
- `VERIFIED_AUTOMATED`: Phase H: AT+CMEE=1 completed
- `VERIFIED_AUTOMATED`: Phase H: AT+CLCC completed
- `VERIFIED_AUTOMATED`: Phase H: call=0, callsetup=0, callheld=0
- `VERIFIED_AUTOMATED`: Phase H: RFCOMM remained connected after SLC
- `VERIFIED_AUTOMATED`: Phase H: `telephony_ag_register` called — AudioGateway registered
- `VERIFIED_AUTOMATED`: Phase I: iPhone sent `+BCS:2` (mSBC codec selection) — incoming call
- `VERIFIED_AUTOMATED`: Phase I: WirePlumber sent `AT+BCS=2` → OK (codec confirmed)
- `VERIFIED_AUTOMATED`: Phase I: HCI Connect Request (eSCO) → Accept Synchronous Connection → Synchronous Connect Complete (Handle 6)
- `VERIFIED_AUTOMATED`: Phase I: Bidirectional SCO data flowing (SCO Data RX + TX, Handle 6, dlen 60)
- `VERIFIED_AUTOMATED`: Phase I: `+CIEV: 3,1` → `call=1` → `callsetup=0` → `+CIEV: 2,0` → `call=0` (call indicator transitions)
- `VERIFIED_AUTOMATED`: Phase I: `telephony_call_register` and `telephony_call_unregister` lifecycle
- `VERIFIED_AUTOMATED`: Phase I: SCO transport stopped and released cleanly after hangup
- `VERIFIED_AUTOMATED`: Phase I: RFCOMM retained after call — DLC alive post-call
- `VERIFIED_AUTOMATED`: Phase I: MAP folders and inbox listed post-call (exit 0)
- `VERIFIED_AUTOMATED`: Phase I: PBAP contacts pulled post-call (exit 0)
- `VERIFIED_AUTOMATED`: `/sys/kernel/debug/bluetooth/rfcomm` shows active RFCOMM session to iPhone channel 8, dlci 16, mtu 1015
- `VERIFIED_AUTOMATED`: Phase G4: `DisconnectProfile("0000111f-...")` successfully removes RFCOMM session
- `VERIFIED_AUTOMATED`: Phase G6: `ConnectProfile("0000111f-...")` successfully creates fresh RFCOMM session
- `VERIFIED_AUTOMATED`: Phase G6: D-Bus monitor captured `NewConnection` delivered to `Destination=:1.885` (current WirePlumber) on `/Profile/HFPHF`, accepted with method_return success (3ms)
- `VERIFIED_AUTOMATED`: Phase 4 restart: `spa.bluez5.sink.sco: failed to write data: -104 (Connection reset by peer)` — headset-head-unit DID appear and SCO transport was created, but SCO link was reset by iPhone
- `VERIFIED_AUTOMATED`: Phase 4 restart: A2DP `SET_CONFIGURATION request rejected: Configuration not supported (41)` — iPhone rejected initial A2DP codec negotiation
- `DOCUMENTED`: Profile mapping: `path_to_profile("/Profile/HFPHF")` → `SPA_BT_PROFILE_HFP_AG` → `SPA_BT_PROFILE_HEADSET_AUDIO_GATEWAY`. The `headset-head-unit` EnumProfile requires `SPA_BT_PROFILE_HEADSET_HEAD_UNIT = SPA_BT_PROFILE_HSP_HS | SPA_BT_PROFILE_HFP_HF`, which is only set when a remote device connects to our AG profile.
- `DOCUMENTED`: Service Level Connection and Audio Connection are separate procedures. Codec negotiation and SCO are initiated only when audio is needed.

### Post-Phase H PipeWire State

- `VERIFIED_AUTOMATED`: No HFP transport was present in the post-test PipeWire state
- `VERIFIED_AUTOMATED`: No SCO source or sink was present in the post-test PipeWire state
- These are expected possibilities while idle — they may appear only when audio is requested

### Corrected (previous claim withdrawn)

- ~~`HFP_SLC_COMPLETED_BUT_NO_CODEC_NEGOTIATION`~~ — **WITHDRAWN**. Not a failure classification. Idle lack of codec negotiation is expected behavior.
- ~~`HFP_CURRENT_OWNER_ACCEPTS_BUT_PIPELINE_INACTIVE`~~ — **SUPERSEDED**. Phase H proved the AT SLC completes successfully.
- ~~`HFP_RFCOMM_PRECEDES_CURRENT_WIREPLUMBER_REGISTRATION`~~ — **REFUTED**. Phase G proved the current WirePlumber DOES receive and accept NewConnection.
- ~~`HFP_SLC_NOT_REFLECTED_IN_CONNECTED_PROFILES`~~ — **SUPERSEDED**. The SLC is reflected in connected_profiles.

### Superseded (do not cite as current)

- ~~`PIPEWIRE_HFP_BACKEND_NOT_CONFIGURED`~~ — withdrawn
- ~~"WirePlumber registers 0 Profile1 interfaces after restart"~~ — withdrawn
- ~~"Stale UUID registration persists across bluetoothd restart"~~ — withdrawn
- ~~"D-Bus rejects WirePlumber method_returns"~~ — withdrawn
- ~~`HFP_CURRENT_REGISTRATION_AND_CONNECT_STATE_UNRESOLVED`~~ — superseded
- ~~`HFP_CONTROL_CONNECTION_DROPPED`~~ — superseded

## Current state summary

```
Milestone 0: BLUETOOTH FEASIBILITY COMPLETE
MAP: VERIFIED_HARDWARE
PBAP: VERIFIED_HARDWARE
HFP SLC and call indicators: VERIFIED_AUTOMATED
HFP codec/eSCO/SCO packet flow: VERIFIED_AUTOMATED
Pi-originated call commands: UNKNOWN
Human-confirmed intelligible audio: UNKNOWN
RFCOMM alive post-call
MAP/PBAP operational post-call
SCO cleanly torn down after hangup
```

## Do not require before the call

- `headset-head-unit` — may appear only when a remote device connects to our AG profile
- An HFP audio transport — may be created only when audio is requested
- SCO nodes — may appear only when audio is routed to the Pi
- `+BCS` — the AG sends `+BCS:<codec>` when it initiates codec selection; the HF responds with `AT+BCS=<codec>`

## Approved system changes
- Removed malformed system fragment `/etc/wireplumber/wireplumber.conf.d/51-bluez-hfp.conf` → `.invalid-disabled`
- Removed user-level isolation fragment `90-analogconnect-hfp-isolation.conf`

## Pending user actions
- Confirm the Android UI is visible and exercise synthetic-token secure enrollment.
  MAP notification and HFP command validation still require guided iPhone interaction.

## Next action
Validate the Android UI and Keystore enrollment with a synthetic token, design the
secure phone-to-Pi transport, then continue sanitized iPhone hardware tests.

## Tests
- test-diagnostics.sh: 31/31 passing
- Rust unit/API tests: 13/13 passing
- Android API-27 debug APK build and signature verification: passing
- Android 8.1 hardware install, activity launch, and live-process check: passing
- Cargo fmt: passing
- Cargo Clippy with warnings denied: passing
- Manual loopback daemon health/status and graceful-shutdown smoke test: passing
- MH-MAP-001: PASS (MAP listing, retrieval working)
- MH-PBAP-001: PASS (PBAP listing working after reconnection)
- HFP RFCOMM establishment: PASS (Phase E, G, H — SABM TX, UA received, channel 8, dlci 16)
- HFP AT SLC: PASS (Phase H — all AT commands return OK, full WirePlumber trace)
- HFP SLC alive: PASS (Phase F, H — RFCOMM in debugfs, no disconnect)
- HFP NewConnection callback: VERIFIED (Phase G, H — delivered to `:1.885`, accepted)
- HFP DisconnectProfile/ConnectProfile: VERIFIED (Phase G — RFCOMM successfully removed and re-created)
- HFP control plane: VERIFIED (Phase H — SLC complete, indicators synchronized, RFCOMM alive)
- HFP incoming-call transport test: VERIFIED (Phase I — +BCS codec negotiation, eSCO established, bidirectional SCO packets, teardown, RFCOMM retained, MAP/PBAP post-call)

## Important decisions
- imsg works without bluez-obexd — uses own OBEX implementation
- iOS requires manual permission grant before first MAP/PBAP connection
- Re-pairing not justified — remote UUIDs are correct, service discovery complete
- Native backend and `hfp_hf` role are WirePlumber 0.5 defaults — no explicit config needed
- Profile1 objects are client-owned, not BlueZ-owned — cannot be found via BlueZ ObjectManager
- One-shot busctl RegisterProfile does not persist — registration removed when caller exits
- `bluez5.profile` is NOT the authoritative active-profile state — EnumProfile and Profile parameters are authoritative
- Pi-as-HF → `SPA_BT_PROFILE_HFP_AG` → `SPA_BT_PROFILE_HEADSET_AUDIO_GATEWAY` (NOT `HEADSET_HEAD_UNIT`)
- Service Level Connection and Audio Connection are separate procedures
- Codec negotiation and SCO are initiated only when audio is needed
- Idle lack of +BCS is expected behavior, not a failure
