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

The Android app is now part of the active product milestone. The feasibility
work remains relevant evidence, but the project has moved into end-to-end
productization.

## Current milestone

Milestone 7 — Integrated Android companion product

Current priorities, in order:

1. Add privacy-safe, fail-closed group detection using the verified ANCS signal;
   keep group replies disabled until a verified group target exists.
2. Preserve reliable Android-to-iPhone outbound private messaging.
3. Preserve the hardware-verified two-way call-audio path while making call
   recovery and startup dependable.
4. Build the interactive Android call and conversation interfaces on the
   authenticated control and media transports already in the repository.
5. Complete contacts, incoming messages, notifications, native call integration,
   installation, recovery, and release validation.

Authoritative status is in `docs/current-state.md`. The execution plan and
definition of done are in `docs/product-roadmap.md`. Historical phase documents
are investigation records and may contain superseded conclusions.
The current group-messaging feasibility evidence and implementation guidance are
in `docs/group-chat-detection-report.md`.

## Current hardware evidence

- `VERIFIED_HARDWARE`: a real call had clear, intelligible audio in both
  directions through the Android, Pi, and iPhone path.
- `VERIFIED_HARDWARE`: an earlier Android-originated SMS traversed the Pi and
  iPhone and reached its recipient.
- `VERIFIED_HARDWARE`: Android-originated SMS sending works again after granting
  the daemon narrowly scoped write access to imsg's data/state directories.
- `VERIFIED_HARDWARE`: the prior outbound-message regression was caused by the
  daemon's read-only home sandbox conflicting with `imsg send` writable
  store/state requirements.
- `VERIFIED_HARDWARE`: controlled ANCS testing distinguished two group Messages
  notifications (Title+Subtitle) from a clean direct notification and its update
  (Title only). Classification is `ANCS_GROUP_DETECTION_ONLY`; no safe group reply
  target is available, so group replies remain disabled.
- `VERIFIED_HARDWARE`: follow-up ephemeral-HMAC testing promoted incoming identity
  to `ANCS_STABLE_GROUP_IDENTITY_VERIFIED`: different senders in one group matched,
  a different group differed, and direct remained Subtitle-absent. This local key
  is not a safe group reply target.
- `BLOCKED`: Apple's iOS 26.5 Accessory Notifications reply path cannot be tested
  in the Linux/Pi environment because it requires a current Mac/Xcode SDK and
  provisioned accessory extension entitlements.

## Autonomous work boundary

Continue independently through source inspection, repository changes, automated
tests, privacy-safe diagnostics, documentation, and hardware-free implementation.
Stop when progress requires operator interaction, physical-phone observations, a
system change covered by the approval gate below, or access to private data.

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
