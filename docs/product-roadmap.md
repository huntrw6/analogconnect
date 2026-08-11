# AnalogConnect product roadmap

## Product outcome

The finished product lets the Android 8.1 slider serve as a dependable companion
interface while the iPhone remains the cellular endpoint. A user can discover and
enroll the Pi, browse contacts and conversations, send and receive messages,
place and receive calls, hear and speak clearly, recover from ordinary disconnects,
and understand failures without operating the Pi.

## Current baseline

- `VERIFIED_HARDWARE`: iPhone MAP, PBAP, HFP call control, and intelligible
  two-way call audio have each worked through the bridge.
- `VERIFIED_HARDWARE`: Android discovery, pinned TLS, bearer authentication,
  earpiece/speaker routing, and one prior outbound SMS delivery have worked.
- `VERIFIED_HARDWARE`: Android-originated SMS delivery works after correcting the
  daemon/imsg writable-path regression.
- `VERIFIED_AUTOMATED`: the repository contains mockable Rust profile boundaries,
  authenticated REST/WebSocket transports, bounded audio, Android API-27 codecs,
  an opt-in Telecom foundation, and an inactive Contacts sync foundation.
- `DOCUMENTED`: the present Android interface is a 749-line programmatic
  engineering activity rather than a conversation/call product interface.
- `UNKNOWN`: reliable incoming-message sync, notifications, incoming Telecom
  calls, native contact publication, reboot recovery, and release installation.

## Definition of done

The project is product-complete when all of the following are true:

1. `VERIFIED_HARDWARE`: calls can be placed, received, answered, rejected, and
   ended from Android with clear earpiece and speakerphone audio, DTMF, mute, and
   bounded latency over repeated and sustained calls.
2. `VERIFIED_HARDWARE`: conversations load, new SMS messages appear, notifications
   arrive, sends show honest queued/sent/failed state, retries do not duplicate,
   and reconnect/reboot cases recover.
3. `VERIFIED_HARDWARE`: iPhone contacts appear with explicit user consent, remain
   attributable to the AnalogBridge account, support calling/messaging, and are
   removed cleanly on disconnect/account removal.
4. `VERIFIED_HARDWARE`: enrollment survives ordinary restarts; loss of Wi-Fi,
   Bluetooth, Pi power, app process, or iPhone availability produces a clear state
   and recovers without re-pairing in the normal case.
5. `VERIFIED_AUTOMATED`: privacy boundaries, authentication, TLS pinning, input
   limits, redaction, state machines, codecs, persistence, migration, and recovery
   paths have regression tests and all supported builds/lints pass.
6. `VERIFIED_HARDWARE`: a release-signed, reproducible APK and Pi installer can be
   installed from clean supported devices using documented setup and rollback.

MMS, attachments, group-message semantics, multiple iPhones, and app-store-style
distribution are outside the first finished-product definition unless later
promoted.

## Execution phases

### Phase 1 — Restore the reliable vertical slice

Goal: make today's calling and outbound messaging paths trustworthy before adding
more UI surface.

- Preserve the hardware-verified SMS regression correction and its readiness test.
- Add privacy-safe backend error classes and Android status mapping.
- Preserve optional operation-ID duplicate suppression for v1 clients while v2
  gains durable outbox/reconciliation semantics.
- Make the minimum service-sandbox correction and cover it with an automated
  policy check.
- Preserve compose text on failure and prevent accidental duplicate submissions.
- Exercise repeat calls, clean teardown, media-session renewal, and five-minute
  latency behavior.
- Validate cold Pi boot, Android rediscovery, and iPhone profile reconnection.

Exit: outbound messaging and calls pass the specified reboot/reconnect hardware
matrix; no known P0 loss, duplication, stuck audio, or silent-failure issue remains.

### Phase 2 — Product data contracts

Goal: expose the bounded data needed by a real UI without weakening privacy.

- Define API v2 resources for contact pages/search, conversation summaries,
  paginated message history, send-operation IDs/status, and capability state.
- Keep private values out of URLs, logs, errors, debug formatting, metrics, and
  repository fixtures; use synthetic fixtures for tests.
- Integrate the existing contact store and imsg encrypted message store behind
  narrow repository interfaces rather than parsing display-oriented CLI output.
- Add polling-first synchronization with notification acceleration, explicit
  cursors, idempotency, and bounded backoff.
- Provide a single authenticated event stream for aggregate connection state,
  call transitions, conversation invalidation, and send-status changes; refetch
  private data over bounded HTTPS requests.

Exit: contract tests demonstrate pagination, ordering, idempotency, migration,
redaction, and reconnect behavior with synthetic data.

Current evidence: `VERIFIED_AUTOMATED` for bounded conversation/history contracts,
opaque IDs/cursors, authenticated no-store routes, synthetic encrypted-store reads,
v1 operation-ID idempotency, and polling-driven inbox/sent synchronization.
`VERIFIED_HARDWARE`: the real encrypted store opens through v2; deployed automatic
sync populated five conversations and an aggregate-only history check verified a
correctly ordered five-message page. Durable send state, full cursor UI,
migration, and reconnect behavior remain.

### Phase 3 — Interactive Android application

Goal: replace the engineering panel with a keyboard-friendly companion UI.

- Split enrollment/settings, conversations, compose, contacts, and active-call
  responsibilities out of `MainActivity` into testable controllers and screens.
- Build a conversation list, thread history, compose/review flow, delivery state,
  retry controls, unread state, empty/loading/offline/error states, and search.
- Build contact browsing/search and contact actions using the API first; enable
  the prepared native Contacts account only after cleanup semantics are proven.
- Build incoming/outgoing/active call surfaces with caller match, answer/reject,
  hangup, DTMF keypad, mute, speaker, duration, reconnect, and visible audio state.
- Optimize focus order, D-pad/physical-keyboard operation, touch targets, contrast,
  large text, and screen-reader labels for the slider-phone form factor.

Exit: automated controller tests and a hardware walkthrough cover every normal,
empty, loading, offline, error, and retry state without requiring Pi terminal use.

Current evidence: `VERIFIED_AUTOMATED` for the first API-27 conversation list,
thread, compose/review/send, unread/status, empty/loading/fixed-error, and retry
slice. `VERIFIED_HARDWARE`: the signed APK is installed. Physical-phone layout,
focus, populated content correctness, send refresh, and pagination remain. A
participant-set group-thread correction is `VERIFIED_AUTOMATED`, but
`VERIFIED_HARDWARE` checks found that the iPhone supplies neither complete
participants, message-list conversation IDs, nor conversation-list entries.
Exact group reconstruction is therefore `BLOCKED`; the UI keeps group sending
fail-closed and the first-product definition continues to exclude group semantics.
The dedicated state-driven call screen is also `VERIFIED_AUTOMATED` and installed;
its physical focus, live state transitions, automatic audio, speaker route, DTMF,
and teardown are the current operator gate.
The authenticated contact list/search and conversation-name resolution are
`VERIFIED_AUTOMATED`; real PBAP sync and aggregate name matching are
`VERIFIED_HARDWARE`, and the installed Android presentation is ready for review.

### Phase 4 — Background behavior and Android integration

Goal: make the companion useful when its main screen is not open.

- Add a foreground/background connection service appropriate for Android 8.1,
  with bounded wake/network behavior and explicit persistent status when needed.
- Add privacy-conscious incoming-message and incoming-call notifications with
  user-configurable preview behavior.
- Complete incoming Telecom connections and hardware-test the vendor dialer;
  retain the in-app call UI as a fallback.
- Enable the Contacts sync account with permission, atomic replacement,
  account-owned rows, deduplication rules, and removal cleanup.
- Verify ordinary cellular and emergency calling remain unaffected.

Exit: calls and message notifications arrive from a backgrounded/killed UI, and
all native integrations can be disabled or removed cleanly.

### Phase 5 — Resilience, security, and observability

Goal: recover automatically and make remaining failures diagnosable without
collecting private content.

- Implement explicit state machines for Wi-Fi, TLS identity, authentication,
  Bluetooth profiles, MAP broker, HFP/SCO, synchronization, and media transport.
- Add bounded retries, jitter, circuit breaking, idempotent commands, stale-session
  cleanup, and recovery after either device restarts.
- Expose a user-facing connection dashboard and a privacy-safe support bundle made
  only of versions, states, counters, fixed error codes, and timestamps.
- Threat-model enrollment, LAN attackers, Android storage, Pi filesystem access,
  subprocess argument exposure, notification previews, database retention, and
  certificate/token rotation.
- Replace private CLI arguments or document/mitigate the single-user-host
  limitation before broad deployment.

Exit: fault-injection tests and the hardware recovery matrix pass without private
logs, unbounded retry loops, duplicate sends, or wedged calls.

### Phase 6 — Release and maintenance

Goal: make installation and upgrades repeatable for a non-developer operator.

- Pin supported Pi OS, BlueZ, PipeWire/WirePlumber, Rust build, Android API/build
  tools, and imsg versions; detect incompatible versions before deployment.
- Create idempotent install, upgrade, health-check, backup, uninstall, and rollback
  procedures, each respecting the system-change approval rules during development.
- Establish release signing and secret handling outside Git, version both API and
  stored data, and test forward/rollback migrations.
- Run the complete automated suite plus a privacy-safe release hardware checklist.
- Maintain a limitations page and triage policy for upstream iOS, Android vendor,
  BlueZ, PipeWire, and imsg changes.

Exit: a clean Pi and target Android phone can be set up, upgraded, tested, and
rolled back from the documentation without source-level intervention.

## Hardware gates requiring the operator

Development should proceed autonomously until one of these observations is needed:

- the fixed Android result category from a deliberate SMS attempt;
- recipient delivery, incoming message arrival, or notification timing;
- subjective call clarity/delay and DTMF audibility;
- physical iPhone permission, lock-state, Bluetooth, or call interaction;
- target vendor dialer/Contacts behavior and emergency-call isolation;
- reboot, range-loss, power-loss, and long-idle recovery behavior.

Every request for a hardware test must specify setup, exact actions, expected
aggregate observations, privacy exclusions, stop conditions, and how the result
changes the next engineering decision.

## Autonomous work order

Until a hardware gate blocks progress, work in this order:

1. Finish Phase 1 automated diagnostics and the proposed minimal sandbox patch.
2. Design and test the API v2 domain/contracts with synthetic data.
3. Refactor Android networking/state from the activity and implement the
   conversation UI against fakes.
4. Implement backend contact/conversation read models and synchronization.
5. Implement the interactive call UI and background event/state layer.
6. Complete native Contacts/Telecom integrations behind fail-closed switches.
7. Build fault-injection, installation, upgrade, and release automation.

Do not advance a later phase in a way that hides or works around a known Phase 1
reliability failure.
