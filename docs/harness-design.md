# AnalogConnect — Feasibility Harness Design

## Overview

Minimal command-line tool for Milestone 0 feasibility testing. Each command tests one capability and returns a clear result state.

## Result states

```text
PASS              Capability verified and working
FAIL              Capability detected but not working
BLOCKED           Cannot test without hardware interaction or missing dependency
NOT_SUPPORTED     Device does not advertise this capability
NOT_CONFIGURED    Component exists but not configured for this use
USER_ACTION_REQUIRED  User must perform an action on the iPhone
```

## Commands

### analogconnect status

**Purpose**: Show overall system readiness for Bluetooth feasibility testing.

**Inputs**: None

**Outputs**: Human-readable summary (or `--json`)

**Dependencies**: doctor.sh

**Implemented**: Yes (Phase 0C — uses doctor.sh)

### analogconnect adapters

**Purpose**: List available Bluetooth adapters and their state.

**Inputs**: None

**Outputs**: Adapter name, address (redacted), powered state, mode

**Dependencies**: bluetoothctl

**Implemented**: Yes (wrapper around bluetoothctl show)

### analogconnect devices

**Purpose**: List paired Bluetooth devices.

**Inputs**: None

**Outputs**: Device name, address (redacted), paired/connected state

**Dependencies**: bluetoothctl

**Implemented**: Yes (wrapper around bluetoothctl paired-devices)

### analogconnect inspect-device

**Purpose**: Inspect a specific paired device for supported profiles.

**Inputs**: `--device <MAC>`

**Outputs**: Profile checklist (MAP, PBAP, HFP, A2DP, etc.)

**Dependencies**: bluetoothctl, inspect-device.sh

**Implemented**: Yes (Phase 0C — uses inspect-device.sh)

### analogconnect test-map

**Purpose**: Test MAP (Message Access Profile) connection to a paired device.

**Inputs**: `--device <MAC>`

**Outputs**: PASS/FAIL with message count or error

**Dependencies**: imsg (GNUfood); `bluez-obexd` is not required for the verified path

**Implemented**: No in the shell harness. The installed `imsg` CLI has been used directly against the paired device.

**Requires real hardware**: Yes

### analogconnect test-pbap

**Purpose**: Test PBAP (Phonebook Access Profile) connection to a paired device.

**Inputs**: `--device <MAC>`

**Outputs**: PASS/FAIL with contact count or error

**Dependencies**: imsg (GNUfood); `bluez-obexd` is not required for the verified path

**Implemented**: No in the shell harness. The installed `imsg` CLI has been used directly against the paired device.

**Requires real hardware**: Yes

### analogconnect test-hfp

**Purpose**: Test HFP (Hands-Free Profile) connection and call detection.

**Inputs**: `--device <MAC>`

**Outputs**: PASS/FAIL with connection state

**Dependencies**: BlueZ, PipeWire (or oFono)

**Implemented**: No — requires paired device and phone call test

**Requires real hardware**: Yes

### analogconnect test-sco

**Purpose**: Test bidirectional SCO audio routing.

**Inputs**: `--device <MAC>`

**Outputs**: PASS/FAIL with audio routing state

**Dependencies**: PipeWire, WirePlumber, BlueZ

**Implemented**: No — requires paired device and active call

**Requires real hardware**: Yes

### analogconnect collect-logs

**Purpose**: Collect diagnostic logs for analysis.

**Inputs**: `--output <dir>`, `--include-sensitive`

**Outputs**: Log files with manifest

**Dependencies**: collect-logs.sh

**Implemented**: Yes (Phase 0C — uses collect-logs.sh)

## Privacy behavior

- All commands redact Bluetooth addresses by default
- `--no-redact` disables redaction (requires explicit opt-in)
- `--include-sensitive` enables collection of device names
- Message bodies, contact names, phone numbers are never collected by default
- Pairing keys are never collected

## Implementation approach

For Milestone 0, implement only commands that provide immediate value:

1. **analogconnect status** — already available via doctor.sh
2. **analogconnect adapters** — thin wrapper, easy to implement
3. **analogconnect devices** — thin wrapper, easy to implement
4. **analogconnect inspect-device** — already available via inspect-device.sh
5. **analogconnect collect-logs** — already available via collect-logs.sh

Commands requiring real hardware (test-map, test-pbap, test-hfp, test-sco) are designed but not implemented until paired device is available.

## File structure

```text
bin/
  analogconnect          # Main entry point (shell script)
scripts/
  doctor.sh              # System diagnostics
  inspect-device.sh      # Device profile inspection
  collect-logs.sh        # Log collection
```

## Implementation

The main `analogconnect` script is a thin dispatcher that delegates to the appropriate sub-script or subcommand.
