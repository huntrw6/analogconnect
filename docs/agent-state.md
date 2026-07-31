# AnalogConnect Agent State

## Current milestone
Milestone 0

## Current phase
Phase 5 — HFP ConnectProfile test (completed)

## Current objective
Investigate BlueZ profile matching for HFP — determine why BlueZ does not route incoming HFP RFCOMM connection to WirePlumber's registered `/Profile/HFPHF`

## Last completed action
Executed explicit `ConnectProfile(0000111f)` to iPhone. HFP RFCOMM connection established and AT negotiation completed successfully. But BlueZ did not invoke `/Profile/HFPHF/NewConnection` — WirePlumber never received the connection.

## Evidence
- `VERIFIED_AUTOMATED`: BlueZ `ConnectProfile(111f)` succeeds
- `VERIFIED_AUTOMATED`: RFCOMM connection established (btmon SABM/UA frames)
- `VERIFIED_AUTOMATED`: HFP AT negotiation completes — all commands return OK
- `VERIFIED_AUTOMATED`: `/Profile/HFPHF/NewConnection` NOT invoked by BlueZ
- `VERIFIED_AUTOMATED`: `headset-head-unit` absent from EnumProfile
- `VERIFIED_AUTOMATED`: No HFP transport or SCO objects created
- `VERIFIED_HARDWARE`: MAP works after HFP connection
- `VERIFIED_HARDWARE`: PBAP works after HFP connection

## Current blockers
- BlueZ does not route HFP connection to WirePlumber's registered `/Profile/HFPHF`
- Root cause: BlueZ profile matching issue, not PipeWire enumeration

## Approved system changes
- Disabled three custom WirePlumber config fragments (renamed to .disabled)
- Removed temporary test fragment 90-analogconnect-hfp-test.conf

## Pending user actions
- None

## Next action
Investigate BlueZ profile matching — examine why BlueZ does not match incoming HFP AG connection to WirePlumber's `/Profile/HFPHF`. May require checking BlueZ `profile.c` source or registration parameters.

## Tests
- test-diagnostics.sh: 31/31 passing
- MH-MAP-001: PASS (MAP listing, retrieval working)
- MH-PBAP-001: PASS (PBAP listing working)
- HFP D-Bus registration trace: PASS (both HF and AG registered)
- iPhone service discovery: PASS (`111f` present, `ServicesResolved: true`)
- HFP ConnectProfile: PASS (RFCOMM established, AT negotiation successful)
- HFP NewConnection callback: FAIL (BlueZ did not invoke)

## Important decisions
- imsg works without bluez-obexd — uses own OBEX implementation
- iOS requires manual permission grant before first MAP/PBAP connection
- Re-pairing not justified — remote UUIDs are correct, service discovery complete
- Custom config fragments with quoted-string `bluez5.roles` are invalid syntax
- HFP failure is in BlueZ profile matching, not PipeWire profile enumeration

## Unresolved questions
- Why does BlueZ not match incoming HFP AG connection to WirePlumber's `/Profile/HFPHF`?
- Are the registration parameters (Features, Version) correct for HFP HF profile?
- Does BlueZ need different profile registration to accept incoming HFP connections?
