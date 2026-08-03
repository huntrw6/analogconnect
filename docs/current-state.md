# AnalogConnect Current State

This document is authoritative for current project status. Earlier phase documents
remain as investigation history; where they disagree with this document, their
interpretations are superseded.

## Current implementation

- Milestone 0A baseline and Milestone 1 backend skeleton are complete.
- Milestone 2 contact parsing is validated against the real iPhone using an
  aggregate-only pipeline; persistence/search remain hardware-free validated.
- Milestone 3 MAP synchronization orchestration is implemented around imsg's
  encrypted store and awaits notification behavior validation with the iPhone.
- Milestone 4 outbound MAP messaging is complete: validation, authenticated API,
  redacted transport, Android confirmation UI, iPhone acceptance, and recipient
  delivery have been verified.
- Milestone 5 HFP call-control domain, AT encoding, WirePlumber Telephony D-Bus
  adapter, authenticated API, and Android controls are implemented; live command
  effects on the iPhone still require guided validation.
- Milestone 6 in-memory audio bridge queues and aggregate instrumentation are
  implemented alongside a framed-PCM diagnostic codec, bounded jitter buffer,
  PipeWire SCO discovery, managed `pw-cat` binding, and exact PCM frame adapters;
  they are not yet connected to an Android network transport.
- The server-side Android control plane now requires bearer authentication for all
  non-health endpoints and refuses startup without an explicit token.
- An API-27 Android client foundation builds as a signed debug APK on the ARM64
  Raspberry Pi and installs and launches on the target Android 8.1 phone.

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
       Android 8.1 application foundation
```

The repository contains the Bash feasibility harness plus a manually runnable Rust
daemon. The daemon exposes hardware-free health/status/contact-summary APIs and a
privacy-safe PBAP adapter, transactional SQLite contact store, normalization,
search, and caller matching. The initial Android application securely stores its
bearer token and can perform privacy-safe health and authenticated status checks.

## Environment

- `VERIFIED_AUTOMATED`: Raspberry Pi 5, Debian 13/trixie, aarch64.
- `VERIFIED_AUTOMATED`: BlueZ 5.82.
- `VERIFIED_AUTOMATED`: PipeWire 1.4.2 and WirePlumber 0.5.8.
- `VERIFIED_AUTOMATED`: Rust/Cargo 1.97.1, ShellCheck 0.10.0, imsg 0.3.1.
- Target Android version: Android 8.1 / API 27.
- `VERIFIED_AUTOMATED`: OpenJDK 21, Android API 27 platform files, ARM64 Android
  build tools, and pinned R8/D8 8.3.41 produce a signature-verified APK.

## Verified capabilities

### Backend skeleton

- `VERIFIED_AUTOMATED`: explicit independent Bluetooth, message, contact, HFP,
  call, audio, and Android-client states have validated transitions.
- `VERIFIED_AUTOMATED`: Bluetooth-facing boundaries are mockable without hardware.
- `VERIFIED_AUTOMATED`: `GET /api/v1/health` and `GET /api/v1/status` pass API tests.
- `VERIFIED_AUTOMATED`: Rust formatting and Clippy with warnings denied pass across
  the workspace.
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
- `VERIFIED_HARDWARE`: the aggregate-only validator parsed the real iPhone PBAP
  output successfully: 438 contacts and 471 phone fields. No names or numbers
  were displayed, logged, or written by the validator.

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
- `VERIFIED_HARDWARE`: a bounded real-iPhone inbox listing passed through the
  aggregate-only validator with one row observed; no message metadata or content
  was emitted by the validator.

### Outbound messaging software

- `VERIFIED_AUTOMATED`: authenticated outbound requests are size-bounded and
  parsed only after authentication.
- `VERIFIED_AUTOMATED`: recipient/body validation, redacted diagnostics,
  aggregate-only API responses, and mock transport invocation pass tests.
- `VERIFIED_AUTOMATED`: the Android API-27 compose UI requires an explicit review
  dialog and clears the body only after transport acceptance.
- `DOCUMENTED`: installed `imsg 0.3.1` exposes recipient/body as CLI arguments;
  this can briefly expose them through same-machine process inspection.
- `VERIFIED_HARDWARE`: one deliberately confirmed Android-originated request was
  sent through iPhone MAP and received by the intended recipient. No private
  message fields were retained as evidence.
- `UNKNOWN`: sent-folder reflection, failure recovery, locked-iPhone behavior,
  MMS, and attachments.

### HFP call-control software

- `VERIFIED_AUTOMATED`: answer, reject, hangup, dial, DTMF, mute, and gain
  commands are validated against call state before reaching a backend.
- `VERIFIED_AUTOMATED`: failed backend commands preserve the prior call state.
- `VERIFIED_AUTOMATED`: dial targets and DTMF values are redacted from `Debug`
  output and backend errors contain no command payload.
- `VERIFIED_AUTOMATED`: validated commands encode to HFP AT operations; mute uses
  microphone gain zero and restores the last configured gain.
- `DOCUMENTED`: PipeWire 1.4.2 exposes an oFono-compatible Telephony D-Bus API
  with AudioGateway dial/hangup/DTMF and per-call answer/hangup methods.
- `VERIFIED_AUTOMATED`: the installed WirePlumber owns that service, and the
  adapter privately discovers numeric gateway/call paths without identity fields.
- `VERIFIED_AUTOMATED`: live state gating reads only the non-private call `State`
  property and blocks answer, dial, DTMF, or hangup in incompatible states.
- `VERIFIED_AUTOMATED`: authenticated status reads reduce the live WirePlumber
  gateway/call tree to aggregate HFP and call states; multiple calls use fixed
  precedence without exposing object paths.
- `VERIFIED_AUTOMATED`: authenticated, bounded call-command API requests and the
  Android API-27 controls build and pass mock-backed tests.
- `UNKNOWN`: real-iPhone acceptance and effects of commands sent through this seam.

### Audio bridge software

- `VERIFIED_AUTOMATED`: narrowband and wideband HFP PCM frames have explicit
  7.5 ms format invariants and reject mismatched payload sizes.
- `VERIFIED_AUTOMATED`: independent uplink/downlink queues are bounded and drop
  the oldest frame on overflow to prevent unbounded latency growth.
- `VERIFIED_AUTOMATED`: audio frame `Debug` output and the aggregate API contain
  no sample values.
- `VERIFIED_AUTOMATED`: queue depth, drop count, throughput, and maximum observed
  in-memory latency are tracked without recording audio.
- `VERIFIED_AUTOMATED`: strict versioned PCM packets round-trip, malformed packets
  fail closed, and the jitter buffer accounts for missing, late, duplicate, and
  overflow frames while bounding future latency.
- `VERIFIED_AUTOMATED`: Rust and Android API-27 packet codecs match a shared golden
  header vector without including sample values in diagnostics.
- `VERIFIED_AUTOMATED`: Android sequence handling and bounded jitter behavior match
  the Pi policy, including the shared signed-63-bit wire range.
- `VERIFIED_AUTOMATED`: both platforms expose explicit 7.5 ms playout ticks;
  pre-start polling is inert, while every empty post-start tick counts an underflow
  and advances late-packet rejection consistently.
- `VERIFIED_AUTOMATED`: privacy-safe PipeWire SCO discovery selects only official
  source/sink factory identifiers, retains the targetable numeric object serials,
  and the validator emits only aggregate presence.
- `VERIFIED_AUTOMATED`: runtime refresh maps exactly one SCO pair to `sco_active`,
  no nodes to `inactive`, and malformed or ambiguous snapshots to a fail-closed
  audio error without exposing the PipeWire snapshot.
- `VERIFIED_AUTOMATED`: managed `pw-cat` capture/playback commands bind the proper
  SCO directions for narrowband and wideband HFP, stream through anonymous pipes,
  and clean up partial starts and both child processes without logging stderr.
- `VERIFIED_AUTOMATED`: PipeWire PCM adapters reconstruct short reads into exact
  frames, preserve little-endian samples, reject truncated/mismatched frames, and
  can transfer the two directions to independent workers.
- `VERIFIED_AUTOMATED`: a transport-neutral framed-media bridge encodes downlink,
  strictly decodes and format-checks uplink, applies bounded jitter ordering, and
  reports aggregate state without committing to RTP or WebRTC.
- `VERIFIED_AUTOMATED`: an inactive-by-default Android audio-device adapter builds
  with voice-communication routing, exact HFP frame sizes, blocking PCM I/O, and
  optional acoustic echo/noise processing. Partial startup unwinds player,
  recorder, and routing state; stop/close are idempotent and suppress vendor error
  details.
- `UNKNOWN`: Android microphone permission and real-device audio initialization,
  routing, frame timing, and intelligibility.
- `UNKNOWN`: live-call PipeWire process operation, network transport latency, and
  intelligibility with real call audio.

### Control-plane security

- `VERIFIED_AUTOMATED`: all non-health API endpoints reject missing and incorrect
  bearer credentials.
- `VERIFIED_AUTOMATED`: token comparison is constant-time for equal-length inputs.
- `VERIFIED_AUTOMATED`: token `Debug` output is redacted and token length is bounded.
- `VERIFIED_AUTOMATED`: daemon startup fails when `ANALOGCONNECT_API_TOKEN` is absent.
- `DOCUMENTED`: the OpenAPI contract marks health public and every other endpoint protected.
- `VERIFIED_AUTOMATED`: plaintext daemon binds and Android HTTP endpoints are
  restricted to loopback; non-loopback transport must use TLS/HTTPS.
- `VERIFIED_AUTOMATED`: Android certificate-pin parsing, constant-time SHA-256
  matching, redaction, and pinned TLS trust-manager packaging pass API-27 checks.
- `VERIFIED_AUTOMATED`: authenticated SMS/HFP mutations are rate-limited and emit
  only fixed payload-free acceptance audit events.
- `VERIFIED_AUTOMATED`: staged current/previous bearer-token rotation supports
  Android migration and explicit old-token revocation on daemon restart.
- `VERIFIED_AUTOMATED`: per-call media grants use separate OS-random credentials,
  opaque session IDs, strict constant-time authorization, a five-minute maximum
  lifetime, monotonic expiry, immediate revocation, and redacted diagnostics.
- `VERIFIED_AUTOMATED`: the media registry enforces one current session and one
  connected client, revokes replacement/teardown, and allows bounded reconnects
  only after the prior connection lease is released.
- `VERIFIED_AUTOMATED`: Android API-27 strictly validates the matching transient
  media credential shape and lifetime, uses monotonic expiry, and redacts all
  diagnostic output.
- `VERIFIED_AUTOMATED`: an authenticated, mutation-limited media-session endpoint
  issues a one-minute registry credential only while call and SCO states are both
  active; the OpenAPI contract fixes its three-field response and mandatory
  `no-store`/`no-cache` headers.
- `VERIFIED_AUTOMATED`: status and media issuance refresh HFP, call, and SCO from
  read-only WirePlumber/PipeWire snapshots, so production issuance no longer
  depends on static startup state. Expected Telephony absence maps to
  `disconnected`/`idle`; malformed or ambiguous state fails closed.
- `VERIFIED_AUTOMATED`: `busctl` and `pw-dump` snapshot helpers have a two-second
  wall-time bound and respective 1 MiB/16 MiB output bounds, drain stdout privately,
  discard stderr, and kill/reap stalled children with fixed payload-free errors.
- `VERIFIED_AUTOMATED`: a production-binary loopback smoke test exercised the
  installed WirePlumber/PipeWire observers while idle and returned HFP
  `disconnected`, call `idle`, and audio `inactive`, then shut down cleanly.
- `VERIFIED_AUTOMATED`: Android compiles the matching bounded issuance request and
  validates the response into an in-memory-only monotonic credential object.
- `VERIFIED_AUTOMATED`: the Android client compiles token-at-rest protection using
  Android Keystore AES/GCM and does not log tokens or response bodies.
- `VERIFIED_HARDWARE`: manual enrollment persistence, authenticated daemon access,
  staged short-token testing, and certificate-pin UI behavior work on the real
  Android 8.1 phone through the development ADB tunnel.
- `UNKNOWN`: one-time enrollment issuance, automatic expiration, real LAN TLS,
  and hardware-backed Keystore availability on this phone.

### Android client

- `VERIFIED_AUTOMATED`: a dependency-free API-27 client compiles, DEXes, packages,
  aligns, signs, and passes APK signature verification on the Raspberry Pi.
- `VERIFIED_AUTOMATED`: endpoint validation rejects embedded credentials and
  unsupported URL schemes.
- `VERIFIED_HARDWARE`: the signed APK installs on the real Android 8.1/API 27
  phone, `MainActivity` launches successfully, and its process remains running.
- `VERIFIED_HARDWARE`: enrollment, hidden-token visibility toggle, health check,
  authenticated status, and confirmed outbound-message UI behavior work on the
  real phone.
- `VERIFIED_HARDWARE`: phone-to-Pi control requests work through an ADB reverse
  loopback tunnel. Direct LAN transport remains disabled until TLS is implemented.

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
- `VERIFIED_HARDWARE`: Android-to-Pi-to-iPhone MAP SMS sending and recipient
  delivery work for a deliberate test message.
- `UNKNOWN`: MAP delivery-state notifications, MMS, attachments, sent-folder
  reflection, and locked-iPhone behavior.
- `UNKNOWN`: automatic recovery after reboot, Bluetooth loss, or network loss.
- `UNKNOWN`: real-device UI usability, Keystore behavior, connectivity, and
  end-to-end behavior.

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

Milestone 5: hardware-verify Pi-originated HFP call control, beginning with the
lowest-risk active-call hangup path. Do not deploy the daemon as a system service yet.

## End-to-end roadmap

1. Backend state model and API skeleton.
2. PBAP contact synchronization and caller matching.
3. MAP incoming synchronization and notification/polling fallback.
4. Hardware-verified outgoing MAP messages.
5. Hardware-verified HFP call-control commands.
6. Pi SCO audio bridge and transport benchmarks.
7. Android API-27 control-plane application (foundation installs and launches;
   UI, secure enrollment, and transport validation pending).
8. Android call audio.
9. Full integration, recovery, security, deployment, and maintenance.
