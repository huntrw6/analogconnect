# AnalogConnect Current State

This document is authoritative for current project status. Earlier phase documents
remain as investigation history; where they disagree with this document, their
interpretations are superseded.

## Current commit

- Baseline audited at `00c2c12` on branch `main`.
- `VERIFIED_AUTOMATED`: the audit began with a clean working tree.

## Current architecture

```text
iPhone MAP / PBAP / HFP / eSCO
              |
      Raspberry Pi 5
      BlueZ + PipeWire/WirePlumber + imsg
              |
       analogconnectd (Milestone 1)
              |
   authenticated REST + WebSocket + audio transport
              |
       Android 8.1 application (later milestone)
```

The repository currently contains a Bash feasibility harness. The Rust daemon and
Android application have not yet been implemented.

## Environment

- `VERIFIED_AUTOMATED`: Raspberry Pi 5, Debian 13/trixie, aarch64.
- `VERIFIED_AUTOMATED`: BlueZ 5.82.
- `VERIFIED_AUTOMATED`: PipeWire 1.4.2 and WirePlumber 0.5.8.
- `VERIFIED_AUTOMATED`: Rust/Cargo 1.97.1, ShellCheck 0.10.0, imsg 0.3.1.
- Target Android version: Android 8.1 / API 27.

## Verified capabilities

### MAP

- `VERIFIED_HARDWARE`: folder and inbox listing work through `imsg`.
- `VERIFIED_HARDWARE`: at least one message body was retrieved.
- `VERIFIED_AUTOMATED`: MAP commands worked after Phase I call teardown.

### PBAP

- `VERIFIED_HARDWARE`: contact listing works through `imsg`.
- `VERIFIED_HARDWARE`: at least 456 contacts were listed during testing.
- `VERIFIED_AUTOMATED`: PBAP commands worked after Phase I call teardown.

### HFP and eSCO

- `VERIFIED_AUTOMATED`: the Pi initiates HFP RFCOMM channel 8 as HF to the iPhone AG.
- `VERIFIED_AUTOMATED`: WirePlumber receives and accepts `Profile1.NewConnection`.
- `VERIFIED_AUTOMATED`: the complete HFP SLC succeeds and call indicators update.
- `VERIFIED_AUTOMATED`: the iPhone selected mSBC and an eSCO connection was established.
- `VERIFIED_AUTOMATED`: SCO packets flowed in both directions.
- `VERIFIED_AUTOMATED`: SCO released cleanly while RFCOMM remained alive.

## Unverified capabilities

- `UNKNOWN`: human-confirmed intelligible call audio in either direction.
- `UNKNOWN`: Pi-originated answer, reject, hangup, dial, DTMF, mute, and gain control.
- `UNKNOWN`: MAP Message Notification Service behavior and reliable incremental sync.
- `UNKNOWN`: MAP sending, delivery state, MMS, attachments, and locked-iPhone behavior.
- `UNKNOWN`: complete PBAP records, phone-number extraction, normalization, and caller matching.
- `UNKNOWN`: automatic recovery after reboot, Bluetooth loss, or network loss.
- `UNKNOWN`: Android device hardware characteristics and end-to-end Android behavior.

## Superseded findings

- `headset-head-unit` is not required for the Pi-as-HF architecture. The remote
  iPhone appears under PipeWire's `audio-gateway` profile.
- Idle absence of `+BCS`, an SCO transport, or SCO nodes is not a failure. The HFP
  SLC and Audio Connection are separate lifecycles.
- `bluez5.profile="off"` is not the authoritative active-profile state.
- `ConnectProfile(111f)` uses the correct remote HFP Audio Gateway UUID.
- D-Bus policy, stale registration, and missing native-backend root-cause claims
  were withdrawn after later captures.
- Phase I proves bidirectional SCO packet flow, not intelligible audio.

## Known issues

- A2DP `SET_CONFIGURATION` was rejected with `Configuration not supported (41)`.
  This is separate from the verified MAP/PBAP/HFP/eSCO path.
- Historical documents contain duplicated and superseded investigation conclusions.
- Raw hardware evidence is private and remains ignored under `test-results/`.

## Safety constraints

- Obtain approval before sudo, package installation, service/configuration changes,
  Bluetooth pairing changes, networking changes, or Bluetooth interruption.
- Never commit device addresses, pairing material, telephone data, contacts,
  messages, captured audio, authentication tokens, or raw private captures.
- Never record call audio.

## Next milestone

Milestone 1: implement a manually runnable Rust `analogconnectd` skeleton with
explicit state models, mockable Bluetooth interfaces, redacted structured logging,
graceful shutdown, and versioned health/status endpoints. Do not deploy it as a
system service yet.

## End-to-end roadmap

1. Backend state model and API skeleton.
2. PBAP contact synchronization and caller matching.
3. MAP incoming synchronization and notification/polling fallback.
4. Hardware-verified outgoing MAP messages.
5. Hardware-verified HFP call-control commands.
6. Pi SCO audio bridge and transport benchmarks.
7. Android API-27 control-plane application.
8. Android call audio.
9. Full integration, recovery, security, deployment, and maintenance.
