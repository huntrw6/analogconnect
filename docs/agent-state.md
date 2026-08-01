# AnalogConnect Agent State

## Current milestone
Milestone 0

## Current phase
Phase 6 — HFP Isolation Test Complete (manual checkpoint)

## Current objective
Determine why a successful HFP Hands-Free service-level connection does not produce `headset-head-unit` or an HFP transport/audio path in PipeWire.

## Current classification
`HFP_CONTROL_CONNECTED_PIPEWIRE_AUDIO_PROFILE_MISSING`

## Last completed action
Completed HFP isolation test cycle. Found: (1) `bluez5.roles=[hfp_hf]` causes Bluetooth device to disappear from PipeWire entirely, (2) WirePlumber does NOT register HFP Profile1 interfaces after restart — only A2DP MediaEndpoints are registered, (3) stale UUID `111e` registration in BlueZ blocks re-registration.

## Evidence

### Established

- `VERIFIED_AUTOMATED`: BlueZ `ConnectProfile(111f)` succeeds
- `VERIFIED_AUTOMATED`: RFCOMM connection established (btmon SABM/UA frames)
- `VERIFIED_AUTOMATED`: HFP AT negotiation completes — all commands return OK
- `VERIFIED_AUTOMATED`: `/Profile/HFPHF/NewConnection` IS invoked by BlueZ at correct path
- `VERIFIED_AUTOMATED`: WirePlumber received RFCOMM fd (inode 1091667)
- `VERIFIED_AUTOMATED`: Pi sent HFP Hands-Free AT commands; iPhone responded as Audio Gateway
- `VERIFIED_AUTOMATED`: `headset-head-unit` absent from EnumProfile
- `VERIFIED_AUTOMATED`: `bluez5.profile = "off"` — HFP profile not activated
- `VERIFIED_AUTOMATED`: No HFP transport or SCO objects created
- `VERIFIED_AUTOMATED`: Only `off` and `audio-gateway` in EnumProfile — no `headset-head-unit`
- `VERIFIED_AUTOMATED`: `bluez5.roles=[hfp_hf]` isolation fragment causes Bluetooth device to disappear from PipeWire entirely
- `VERIFIED_AUTOMATED`: WirePlumber registers A2DP MediaEndpoints but NOT HFP Profile1 interfaces
- `VERIFIED_AUTOMATED`: After bluetoothd + wireplumber restart, 0 Profile1 interfaces in ObjectManager
- `VERIFIED_AUTOMATED`: `RegisterProfile(111e)` fails with "UUID already registered" — stale registration persists across bluetoothd restart
- `VERIFIED_AUTOMATED`: D-Bus `method_return` from WirePlumber to bluetoothd rejected — "0 matched rules"
- `VERIFIED_HARDWARE`: MAP works (with reconnection)
- `VERIFIED_HARDWARE`: PBAP works (with reconnection)

### Superseded (do not cite as current)

- ~~`PIPEWIRE_HFP_BACKEND_NOT_CONFIGURED`~~ — withdrawn. Based on malformed fragment syntax and incorrect assumption that native backend was not running.
- ~~"Missing `bluez5.hfphsp-backend` caused the failure"~~ — not established. Default is already `native`.
- ~~Previous D-Bus RegisterProfile evidence~~ — from old WirePlumber instance that has since been killed and restarted. After clean restart, no Profile1 registration occurs.

### Current classification

- `HFP_CONTROL_CONNECTED_PIPEWIRE_AUDIO_PROFILE_MISSING` — HFP control connection succeeds but `headset-head-unit` and HFP transport are absent from PipeWire.

## Current blockers

1. WirePlumber does NOT register HFP Profile1 interfaces — only A2DP MediaEndpoints are registered during startup
2. Stale UUID `111e` registration in BlueZ blocks manual re-registration — persists across bluetoothd restart
3. `bluez5.roles=[hfp_hf]` isolation causes device disappearance — cannot isolate HFP-only behavior
4. D-Bus policy rejects WirePlumber method_returns to bluetoothd — "0 matched rules"

## Approved system changes
- Removed malformed system fragment `/etc/wireplumber/wireplumber.conf.d/51-bluez-hfp.conf` → `.invalid-disabled`
- Removed user-level isolation fragment `90-analogconnect-hfp-isolation.conf`
- Restarted bluetoothd and WirePlumber

## Pending user actions
- None

## Next action
Phase 7: Inspect exact Debian PipeWire 1.4.2 source to understand why HFP Profile1 registration does not occur after WirePlumber restart. Specifically: when does `spa_bt_device_add_profile` get called? What condition prevents HFP HF profile construction? Why does `bluez5.roles=[hfp_hf]` cause device disappearance?

## Tests
- test-diagnostics.sh: 31/31 passing
- MH-MAP-001: PASS (MAP listing, retrieval working)
- MH-PBAP-001: PASS (PBAP listing working after reconnection)
- HFP D-Bus registration trace: PASS (both HF and AG registered — old instance)
- iPhone service discovery: PASS (`111f` present, `ServicesResolved: true`)
- HFP ConnectProfile: PASS (RFCOMM established, AT negotiation successful)
- HFP NewConnection callback: PASS (corrected monitor confirmed delivery)
- HFP profile activation: FAIL (`headset-head-unit` absent)
- HFP Profile1 registration after restart: FAIL (0 Profile1 interfaces)
- Isolation fragment test: FAIL (device disappears with `bluez5.roles=[hfp_hf]`)
- Manual RegisterProfile: FAIL ("UUID already registered" — stale)

## Important decisions
- imsg works without bluez-obexd — uses own OBEX implementation
- iOS requires manual permission grant before first MAP/PBAP connection
- Re-pairing not justified — remote UUIDs are correct, service discovery complete
- Custom config fragments with quoted-string `bluez5.roles` are invalid syntax
- NewConnection callback confirmed at `/Profile/HFPHF` — WirePlumber DID receive the RFCOMM fd
- Native backend and `hfp_hf` role are WirePlumber 0.5 defaults — no explicit config needed
- `bluez5.roles=[hfp_hf]` is NOT a safe isolation setting — causes device disappearance
- WirePlumber restart does NOT re-register HFP Profile1 — only A2DP endpoints are registered
- Stale BlueZ profile registrations persist across bluetoothd restarts
- D-Bus policy may reject WirePlumber method_returns — needs investigation

## Unresolved questions
- Why does WirePlumber NOT register HFP Profile1 interfaces after restart?
- Why does `bluez5.roles=[hfp_hf]` cause the Bluetooth device to disappear entirely?
- Why does stale UUID registration persist across bluetoothd restart?
- Is the D-Bus rejection causing profile registration failure?
- What condition in spa-bluez5 prevents HFP HF profile construction?
- Does the phone form factor affect profile construction?
