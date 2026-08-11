# AnalogConnect Current State

This document is authoritative for current project status. Earlier phase documents
remain as investigation history; where they disagree with this document, their
interpretations are superseded.

## Current implementation

Status updated 2026-08-11 after the physical call-control safety checkpoint.

## Android product UI checkpoint

- `VERIFIED_AUTOMATED`: the API-27 application now launches into a user-facing
  communication shell with Messages, Calls, Contacts, and Settings navigation.
  The previous diagnostic launcher remains intact under Settings → Developer
  Tools and no longer dominates the normal journey.
- `VERIFIED_AUTOMATED`: conversation rows render human titles, group avatars,
  timestamps, and unread emphasis; threads render conventional left/right
  bubbles and sender labels for group messages. Stable `ancs-v1-*` identifiers
  remain model-only and are never used as visible titles.
- `VERIFIED_AUTOMATED`: direct conversations and the dedicated new-message flow
  can send through the real authenticated API. Group and ambiguous composers
  fail closed with user-facing explanations.
- `VERIFIED_AUTOMATED`: once ringing or dialing begins, the call screen has no
  touch-activated call controls. The device's dedicated green Call key answers,
  the red End key rejects/cancels/ends according to authoritative call state,
  and physical digits/`*`/`#` send DTMF only while active. The target red key is
  combined End/Power; a permission-protected raw-key monitor on the connected Pi
  distinguishes short End from held native Power. Hardware capture verified both
  paths without a system keylayout modification. Repeats, key-up, idle
  End, and terminal states fail closed. A proximity wake lock protects the
  screen without being required for control. Proximity blanking remains pending
  hardware validation.
- `VERIFIED_HARDWARE`: on the target phone, a short red End/Power press rejected
  a real incoming iPhone call and the same key ended a real active call; green
  answered once, bidirectional audio was human-confirmed clear, screen touches
  produced no call command, and held Power retained the native power menu. HFP
  stayed `slc_ready` after teardown. Physical DTMF was captured but its first
  real press had two possible software paths; the Pi path is now removed and a
  repeat produced exactly one caller-heard tone. A persistent Android foreground
  watcher now surfaces real incoming calls from other screens, and immersive mode
  removes status/navigation affordances while live. Proximity blanking was also
  confirmed on the physical phone.
- `VERIFIED_AUTOMATED`: an explicitly enabled offline demo source exercises
  direct, named group, unnamed group, unread, outgoing failure, ambiguity,
  multiple-sender, contact-search, and empty-search states. It is in-memory and
  cannot write to the encrypted production store.
- `BLOCKED`: an emulator remains unavailable; this aarch64 Pi has no installed
  emulator binary or AVD. Physical API-27 testing is used instead.
- `VERIFIED_AUTOMATED`: the signed APK was upgraded on the physical API-27 phone
  without clearing data. Offline demo walkthroughs validated onboarding, fixed
  navigation, direct/group threads, a fixed composer, group fail-closed wording,
  dark appearance, 1.5× font scaling, dialer recovery, notification channels,
  group notification content, and notification deep linking. These are physical
  layout checks with synthetic data, not iPhone hardware evidence.
- `VERIFIED_AUTOMATED`: conversation summaries now include bounded latest-message
  previews and group sender attribution. Android retains last-known-good
  conversations, messages, and contacts in an AES-GCM Android Keystore cache and
  labels cached views as offline; send routing never consults the cache.
- `VERIFIED_AUTOMATED`: API-27 Messages and Incoming Calls notification channels
  exist. Synthetic group notification content and deep linking to the correct
  human-titled group thread passed ADB/UIAutomator inspection.
- `VERIFIED_AUTOMATED`: the existing foreground connection service now polls
  authenticated conversation summaries at a bounded interval, refreshes the
  encrypted cache, suppresses historical/duplicate/sent notifications, and uses
  stable per-thread notification IDs. Group previews retain sender attribution.
- `VERIFIED_AUTOMATED`: explicit Light, Dark, and Follow device preferences apply
  to all user and developer Activities; light/dark message bubbles passed physical
  screenshot inspection.

- Milestone 0A baseline and Milestone 1 backend skeleton are complete.
- Milestone 2 contact parsing is validated against the real iPhone using an
  aggregate-only pipeline; persistence/search remain hardware-free validated.
- Milestone 3 MAP synchronization orchestration is implemented around imsg's
  encrypted store and awaits notification behavior validation with the iPhone.
- Milestone 4 outbound MAP messaging has a hardware-verified earlier success, but
  the latest Android-originated retest failed and the path is not currently
  considered reliable.
- Milestone 5 HFP call-control domain, AT encoding, WirePlumber Telephony D-Bus
  adapter, authenticated API, and Android controls are implemented; live command
  effects on the iPhone still require guided validation.
- Milestone 6 in-memory audio bridge queues and aggregate instrumentation are
  implemented alongside a framed-PCM diagnostic codec, bounded jitter buffer,
  PipeWire SCO discovery, managed `pw-cat` binding, exact PCM frame adapters,
  authenticated WebSocket transport, and Android microphone/earpiece pump.
- The server-side Android control plane now requires bearer authentication for all
  non-health endpoints and refuses startup without an explicit token.
- An API-27 Android client foundation builds as a signed debug APK on the ARM64
  Raspberry Pi and installs and launches on the target Android 8.1 phone.
- Android system integration follows a hybrid design: native Contacts through a
  future sync account, an inactive fail-closed Telecom call-provider experiment,
  and a dedicated AnalogBridge conversation UI for bridged messages.

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

- `VERIFIED_HARDWARE`: the deployed periodic PBAP refresh loaded 438 contacts and
  471 phone fields. The private API returned 100 named first-page items with more
  available, and uniquely matched names for all seven current conversation rows;
  aggregate validation emitted no names or numbers.
- `VERIFIED_AUTOMATED`: `POST /api/v2/contacts/search` is authenticated, bounded,
  paginated, body-based, no-store, and redacts diagnostics. Conversation summaries
  preserve the phone-number routing target separately from the nullable display
  name, preventing accidental sends to labels.
- `VERIFIED_AUTOMATED`: Android provides **Open contacts**, name search, incremental
  loading, contact-to-call actions, name-first conversation rendering, unknown
  number fallback, and explicit context-unknown/private-message wording.
- `VERIFIED_HARDWARE`: the matching daemon and signed APK are installed.
- `UNKNOWN`: physical contact-list focus/layout and the correctness of displayed
  private names await operator inspection.

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
- `VERIFIED_AUTOMATED`: the production daemon now starts a polling coordinator
  immediately and every 30 seconds, synchronizes exactly the inbox and sent
  folders, excludes deleted messages, and reports only aggregate/redacted state.
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
- `FAILED`: the latest Android-originated outbound SMS retest did not work. No
  recipient, body, or other private message data was retained.
- `DOCUMENTED`: the installed user service runs with `ProtectHome=read-only`,
  while installed `imsg 0.3.1` opens its encrypted SQLCipher message database for
  every `send`; the store source requires a writable path. Its broker may also
  write under the per-user state directory.
- `INFERRED`: the service sandbox/store mismatch is the leading regression cause
  because the hardened service was added after the earlier successful messaging
  implementation, while calls and audio can continue without these `imsg` writes.
- `VERIFIED_AUTOMATED`: the tracked and installed user service now retain
  `ProtectHome=read-only` while granting write access only to imsg's per-user data
  and state directories; the daemon restarted and remained active.
- `VERIFIED_HARDWARE`: the operator confirmed that Android-originated texting
  works correctly again after the minimal sandbox correction. This verifies the
  service sandbox/store mismatch as the cause of the observed regression.
- `VERIFIED_AUTOMATED`: optional 128-bit operation IDs now provide bounded
  daemon-lifetime duplicate suppression; accepted duplicates do not invoke MAP a
  second time, simultaneous duplicates fail closed, and identifiers redact from
  diagnostics.
- `VERIFIED_AUTOMATED`: new Android builds generate operation IDs with
  `SecureRandom`, reuse the ID for an unchanged failed draft, disable sending while
  a request is active, preserve failed compose text, and remain compatible with
  the existing v1 endpoint.
- `DOCUMENTED`: duplicate suppression does not yet survive daemon/app restart;
  durable outbox status and reconciliation are part of the v2 messaging contract.
- `VERIFIED_AUTOMATED`: API v2 conversation summaries and message history use
  authenticated bounded pages, daemon-scoped opaque conversation IDs, canonical
  cursors, private request bodies, no-store/no-cache responses, fixed unavailable
  and expired states, and redacted domain diagnostics.
- `VERIFIED_AUTOMATED`: a typed lazy adapter opens a synthetic imsg SQLCipher
  store through the same query types used in production and maps concurrent
  thread/history reads without parsing CLI output or exposing MAP handles.
- `VERIFIED_AUTOMATED`: the API-27 Android conversation models/controllers and
  signed APK build cover loading, empty, list, thread, fixed-error, retry,
  compose/review/send, unread, and outgoing-state surfaces without logging private
  data.
- `VERIFIED_HARDWARE`: the deployed v2 daemon opened the real encrypted imsg store
  and served the aggregate-only conversation endpoint successfully. Before
  automatic sync the store was empty (`count=0`, `has_more=false`).
- `VERIFIED_HARDWARE`: the automatic inbox/sent synchronization daemon is
  installed and active. Its first real-iPhone checks reported two successful
  synchronizations, zero failures, five conversation summaries, and no next page.
- `VERIFIED_HARDWARE`: an aggregate-only history check loaded five messages from
  one real conversation in correct newest-first order with no next page; no
  address, body, identifier, or timestamp was emitted as evidence.
- `VERIFIED_HARDWARE`: the signed conversation APK and latest v2 daemon are
  installed on the physical devices.
- `VERIFIED_HARDWARE`: the operator observed five populated rows and plausible
  message content on the physical Android conversation screen.
- `FAILED`: group messages are incorrectly split into per-phone-number rows. The
  root cause is imsg 0.3.1 collapsing each MAP message to one peer address before
  storing and aggregating it.
- `DOCUMENTED`: MAP bMessages and the installed parser support an originator plus
  multiple recipient vCards, but imsg 0.3.1 does not persist that participant set;
  upstream 0.4.0 retains the same per-address thread model.
- `VERIFIED_AUTOMATED`: the patched imsg ingestion preserves sorted/deduplicated
  participant sets, retains peer keys for direct/outgoing rows, groups different
  senders under one synthetic group key, refreshes metadata on existing handles,
  and resets sync cursors once without deleting message content.
- `VERIFIED_AUTOMATED`: API v2 marks group threads and exposes private sender labels;
  the Android UI groups them, labels received messages by sender, and fails closed
  on group replies until multi-recipient MAP push is implemented.
- `INFERRED`: the participant-set key will match the iPhone's actual group identity
  if its retrieved bMessages contain the complete originator/recipient vCard set.
- `VERIFIED_HARDWARE`: the first participant-set deployment synchronized once with
  zero failures but returned seven rows and zero detected groups. This demonstrates
  that the iPhone payload does not provide enough vCard participants per message
  for cardinality-only grouping; no unsafe group reply was enabled.
- `DOCUMENTED`: the MAP message-listing format carries a separate opaque
  `conversation_id`; imsg 0.3.1 ignores unknown listing attributes, including that
  identity. The next correction preserves it and filters UI reads to inbox/sent.
- `INFERRED`: grouping by the iPhone-provided MAP conversation identifier will
  preserve the real group boundary without content/timestamp heuristics.
- `VERIFIED_HARDWARE`: SDP advertises one MAP MAS record with message-listing v1.1
  support and no conversation-version-counter feature. Before deploying the
  explicit parameter-mask build, the privacy-safe validator observed seven
  threads, zero stored MAP conversation identities, and zero groups.
- `VERIFIED_HARDWARE`: after explicitly requesting every message-list attribute,
  a complete sync still stored zero MAP conversation identities. The separate
  MAP `x-bt/MAP-convo-listing` request was accepted but returned zero
  conversations while the message store remained non-empty.
- `DOCUMENTED`: Bluetooth MAP defines conversation listing as a distinct GET
  operation with a conversation parameter mask and nested participant elements.
  The implementation and aggregate-only probe follow that wire format.
- `VERIFIED_HARDWARE`: the sole iPhone MAS record is MAP 1.4, MAS instance 0,
  SMS_GSM, with raw `MapSupportedFeatures=0x0006027f`. It advertises
  Messages-Listing v1.1 but not Conversation Listing, Event Report v1.2,
  Conversation Version Counters, or MapSupportedFeatures-in-CONNECT. Therefore
  omitting application parameter `0x29` from CONNECT is correct for this server.
- `VERIFIED_HARDWARE`: a complete message-listing request returned OBEX `0xa0`,
  `ListingSize=10`, and no `conversation_id`, `conversation_name`, `direction`, or
  participant attributes. Conversation Listing returned OBEX `0xa0` with absent
  `ListingSize`, an empty body, and no database/version identifiers.
- `VERIFIED_HARDWARE`: a MAP 1.4 MNS listener captured controlled group and direct
  `NewMessage` events. Both contained only `type`, `handle`, `folder`, and
  `msg_type`; neither exposed conversation or participant metadata.
- `VERIFIED_HARDWARE`: direct-GATT ANCS established an LE bearer and subscribed to
  Notification Source and Data Source without changing pairing. Two controlled
  group notifications had non-empty Title and Subtitle fields; a clean controlled
  direct notification (and its duplicate update) had a non-empty Title and empty
  Subtitle. This verifies stable group detection for the tested Messages setup,
  but the privacy-safe structural capture does not yet prove a stable group name,
  participant set, or reply target.
- `VERIFIED_HARDWARE`: `ANCS_GROUP_DETECTION_ONLY`. MAP and ANCS do not currently
  provide a verified group reply target, so group replies remain disabled and
  direct threads remain supported.
- `VERIFIED_HARDWARE`: `ANCS_STABLE_GROUP_IDENTITY_VERIFIED`. With one ephemeral
  HMAC key, two different senders in Group A produced the same normalized
  Subtitle HMAC, Group B produced a different HMAC, and a direct message had no
  Subtitle. This is a privacy-safe local incoming/history key, not a reply target.
- `VERIFIED_HARDWARE`: `ANCS_GROUP_THREADING_VERIFIED`. Plaintext controlled
  inspection established that ANCS Title is the current sender, named-group
  Subtitle is the exact Messages group name, unnamed-group Subtitle is Apple's
  participant-generated label, and direct Subtitle is empty. Different senders in
  the tested named group retained the same Subtitle.
- `VERIFIED_AUTOMATED`: `normalize_ancs_subtitle_v1` applies Unicode NFKC, Unicode
  case folding, whitespace collapse, and trimming. Non-empty Subtitles produce a
  deterministic full-SHA256 `ancs-v1-…` ID with an explicit domain/version prefix;
  empty Subtitles produce no group ID.
- `VERIFIED_AUTOMATED`: the encrypted imsg migration persists ANCS group metadata,
  display Subtitle, observed senders, NotificationUID assignment, first/last seen,
  conflict state, and future alias/split records separately from immutable message
  row IDs. Close/reopen tests preserve grouping and titles; conflicting assignment
  evidence fails closed without rewriting the original assignment.
- `VERIFIED_AUTOMATED`: a bounded ANCS/MAP correlation boundary matches normalized
  Title to the resolved MAP sender plus a bounded time window, handles duplicate
  UIDs and out-of-order arrival, rejects stale/competing candidates as ambiguous,
  and atomically applies only proven groups to the encrypted store.
- `VERIFIED_AUTOMATED`: no-match correlation remains pending inside the bounded
  window, while genuinely competing evidence is persisted as a fail-closed
  per-message ambiguity marker. Conversation APIs disable reply until unique
  direct or group evidence clears that marker, including across daemon restart.
- `VERIFIED_AUTOMATED`: API v2 exposes exact stable `ancs-v1-…` IDs, ANCS Subtitle
  titles, explicit private/group/ambiguous kind, and disabled reply state for groups
  and conflicts. Android accepts those IDs, renders the API title, merges assigned
  senders through the shared conversation key, and cannot enter its private reply
  flow for group or ambiguous rows.
- `VERIFIED_AUTOMATED`: analogconnectd now starts a supervised BlueZ D-Bus/GATT
  bearer, discovers the ANCS service and three characteristics, subscribes to both
  sources in order, writes bounded Control Point requests, and feeds metadata into
  immediate MAP sync plus the encrypted-store correlation boundary. It never
  disconnects the device, so ANCS retry does not intentionally tear down Classic.
  The temporary direct-GATT script remains diagnostic only.
- `VERIFIED_AUTOMATED`: the production ANCS protocol core strictly parses
  Notification Source events, requests AppIdentifier/Title/Subtitle/size/date and
  action labels without requesting Message body, reassembles bounded Data Source
  fragments, serializes one Control Point request at a time, filters non-Messages
  apps, suppresses duplicate/replayed UIDs, bounds queued/completed state, and
  provides a subscription-ordering/reconnect supervisor with capped backoff. Live
  BlueZ/Classic coexistence and end-to-end delivery remain hardware-pending.
- `UNKNOWN`: unnamed-group Subtitle stability across different senders and a
  diagnostic restart awaits `ANCS-UNNAMED-GROUP-IDENTITY-001`.
- `NOT VERIFIED`: group reply targeting remains unavailable and disabled.
- `BLOCKED`: Apple's iOS 26.5 Accessory Notifications API formally exposes
  `threadIdentifier`, notification/source identifiers, text-input actions, and
  typed `NotificationResponse` values, but this Linux/Pi environment has no Mac,
  Xcode, or iOS 26.5 SDK. Messages-specific field/action behavior and Personal
  Team entitlement eligibility cannot be tested here.
- `UNKNOWN`: sent-folder reflection, failure recovery, locked-iPhone behavior,
  MMS, and attachments.

### HFP call-control software

- `VERIFIED_AUTOMATED`: the Android client has a dedicated state-driven call
  screen with idle/dialing/incoming/active/ended/error surfaces, answer/reject/end
  eligibility, a DTMF keypad, elapsed duration, speaker routing, fixed errors, and
  automatic call-audio lifecycle. Pure controller tests cover state/control and
  DTMF rules; the API-27 APK build and v2/v3 signatures pass.
- `VERIFIED_HARDWARE`: that APK is installed on the target Android without
  clearing enrollment or app data.
- `UNKNOWN`: physical focus/layout, real screen state transitions, automatic
  microphone/earpiece audio, speaker switching, DTMF, and teardown through this
  new screen await the focused operator walkthrough.

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
  the shared signed-63-bit wire range and resynchronize toward recent audio if a
  queue reaches its ceiling.
- `VERIFIED_AUTOMATED`: Android holds an expected frame during an empty post-start
  queue, conceals it without exposing samples, crossfades recovery, and applies
  aggregate depth feedback plus bounded crossfaded latency trimming.
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
- `VERIFIED_AUTOMATED`: short-lived, single-connection media credentials protect a
  bounded WebSocket endpoint; a real loopback upgrade rejects a duplicate claim
  and transfers ACAP packets through both shared queues.
- `VERIFIED_AUTOMATED`: authenticated streams own live `pw-cat` capture/playback
  workers, drain uplink at 7.5 ms with silence underflow, frame downlink, and tear
  down processes before joining workers.
- `VERIFIED_AUTOMATED`: the Android pinned-TLS WebSocket wire layer validates the
  RFC 6455 handshake, masks client frames, rejects extensions/fragmentation and
  malformed frames, and feeds a synchronized jitter-buffer audio pump through
  fake device/network boundaries.
- `DOCUMENTED`: Android start/stop controls request runtime microphone permission,
  obtain a fresh media grant, start off the UI thread, monitor fixed pump failure
  codes, cancel stale starts, and stop audio whenever the activity leaves the
  foreground.
- `VERIFIED_HARDWARE`: Android microphone capture, earpiece routing, speakerphone
  routing, live PipeWire operation, authenticated media transport, and intelligible
  audio in both directions work during real iPhone calls.
- `VERIFIED_HARDWARE`: on the latest operator test, a real phone call sounded good
  on both ends. This supersedes the older statement that human-perceived audio
  quality was still unknown, but does not yet prove long-duration stability.
- `VERIFIED_HARDWARE`: Pi downlink delivery showed zero drops and at most about
  16 ms in-memory queueing during a sustained call; Android reported zero late and
  overflow frames after adaptive resynchronization.
- `INFERRED`: remaining periodic buffer growth is capture/playback clock mismatch,
  not weak Wi-Fi. The bounded latency trimmer awaits sustained hardware testing.

### Control-plane security

- `VERIFIED_AUTOMATED`: all non-health API endpoints reject missing and incorrect
  bearer credentials.
- `VERIFIED_AUTOMATED`: token comparison is constant-time for equal-length inputs.
- `VERIFIED_AUTOMATED`: token `Debug` output is redacted and token length is bounded.
- `VERIFIED_AUTOMATED`: daemon startup fails when `ANALOGCONNECT_API_TOKEN` is absent.
- `DOCUMENTED`: the OpenAPI contract marks health public and every other endpoint protected.
- `VERIFIED_AUTOMATED`: plaintext daemon binds and Android HTTP endpoints are
  restricted to loopback; non-loopback transport must use TLS/HTTPS.
- `VERIFIED_AUTOMATED`: complete certificate/key configuration selects an
  HTTPS-only listener that may bind an explicit LAN address; partial configuration,
  invalid PEM data, and group/other-readable Unix private keys fail closed.
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
- `VERIFIED_AUTOMATED`: live HFP discovery follows WirePlumber 0.5's actual API:
  root ObjectManager gateway keys, gateway `GetCalls` call keys, oFono-compatible
  `State`, and PipeWire command methods. Private property values are discarded.
- `VERIFIED_AUTOMATED`: audio is active only when both a unique SCO node pair and
  the gateway transport report active.
- `VERIFIED_AUTOMATED`: an idle/ended call with persistent active SCO transitions
  through a ten-second `sco_tearing_down` grace period, then fails closed with a
  fixed redacted `sco_teardown_stalled` diagnostic and blocks media issuance.
- `VERIFIED_AUTOMATED`: the explicit stuck-SCO recovery script requires a unique
  gateway, zero calls, active transport, existing pairing, and the HFP AG UUID;
  helper calls are bounded and device identifiers never reach output.
- `VERIFIED_AUTOMATED`: `busctl` and `pw-dump` snapshot helpers have a two-second
  wall-time bound and respective 1 MiB/16 MiB output bounds, drain stdout privately,
  discard stderr, and kill/reap stalled children with fixed payload-free errors.
- `VERIFIED_AUTOMATED`: a production-binary loopback smoke test exercised the
  installed WirePlumber/PipeWire observers while idle and returned HFP
  `disconnected`, call `idle`, and audio `inactive`, then shut down cleanly.
- `VERIFIED_AUTOMATED`: the installed daemon is enabled in the lingering user boot
  target; Bluetooth, PipeWire, WirePlumber, and Avahi are enabled; its private
  configuration persists with mode `0600`; and its pinned-TLS listener accepts any
  DHCP-assigned LAN address for mDNS rediscovery.
- `VERIFIED_AUTOMATED`: Android compiles the matching bounded issuance request and
  validates the response into an in-memory-only monotonic credential object.
- `VERIFIED_AUTOMATED`: the Android client compiles token-at-rest protection using
  Android Keystore AES/GCM and does not log tokens or response bodies.
- `VERIFIED_HARDWARE`: manual enrollment persistence, authenticated daemon access,
  staged short-token testing, and certificate-pin UI behavior work on the real
  Android 8.1 phone through the development ADB tunnel.
- `VERIFIED_HARDWARE`: Android-to-Pi LAN TLS works through NSD-resolved addresses,
  stable mDNS certificate identity, exact pinning, and bearer authentication; a
  deliberately stale address was replaced automatically after relaunch.
- `UNKNOWN`: one-time media-session issuance/expiration on the real phone and
  hardware-backed Keystore availability on this phone.

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
- `VERIFIED_HARDWARE`: phone-to-Pi control requests work over direct pinned LAN TLS
  after mDNS discovery and continue authenticating after endpoint replacement.
- `VERIFIED_AUTOMATED`: an API-27 Telecom `ConnectionService`, stable managed
  `PhoneAccount`, explicit registration/removal switch, and calling-account settings
  action compile and package. Install, launch, enrollment, and upgrade do not
  register or enable the account.
- `UNKNOWN`: compatibility of the target phone's vendor dialer with managed
  third-party Telecom calls; registration is opt-in and has not been enabled in a
  hardware test.
- `VERIFIED_AUTOMATED`: the opt-in outgoing Telecom path validates and redacts the
  target, uses the Keystore enrollment and pinned API, monitors aggregate call
  state, keeps DTMF/hang-up off the monitor thread, starts one-time authenticated
  call audio, follows the system speaker route, and tears down idempotently.
- `UNKNOWN`: real Contacts-to-AnalogBridge dialing and Telecom-owned audio; the
  outgoing-routing APK awaits an operator-available deployment window.
- `VERIFIED_AUTOMATED`: a fail-closed native Contacts account authenticator and
  read-only sync-adapter boundary compile and package for the system Contacts
  authority; account creation, runtime permission requests, and contact writes are
  not yet enabled.

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
- `VERIFIED_HARDWARE`: the daemon detected real incoming and active iPhone calls;
  Pi-originated answer, DTMF `5`, and hangup succeeded, and active SCO validation
  found exactly one source/sink pair.
- `FAILED`: a later answer/hangup run returned the call to idle but left SCO active
  until the HFP profile was cycled; automatic stuck-SCO recovery is still needed.

## Unverified capabilities

- `UNKNOWN`: Pi-originated reject and dial, mute/gain behavior, and
  human-confirmed DTMF audibility.
- `UNKNOWN`: MAP Message Notification Service behavior and reliable incremental sync.
- `UNKNOWN`: whether iPhone MAP notifications remain reliable across idle periods,
  reconnects, and locked-device states; polling remains the safe default.
- `VERIFIED_HARDWARE`: Android-to-Pi-to-iPhone MAP SMS sending works after the
  narrow imsg service-sandbox correction. The preceding failed retest remains
  historical regression evidence.
- `UNKNOWN`: MAP delivery-state notifications, MMS, attachments, sent-folder
  reflection, and locked-iPhone behavior.
- `UNKNOWN`: end-to-end automatic recovery after a physical reboot, Bluetooth
  loss, or network loss; static boot configuration is automated-test verified.
- `VERIFIED_HARDWARE`: short-call subjective audio quality is good in both
  directions with the latest tested build.
- `UNKNOWN`: sustained-call stability with the latest bounded latency trimmer and
  hardware-backed Keystore status.

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

Milestone 7 is the integrated Android companion product. The outbound-message
regression is repaired with the minimum required writable paths. Next, harden
send operation semantics and build the interactive conversation and call
experiences, enable incoming data flows, and close recovery and release gaps. The
authoritative phased plan and exit criteria are in `docs/product-roadmap.md`.

## End-to-end roadmap

The original feasibility milestones 0–6 and most of the secure transport
foundation are complete. Remaining work is organized by user-visible product
outcomes rather than Bluetooth profile experiments; see `docs/product-roadmap.md`.
