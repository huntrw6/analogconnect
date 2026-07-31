# AnalogConnect Agent State

## Current milestone
Milestone 0

## Current phase
Phase 4 — HFP registration analysis (completed)

## Current objective
Trace PipeWire profile construction to understand why `headset-head-unit` is not exposed

## Last completed action
Inspected iPhone remote HFP services. Confirmed remote `111f` present, `ServicesResolved: true`, local `111e` registered, but PipeWire EnumProfile only exposes `audio-gateway`. Cleaned up all invalid configuration fragments. Verified MAP and PBAP still work.

## Evidence
- `VERIFIED_AUTOMATED`: WirePlumber native backend registered local HFP HF UUID `0000111e`
- `VERIFIED_AUTOMATED`: WirePlumber native backend registered local HFP AG UUID `0000111f`
- `VERIFIED_AUTOMATED`: PipeWire exposes `audio-gateway` (Pi-as-AG role)
- `VERIFIED_AUTOMATED`: PipeWire does NOT expose `headset-head-unit`
- `VERIFIED_HARDWARE`: iPhone advertises HFP AG UUID `0000111f`
- `VERIFIED_HARDWARE`: `ServicesResolved: true`
- `VERIFIED_HARDWARE`: MAP works with paired iPhone
- `VERIFIED_HARDWARE`: PBAP works with paired iPhone

## Current blockers
- `headset-head-unit` profile not in PipeWire EnumProfile despite successful BlueZ registration
- Failure layer: PIPEWIRE_PROFILE_ENUMERATION

## Approved system changes
- Disabled three custom WirePlumber config fragments (renamed to .disabled)
- Removed temporary test fragment 90-analogconnect-hfp-test.conf

## Pending user actions
- None

## Next action
Trace PipeWire profile construction — investigate why spa-bluez5 in PipeWire 1.4.2 does not create `headset-head-unit` EnumProfile entry for device advertising HFP AG UUID `111f`

## Tests
- test-diagnostics.sh: 31/31 passing
- MH-MAP-001: PASS (MAP listing, retrieval working)
- MH-PBAP-001: PASS (PBAP listing working)
- HFP D-Bus registration trace: PASS (both HF and AG registered)
- iPhone service discovery: PASS (`111f` present, `ServicesResolved: true`)

## Important decisions
- imsg works without bluez-obexd — uses own OBEX implementation
- iOS requires manual permission grant before first MAP/PBAP connection
- Re-pairing not justified — remote UUIDs are correct, service discovery complete
- Custom config fragments with quoted-string `bluez5.roles` are invalid syntax

## Unresolved questions
- Why does spa-bluez5 not create `headset-head-unit` EnumProfile despite compiling the profile name?
- Is `audio-gateway` profile sufficient for HFP call control on this PipeWire version?
