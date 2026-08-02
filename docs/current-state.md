# AnalogConnect Current State

This document is authoritative for current project status. Earlier phase documents
remain as investigation history; where they disagree with this document, their
interpretations are superseded.

## Current implementation

- Milestone 0A baseline and Milestone 1 backend skeleton are complete.
- Milestone 2 contact synchronization software is implemented and awaits a
  privacy-controlled hardware validation before it can be classified as
  `VERIFIED_HARDWARE` end to end.
- Milestone 3 MAP synchronization orchestration is implemented around imsg's
  encrypted store and awaits notification behavior validation with the iPhone.
- Milestone 5 HFP call-control domain and AT encoding are implemented behind a
  mock transport; no live command has been sent to the iPhone.
- Milestone 6 in-memory audio bridge queues and aggregate instrumentation are
  implemented; they are not yet connected to PipeWire or an Android transport.

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
daemon. The daemon exposes hardware-free health/status/contact-summary APIs and a
privacy-safe PBAP adapter, transactional SQLite contact store, normalization,
search, and caller matching. The Android application has not yet been implemented.

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

### Contact synchronization software

- `VERIFIED_AUTOMATED`: sanitized `imsg contacts --raw` output is parsed without
  logging or debug-printing names and numbers.
- `VERIFIED_AUTOMATED`: complete contact snapshots replace SQLite state atomically;
  failed pulls preserve the previous snapshot and enter backoff.
- `VERIFIED_AUTOMATED`: normalized caller lookup returns an exact or unique suffix
  match and refuses ambiguous matches.
- `VERIFIED_AUTOMATED`: case-insensitive name search escapes SQL wildcard input.
- `VERIFIED_AUTOMATED`: the contact database is created with user-only permissions
  on Unix and the API exposes aggregate counts only.
- `DOCUMENTED`: local `imsg` 0.3.1 source defines `contacts --raw` as full PBAP
  contact output with normalization disabled.

### Message synchronization software

- `VERIFIED_AUTOMATED`: message sync starts in polling mode rather than assuming
  that MAP notifications are operational.
- `VERIFIED_AUTOMATED`: relevant MAP events trigger inbox synchronization;
  notification silence falls back to bounded polling.
- `VERIFIED_AUTOMATED`: sync success/failure counters and backoff transitions are
  exposed without addresses, handles, or message bodies.
- `VERIFIED_AUTOMATED`: the `imsg` command adapter discards stdout and stderr and
  returns only redacted failure classes.
- `DOCUMENTED`: local `imsg` 0.3.1 source provides encrypted persistence,
  incremental per-folder cursors, broker `Watch` events, and sync/outbox states.

### HFP call-control software

- `VERIFIED_AUTOMATED`: answer, reject, hangup, dial, DTMF, mute, and gain
  commands are validated against call state before reaching a backend.
- `VERIFIED_AUTOMATED`: failed backend commands preserve the prior call state.
- `VERIFIED_AUTOMATED`: dial targets and DTMF values are redacted from `Debug`
  output and backend errors contain no command payload.
- `VERIFIED_AUTOMATED`: validated commands encode to HFP AT operations; mute uses
  microphone gain zero and restores the last configured gain.
- `UNKNOWN`: which live transport seam can safely share WirePlumber's existing
  RFCOMM ownership without disrupting the verified SLC.

### Audio bridge software

- `VERIFIED_AUTOMATED`: narrowband and wideband HFP PCM frames have explicit
  7.5 ms format invariants and reject mismatched payload sizes.
- `VERIFIED_AUTOMATED`: independent uplink/downlink queues are bounded and drop
  the oldest frame on overflow to prevent unbounded latency growth.
- `VERIFIED_AUTOMATED`: audio frame `Debug` output and the aggregate API contain
  no sample values.
- `VERIFIED_AUTOMATED`: queue depth, drop count, throughput, and maximum observed
  in-memory latency are tracked without recording audio.
- `UNKNOWN`: PipeWire node binding, codec conversion, network transport latency,
  and intelligibility with real call audio.

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
- `UNKNOWN`: real-iPhone acceptance and effects of Pi-originated call-control AT commands.
- `UNKNOWN`: MAP Message Notification Service behavior and reliable incremental sync.
- `UNKNOWN`: whether iPhone MAP notifications remain reliable across idle periods,
  reconnects, and locked-device states; polling remains the safe default.
- `UNKNOWN`: MAP sending, delivery state, MMS, attachments, and locked-iPhone behavior.
- `UNKNOWN`: compatibility of the parser with the real iPhone's complete
  `imsg contacts --raw` output; no private contact payload has been captured or committed.
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

Milestone 4: hardware-verified outgoing MAP messages, followed by HFP call control.
Before that work, validate PBAP parsing and MAP notification behavior with the real
iPhone using aggregate-only output. Do not deploy the daemon as a system service yet.

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
