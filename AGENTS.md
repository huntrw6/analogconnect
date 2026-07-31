# AGENTS.md — AnalogConnect Development Guide

## Project purpose

AnalogConnect lets an Android 8.1 slider phone act as a companion device for a nearby iPhone. The iPhone remains the real cellular phone.

```
iPhone
  ├── Bluetooth MAP/PBAP → messages and contacts
  └── Bluetooth HFP/SCO  → calls and call audio
                            ↓
                    Raspberry Pi bridge
                            ↓
                    Android slider app
```

The Android app is **not part of the current milestone**.

## Current milestone

Milestone 0 — Raspberry Pi and iPhone Bluetooth feasibility

## Operating rules

### Evidence labels

Distinguish every conclusion as one of:

- `VERIFIED_AUTOMATED` — demonstrated by a passing automated test
- `VERIFIED_HARDWARE` — demonstrated with the real iPhone
- `DOCUMENTED` — supported by official source code or documentation
- `INFERRED` — reasonable but not yet tested
- `UNKNOWN` — insufficient evidence
- `FAILED` — attempted and did not work
- `BLOCKED` — cannot proceed without permission, hardware interaction, or missing capability

### System safety rules

Before any command that uses `sudo`, installs packages, changes a system service, modifies configuration, changes groups, or affects Bluetooth pairing, show:

1. Exact command
2. Purpose
3. Expected result
4. Risk
5. Rollback command or procedure

Then stop and request approval.

### Privacy rules

Never commit:

- Bluetooth addresses
- Pairing data
- Telephone numbers
- Contact names
- Messages
- Captured audio
- Personal logs
- Credentials

## Required commands

Before any commit:
```bash
git diff --check
git status
```

## Linting and typecheck

Shell scripts: `shellcheck scripts/*.sh`
