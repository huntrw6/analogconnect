# AnalogConnect Agent State

## Current milestone
Milestone 0

## Current phase
Phase 0C — Diagnostic scripts (completed)

## Current objective
Document upstream research; prepare for Phase 0D

## Last completed action
Created and tested three diagnostic scripts: doctor.sh, inspect-device.sh, collect-logs.sh — all 31 tests passing

## Evidence
- `VERIFIED_AUTOMATED`: doctor.sh — read-only system health check, 13 system checks, JSON output
- `VERIFIED_AUTOMATED`: inspect-device.sh — Bluetooth device profile inspector, MAC validation
- `VERIFIED_AUTOMATED`: collect-logs.sh — privacy-safe log collector with manifest generation
- `VERIFIED_AUTOMATED`: 31/31 tests passing in test-diagnostics.sh
- `VERIFIED_AUTOMATED`: doctor.sh produces valid JSON output
- `VERIFIED_AUTOMATED`: collect-logs.sh collects 9 diagnostic files by default

## Current blockers
- ShellCheck not installed (cannot lint scripts)
- Rust/Cargo not installed (may be needed for some build dependencies)
- CMake not installed (may be needed for some build dependencies)

## Approved system changes
- None (read-only scripts only)

## Pending user actions
- Approval needed for package installations in Phase 0D

## Next action
Begin Phase 0D — focused upstream research (BlueZ, PipeWire, imsg, oFono)

## Tests
- test-diagnostics.sh: 31/31 passing

## Important decisions
- Use MIT license for original AnalogConnect code
- Repository initialized on `main` branch
- Doctor script exits 0 (pass), 1 (fail), 2 (warn/blocked), 64 (usage error)
- All scripts redact Bluetooth addresses by default
- collect-logs.sh requires --include-sensitive for paired device names

## Unresolved questions
- Are MAP/PBAP client profiles available on BlueZ 5.82 with this controller?
- Will SCO audio route through PipeWire without additional configuration?
