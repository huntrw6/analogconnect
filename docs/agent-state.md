# AnalogConnect agent state

## Current milestone

Milestone 7 — Integrated Android companion product

## Current objective

Restore reliable outbound messaging, retain the hardware-verified call/audio
vertical slice, and execute `docs/product-roadmap.md` until an operator-only
hardware observation or approved system change is required.

## Current classification

`CALL_AUDIO_AND_SMS_VERTICAL_SLICE_VERIFIED`

## Latest evidence

- `VERIFIED_HARDWARE`: the latest real call sounded good on both ends.
- `VERIFIED_HARDWARE`: Android-originated texting works again after the narrow
  imsg writable-path correction.
- `VERIFIED_HARDWARE`: an earlier Android-originated SMS was delivered through
  iPhone MAP.
- `DOCUMENTED`: the active daemon service makes the home directory read-only;
  `imsg send` opens a store that requires a writable path and may write broker
  state beneath the home directory.
- `VERIFIED_HARDWARE`: the controlled correction and successful retest verify the
  service/imsg writable-path mismatch as the regression cause.
- `VERIFIED_AUTOMATED`: the approved narrow writable-path correction is installed;
  the daemon retained read-only home protection, restarted, and is active.
- `VERIFIED_HARDWARE`: the correction restores real SMS delivery.
- `VERIFIED_HARDWARE`: the deployed v2 API opened the real encrypted imsg store;
  its aggregate conversation result was empty without exposing private fields.
- `VERIFIED_AUTOMATED`: the next daemon release automatically polls and syncs
  exactly inbox and sent, with deleted excluded and command output discarded.
- `VERIFIED_HARDWARE`: that release is installed and active; aggregate checks
  observed two successful syncs, zero failures, five conversations, and five
  correctly ordered messages in one thread without emitting private content.
- `VERIFIED_HARDWARE`: the Android screen displays five populated conversations,
  but the operator found that group messages are incorrectly split by sender.
- `DOCUMENTED`: imsg 0.3.1 and upstream 0.4.0 aggregate by one peer address even
  though the parsed MAP body can contain multiple participant vCards.
- `VERIFIED_AUTOMATED`: a narrow patched imsg store/session preserves participant
  sets, migrates existing handles, keeps direct peer keys stable, groups synthetic
  multi-sender histories, and passes 52 upstream/extension tests.
- `VERIFIED_AUTOMATED`: the matching daemon/API and Android group-safe UI pass all
  project tests; group replies are deliberately disabled.
- `VERIFIED_HARDWARE`: the first deployed participant-set attempt stayed healthy
  but found zero groups and seven rows, proving that vCard cardinality is
  insufficient on this iPhone while the fail-closed reply guard works.
- `VERIFIED_HARDWARE`: explicitly requesting all message-list attributes still
  returned zero MAP conversation identities, and the separate standardized
  conversation-listing operation succeeded but returned zero conversations.
- `VERIFIED_HARDWARE`: the only iPhone MAS record is MAP 1.4 with raw feature mask
  `0x0006027f`. Messages-Listing v1.1 is advertised; Conversation Listing, Event
  Report v1.2, Conversation Version Counters, and feature-mask-in-CONNECT are not.
  The client correctly omits CONNECT tag `0x29` for this server.
- `VERIFIED_HARDWARE`: Messages Listing reported `ListingSize=10` but no
  conversation/direction/participant fields. Conversation Listing returned an
  empty body with `ListingSize` absent. Controlled MAP group/direct events were
  identical and contained no group identity.
- `VERIFIED_HARDWARE`: direct-GATT ANCS subscribed successfully. Two controlled
  group notifications had Title+Subtitle, while a clean direct notification and
  its duplicate update had Title only. Classification is
  `ANCS_GROUP_DETECTION_ONLY`: detection is reliable in this test, but no stable
  group identifier, participant set, or safe reply target is verified.
- `VERIFIED_HARDWARE`: `ANCS_STABLE_GROUP_IDENTITY_VERIFIED`. Different senders
  in the same group produced one normalized Subtitle HMAC, a different group
  produced another, and direct remained Subtitle-absent. The ephemeral key was
  destroyed; the identity is suitable for local incoming/history correlation but
  not group reply.
- `VERIFIED_HARDWARE`: plaintext controlled inspection established
  `ANCS_GROUP_THREADING_VERIFIED`: Title is sender, named-group Subtitle is the
  Messages group name, unnamed-group Subtitle is a participant-generated label,
  and direct Subtitle is empty.
- `VERIFIED_AUTOMATED`: deterministic full-SHA256 `ancs-v1-…` identity,
  encrypted metadata/sender/assignment/conflict/alias storage, close/reopen
  persistence, bounded fail-closed ANCS↔MAP correlation, stable group API fields,
  and Android title/ID/reply guards pass software tests.
- `VERIFIED_AUTOMATED`: the production daemon starts a supervised BlueZ GATT ANCS
  bearer and feeds body-free metadata through immediate MAP sync into the existing
  correlation/apply boundary. The direct-GATT Python probe remains diagnostic.
  Live coexistence, unnamed-group stability, and end-to-end delivery remain
  hardware-pending; group replies remain disabled.
- `VERIFIED_AUTOMATED`: a transport-independent production ANCS protocol core now
  covers strict Notification Source parsing, body-free attribute requests,
  bounded fragment reassembly, Messages filtering, serialized requests,
  duplicate/replay suppression, bounded queues, ordered subscription supervision,
  and capped reconnect backoff. The BlueZ bearer/daemon adapter is implemented;
  real iPhone coexistence remains `UNKNOWN`.
- `BLOCKED`: Accessory Notifications live testing requires iOS 26.5 plus a current
  Mac/Xcode SDK and provisionable Accessory Data Provider, Transport Security,
  and Transport Extension entitlements. This Linux environment has none of that
  Apple build/signing toolchain, so Messages `threadIdentifier` and text-input
  Reply behavior remain untested.
- `VERIFIED_AUTOMATED`: the API-27 call screen is display-only during incoming,
  dialing, ringing, active, and ending states. A bounded physical-key dispatcher
  maps native Call/End keys and active-call digits to backend commands, suppresses
  repeats, and passes 16 state/key regressions. Raw target capture established
  green=`KEY_SEND` and combined red End/Power=`KEY_POWER`. A permission-protected
  Pi ADB monitor forwards raw short presses because this OEM reserves both keys;
  held Power remains native. Hardware capture verified one forwarded short red
  code and an unforwarded held press opening the normal power menu.
  A narrowly scoped key-filter accessibility service is installed and enabled;
  the companion Pi raw-key service is enabled and persistent.
- `VERIFIED_HARDWARE`: physical red rejected an incoming call and ended an active
  call, green answered, touch lockout preserved the call, held Power opened the
  native power menu, and human audio was clear both ways. After removing duplicate
  Pi DTMF forwarding, one physical digit produced one caller-heard tone. The
  foreground watcher, automatic incoming UI, immersive chrome, and proximity
  blanking are verified on the physical API-27 phone.
- `VERIFIED_HARDWARE`: the call-screen APK is installed on the Android device with
  app data preserved and a recoverable pre-deployment APK backup.
- `VERIFIED_HARDWARE`: automatic PBAP contact refresh loaded 438 contacts and 471
  phone fields; the authenticated contact API returned a populated first page and
  all seven current conversation rows received unique contact-name matches without
  emitting names or numbers as evidence.
- `VERIFIED_AUTOMATED`: contact search/pagination, unique number matching, separate
  display-name/routing-number fields, Android contact models/controllers, and
  privacy-safe conversation labels pass the Rust and API-27 suites.
- `VERIFIED_HARDWARE`: the signed contact-list/name-resolution APK and daemon are
  installed. The Android contact list, search, load-more, and name-rendering
  appearance now require a physical walkthrough.

## Last completed action

Completed the first Android product-shell conversion on top of persistent ANCS
group identity: familiar top-level navigation, conversation rows and bubbles,
private compose, fail-closed group/ambiguous states, dial pad and call-state UI,
contacts/settings, preserved Developer Tools, and isolated in-memory fixtures.
The API-27 unit suite and signed APK build pass. Production BlueZ transport now
feeds the ANCS boundary; real iPhone coexistence remains pending and group reply
remains closed.

Subsequent unattended product hardening added physical-device layout correction,
message previews, fixed navigation/composer behavior, onboarding, explicit dark
appearance, encrypted read-only offline cache, API-27 message/call notifications,
notification deep links, connection-safe call controls, structured Developer
Tools, sanitized diagnostics, and build/install/validation scripts.

## Next autonomous actions

1. Run the controlled production ANCS coexistence and direct/group receive matrix.
2. Confirm Android background notification content and deep links with real data.
3. Add persisted call-recents data only after the backend has a truthful source.

## Next operator gate

Call/End delivery, proximity blanking, incoming/outgoing transitions, audio, and
teardown are `VERIFIED_HARDWARE`. Operator testing is now required for production
ANCS coexistence and real message delivery only.

## Authoritative references

- Current evidence: `docs/current-state.md`
- Completion plan: `docs/product-roadmap.md`
- SMS diagnosis: `docs/outbound-message-regression.md`
- Hardware gates: `docs/pending-hardware-tests.md`

Historical phase documents remain useful investigation records but are not the
authoritative current status.
