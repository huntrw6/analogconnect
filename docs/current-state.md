# AnalogConnect Current State

This document is authoritative for current project status. Earlier phase documents
remain as investigation history; where they disagree with this document, their
interpretations are superseded.

## Current commit

- Milestone 0A baseline: `a65706d`.
- Milestone 1 backend skeleton: `eb451e1`.

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

The repository contains the Bash feasibility harness plus a manually runnable Rust
daemon skeleton. The daemon currently exposes hardware-free health/status APIs;
Bluetooth adapters and the Android application have not yet been implemented.

## Environment

- `VERIFIED_AUTOMATED`: Raspberry Pi 5, Debian 13/trixie, aarch64.
- `VERIFIED_AUTOMATED`: BlueZ 5.82.
- `VERIFIED_AUTOMATED`: PipeWire 1.4.2 and WirePlumber 0.5.8.
- `VERIFIED_AUTOMATED`: Rust/Cargo 1.97.1, ShellCheck 0.10.0, imsg 0.3.1.
- Target Android version: Android 8.1 / API 27.

## Verified capabilities

### Backend skeleton

- `VERIFIED_AUTOMATED`: explicit independent Bluetooth, message, contact, HFP,
  call, audio, and Android-client states have validated transitions.
- `VERIFIED_AUTOMATED`: Bluetooth-facing boundaries are mockable without hardware.
- `VERIFIED_AUTOMATED`: `GET /api/v1/health` and `GET /api/v1/status` pass API tests.
- `VERIFIED_AUTOMATED`: Rust formatting, Clippy with warnings denied, and 13 Rust tests pass.
- `VERIFIED_AUTOMATED`: the existing 31-test Bash suite still passes.
- `VERIFIED_AUTOMATED`: the daemon bound to loopback, returned both endpoints,
  and stopped cleanly on Ctrl-C during a local smoke test.
- `DOCUMENTED`: `protocol/openapi-v1.yaml` is the versioned initial control-plane contract.

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

Milestone 2: PBAP complete-contact synchronization. First build a privacy-safe
`imsg` adapter and sanitized fixtures behind the existing `PbapBackend` boundary,
then add SQLite persistence, phone-number normalization, search, and caller matching.
Do not deploy the daemon as a system service yet.

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
