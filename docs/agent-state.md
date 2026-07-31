# AnalogConnect Agent State

## Current milestone
Milestone 0

## Current phase
Phase 0D — Upstream research (completed)

## Current objective
Begin Phase 0E — feasibility harness design

## Last completed action
Completed research on BlueZ, PipeWire, WirePlumber, gnufood/imsg, and oFono. Documented findings in research-sources.md.

## Evidence
- `DOCUMENTED`: BlueZ 5.87 supports MAP client, PBAP client, HFP via oFono
- `DOCUMENTED`: PipeWire 1.4.2 supports bidirectional SCO audio, HFP HF role
- `VERIFIED_AUTOMATED`: gnufood/imsg v0.3.1 — full MAP + PBAP client, no HFP
- `DOCUMENTED`: oFono works but requires modem emulator — fallback only
- `INFERRED`: Integration of imsg (MAP/PBAP) + PipeWire (HFP/SCO) is feasible

## Current blockers
- ShellCheck not installed
- Rust/Cargo not installed (needed for imsg build)

## Approved system changes
- None (read-only so far)

## Pending user actions
- Approval needed for package installations

## Next action
Begin Phase 0E — feasibility harness design

## Tests
- test-diagnostics.sh: 31/31 passing

## Important decisions
- Use imsg for MAP and PBAP client functionality
- Use PipeWire for HFP call control and SCO audio
- oFono is fallback only — not primary approach
- Integration layer needed to coordinate imsg + PipeWire

## Unresolved questions
- Can PipeWire telephony API replace oFono for HFP control?
- How to coordinate imsg daemon with PipeWire audio session?
