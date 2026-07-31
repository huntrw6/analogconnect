# AnalogConnect Agent State

## Current milestone
Milestone 0

## Current phase
Phase 6 — Corrected HFP callback trace (in progress)

## Current objective
Re-run HFP callback monitor with correct D-Bus object path (`/Profile/HFPHF` not `/org/bluez/Profile/HFPHF`). Determine which process handled the RFCOMM connection.

## Last completed action
Correction: Previous callback monitor used path `/org/bluez/Profile/HFPHF` which differs from registered path `/Profile/HFPHF`. The prior "no NewConnection" result is now classified as `PROFILE_CALLBACK_MONITOR_INCONCLUSIVE`. Preparing corrected trace.

## Evidence
- `VERIFIED_AUTOMATED`: BlueZ `ConnectProfile(111f)` succeeds
- `VERIFIED_AUTOMATED`: RFCOMM connection established (btmon SABM/UA frames)
- `VERIFIED_AUTOMATED`: HFP AT negotiation completes — all commands return OK
- `INCONCLUSIVE`: `/Profile/HFPHF/NewConnection` — previous monitor used wrong path
- `VERIFIED_AUTOMATED`: `headset-head-unit` absent from EnumProfile
- `VERIFIED_AUTOMATED`: No HFP transport or SCO objects created
- `VERIFIED_HARDWARE`: MAP works after HFP connection
- `VERIFIED_HARDWARE`: PBAP works after HFP connection

## Current blockers
- HFP callback monitor result is inconclusive due to wrong D-Bus path filter
- `headset-head-unit` absent from EnumProfile
- Need to determine which process handled the RFCOMM connection

## Approved system changes
- Disabled three custom WirePlumber config fragments (renamed to .disabled)
- Removed temporary test fragment 90-analogconnect-hfp-test.conf

## Pending user actions
- None

## Next action
Run corrected HFP callback trace with path-independent D-Bus monitor. Identify which process owned the RFCOMM file descriptor during AT negotiation.

## Tests
- test-diagnostics.sh: 31/31 passing
- MH-MAP-001: PASS (MAP listing, retrieval working)
- MH-PBAP-001: PASS (PBAP listing working)
- HFP D-Bus registration trace: PASS (both HF and AG registered)
- iPhone service discovery: PASS (`111f` present, `ServicesResolved: true`)
- HFP ConnectProfile: PASS (RFCOMM established, AT negotiation successful)
- HFP NewConnection callback: INCONCLUSIVE (wrong D-Bus path in monitor)

## Important decisions
- imsg works without bluez-obexd — uses own OBEX implementation
- iOS requires manual permission grant before first MAP/PBAP connection
- Re-pairing not justified — remote UUIDs are correct, service discovery complete
- Custom config fragments with quoted-string `bluez5.roles` are invalid syntax
- HFP callback monitor used wrong D-Bus path — result inconclusive
- HFP RFCOMM and AT negotiation succeed — some HF implementation is working
- Need to identify which process handled the connection

## Unresolved questions
- Which process received the HFP RFCOMM file descriptor?
- Was WirePlumber's `/Profile/HFPHF` callback invoked at the correct path?
- Is there another HFP implementation handling the connection?
- Why is `headset-head-unit` absent despite successful AT negotiation?
