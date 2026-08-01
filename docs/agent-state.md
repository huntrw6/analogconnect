# AnalogConnect Agent State

## Current milestone
Milestone 0

## Current phase
Phase 6 — HFP Callback Corrected (completed)

## Current objective
ROOT CAUSE IDENTIFIED: `bluez5.hfphsp-backend` not configured — WirePlumber received RFCOMM fd but didn't activate headset-head-unit

## Last completed action
Ran corrected D-Bus monitor (no path filter) + ConnectProfile. NEW FINDING: BlueZ DID invoke `/Profile/HFPHF/NewConnection` — WirePlumber received the RFCOMM fd (inode 1091667). But `bluez5.profile` remained `"off"`. Root cause: `bluez5.hfphsp-backend = native` is NOT set in default WirePlumber config — only in disabled fragments.

## Evidence
- `VERIFIED_AUTOMATED`: BlueZ `ConnectProfile(111f)` succeeds
- `VERIFIED_AUTOMATED`: RFCOMM connection established (btmon SABM/UA frames)
- `VERIFIED_AUTOMATED`: HFP AT negotiation completes — all commands return OK
- `VERIFIED_AUTOMATED`: `/Profile/HFPHF/NewConnection` IS invoked by BlueZ at correct path
- `VERIFIED_AUTOMATED`: WirePlumber (PID 275878) received RFCOMM fd (inode 1091667)
- `VERIFIED_AUTOMATED`: `headset-head-unit` absent from EnumProfile
- `VERIFIED_AUTOMATED`: `bluez5.profile = "off"` — HFP profile not activated
- `VERIFIED_AUTOMATED`: No HFP transport or SCO objects created
- `VERIFIED_AUTOMATED`: `bluez5.hfphsp-backend` NOT set in default config
- `VERIFIED_HARDWARE`: MAP works after HFP connection
- `VERIFIED_HARDWARE`: PBAP works after HFP connection

## Current blockers
- `bluez5.hfphsp-backend = native` not set in default WirePlumber config
- Without this, WirePlumber's spa-bluez5 doesn't activate native HFP backend
- `headset-head-unit` not exposed in PipeWire EnumProfile

## Approved system changes
- Disabled three custom WirePlumber config fragments (renamed to .disabled)
- Removed temporary test fragment 90-analogconnect-hfp-test.conf

## Pending user actions
- None

## Next action
Create `/etc/wireplumber/wireplumber.conf.d/51-bluez-hfp.conf` with `bluez5.hfphsp-backend = native`. Restart WirePlumber. Retest ConnectProfile to verify headset-head-unit appears.

## Tests
- test-diagnostics.sh: 31/31 passing
- MH-MAP-001: PASS (MAP listing, retrieval working)
- MH-PBAP-001: PASS (PBAP listing working)
- HFP D-Bus registration trace: PASS (both HF and AG registered)
- iPhone service discovery: PASS (`111f` present, `ServicesResolved: true`)
- HFP ConnectProfile: PASS (RFCOMM established, AT negotiation successful)
- HFP NewConnection callback: PASS (corrected monitor confirmed delivery at /Profile/HFPHF)
- HFP profile activation: FAIL (bluez5.hfphsp-backend not configured)

## Important decisions
- imsg works without bluez-obexd — uses own OBEX implementation
- iOS requires manual permission grant before first MAP/PBAP connection
- Re-pairing not justified — remote UUIDs are correct, service discovery complete
- Custom config fragments with quoted-string `bluez5.roles` are invalid syntax
- NewConnection callback confirmed at `/Profile/HFPHF` — WirePlumber DID receive the RFCOMM fd
- Root cause is missing `bluez5.hfphsp-backend = native` — spa-bluez5 doesn't activate HFP backend
- Fix: create `/etc/wireplumber/wireplumber.conf.d/51-bluez-hfp.conf` with single setting
- Previous disabled config fragments had correct `hfphsp-backend` setting but were disabled due to syntax errors in other settings

## Unresolved questions
- Will `bluez5.hfphsp-backend = native` enable headset-head-unit in PipeWire?
- Will HFP audio (SCO) work after the fix?
- Do we need additional settings like `bluez5.roles` or `bluez5.headset-roles`?
