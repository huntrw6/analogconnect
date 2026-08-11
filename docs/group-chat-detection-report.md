# iPhone group-chat detection and reply feasibility

Date: 2026-08-10

## Executive conclusion

AnalogConnect can reliably detect that a tested incoming iPhone Messages
notification belongs to a group conversation by using Apple Notification Center
Service (ANCS) over BLE. In two controlled group tests, both notifications had a
non-empty ANCS `Subtitle`; a clean controlled direct message and its duplicate
ANCS update had an empty `Subtitle`.

Evidence classification: `VERIFIED_HARDWARE`.

Strongest incoming-identity classification:
`ANCS_GROUP_THREADING_VERIFIED`.

ANCS Subtitle provides a privacy-safe local identity after normalization and
keyed hashing: two different senders in the same controlled group produced the
same HMAC, a different group produced a different HMAC, and a direct message had
no Subtitle. This still is not a group reply target. AnalogConnect cannot safely
address a reply to the existing iPhone group, so group replies must remain
disabled. A sender-only reply must be explicitly presented as a private message.

## Product integration checkpoint

- `VERIFIED_AUTOMATED`: named and unnamed group Subtitle titles flow through the
  group-aware API into Android conversation rows and thread headers without
  exposing deterministic identities.
- `VERIFIED_AUTOMATED`: group messages remain in one thread and display their
  current sender above incoming bubbles in both controller tests and isolated
  UI fixtures.
- `VERIFIED_AUTOMATED`: Android does not instantiate a composer for group or
  ambiguous conversations and explains that group replies are not available.
- `VERIFIED_AUTOMATED`: the protocol boundary retains NotificationUID plus
  positive/negative labels, represents explicit positive/negative actions, and
  builds the ANCS action command behind an unimplemented bearer interface. No
  production path invokes notification actions automatically.

## Safety and privacy boundaries

The experiments used controlled direct and group conversations. Diagnostic
output contained no Bluetooth address, telephone number, contact name, message
body, notification text, pairing data, or credential. ANCS attribute values were
held transiently in memory and rendered only as presence, byte count, word count,
and punctuation count. The ANCS `Message` attribute was never requested.

No messages were deleted. No group reply was sent. Pairing and Bluetooth
configuration were preserved. The regular Bluetooth connection and daemon were
restored after testing.

## MAP capability discovery

The iPhone exposed one relevant Message Access Server (MAS) SDP record:

| Property | Result | Evidence |
| --- | --- | --- |
| MAS instance | 0 | `VERIFIED_HARDWARE` |
| Message types | SMS_GSM (`0x02`) | `VERIFIED_HARDWARE` |
| MAP version | 1.4 (`0x0104`) | `VERIFIED_HARDWARE` |
| RFCOMM channel | 2 | `VERIFIED_HARDWARE` |
| Raw `MapSupportedFeatures` | `0x0006027f` | `VERIFIED_HARDWARE` |
| Messages-Listing v1.1 | advertised | `VERIFIED_HARDWARE` |
| Extended Event Report v1.1 | advertised | `VERIFIED_HARDWARE` |
| Event Report v1.2 | not advertised | `VERIFIED_HARDWARE` |
| Conversation Version Counter | not advertised | `VERIFIED_HARDWARE` |
| Conversation Listing | not advertised | `VERIFIED_HARDWARE` |
| `MapSupportedFeatures` in CONNECT | not advertised | `VERIFIED_HARDWARE` |

The mask sets feature bits 0–6, 9, 17, and 18. The MAP specification makes OBEX
CONNECT application parameter `0x29` conditional on the server advertising the
corresponding supported-feature bit. This iPhone does not advertise that bit.
The existing client sends only the MAP target header and omits `0x29`; that is the
correct negotiation for this server. Sending `0x29` anyway would not be a valid
fix for the missing conversation data.

## Messages-Listing v1.1 experiment

The diagnostic client was minimally extended to preserve:

- `conversation_id`
- `conversation_name`
- `direction`
- raw XML attribute names
- response application parameters
- listing size, body size, database identifier, and version counter

It requested the inbox with `MaxListCount=100`, `ListStartOffset=0`, and all
message-listing fields. The controlled direct and group messages were compared
without retaining bodies.

Result:

```text
OBEX result:                         0xa0 (success)
ListingSize:                         10
Parsed messages:                     10
conversation_id fields:              0
conversation_name fields:            0
direction fields:                    0
participant attributes:              0
MAS instance difference:             none
direct/group raw attribute difference: none
```

Both controlled samples exposed the same raw attribute-name set:

```text
handle, subject, datetime, sender_name, sender_addressing,
recipient_name, recipient_addressing, type, size, reception_status,
text, attachment_size, read, sent
```

Conclusion: `FAILED` for group identity. Messages-Listing v1.1 is advertised and
works, but this iPhone does not provide group conversation fields in these
responses.

## Conversation Listing experiment

The client issued `x-bt/MAP-convo-listing` with `MaxListCount=100`,
`ListStartOffset=0`, and no filters against every relevant MAS instance.

```text
OBEX result:                         0xa0 (success)
ListingSize:                         absent
Body size:                           0
ConversationListingVersionCounter:  absent
DatabaseIdentifier:                 absent
Parsed conversations:               0
```

The important distinction is that the response was not a populated listing with
a declared size of zero. It was an accepted OBEX request with an empty body and
no `ListingSize`. This agrees with the SDP mask, which does not advertise
Conversation Listing.

Conclusion: `FAILED` for group identity on this iPhone/MAS combination.

## Raw bMessage recipient experiment

Earlier controlled direct/group inspection used the lowest-level unparsed MAP
GetMessage response before the existing parser modified it. The group bMessage
did not expose a verified complete recipient set distinct from the direct
message. Participant reconstruction by counting originator/recipient vCards
therefore failed.

Conclusion: `FAILED`. This path should not be repeated unless the iPhone MAP
behavior or test conditions materially change.

## MAP notification event experiment

The upstream notification server advertised MAP 1.1 and no supported-feature
attribute, so a minimal diagnostic Message Notification Server was created. It
advertised MAP 1.4, registered with BlueZ, enabled MAP notifications, and printed
`READY` only after accepting the actual MNS OBEX connection. Raw event XML was
parsed in memory and only attribute names/presence were emitted.

Controlled group event:

```text
event type:          NewMessage
conversation_id:    absent
conversation_name:  absent
participant_uci:    absent
contact_uid:         absent
raw attributes:      type, handle, folder, msg_type
```

Controlled direct event: identical.

The server advertises Extended Event Report v1.1 but not Event Report v1.2. The
live result confirms its delivered `NewMessage` reports contain no group context.

Conclusion: `FAILED` for group identity.

## ANCS BLE experiment

### Bearer establishment

BlueZ had a paired, bonded, trusted iPhone identity and cached the ANCS service
and its three characteristics, but its normal `Connected=yes` state represented
the classic bearer. An explicit BlueZ profile connection did not establish LE
GATT. A direct public-address GATT connection succeeded only after the classic
connection was fully disconnected while the iPhone Bluetooth settings page was
open.

The diagnostic then subscribed to:

- Notification Source
- Data Source

and wrote metadata requests to the Control Point. No pairing reset or
configuration change was required.

### Requested attributes

Only these ANCS attributes were requested:

- AppIdentifier
- Title
- Subtitle
- MessageSize
- Date
- PositiveActionLabel
- NegativeActionLabel

The notification Message body attribute was deliberately omitted.

### Controlled results

| Sample | Title | Subtitle | Interpretation |
| --- | --- | --- | --- |
| Group 1 | present, 2 words | present, 2 words | group pattern |
| Group 2, same known group | present, 2 words | present, 2 words | group pattern repeated |
| Clean direct control | present, 3 words | absent | direct pattern |
| Duplicate update for direct control | present, 3 words | absent | direct pattern repeated |

An earlier direct run received both a new and replayed Messages notification and
was treated as inconclusive. It was not used as the decisive direct control.

Conclusion: `ANCS_GROUP_DETECTION_ONLY`, `VERIFIED_HARDWARE`.

Subtitle presence distinguished controlled group notifications from the clean
direct notification across two group samples. The structural capture did not
prove that Title or Subtitle contains a stable group name, stable participant
identity, or the complete participant set. Field lengths also varied between the
two group samples, so lengths must never be used as a conversation identifier.

### ANCS Subtitle identity follow-up

A second controlled experiment normalized Subtitle with Unicode NFKC,
case-folding, whitespace collapse, and trimming. It computed HMAC-SHA256 with a
random in-memory key, emitted only a 12-hex-character prefix and word count, and
destroyed the key when the process ended. Duplicate notification UIDs and
subscription-time replays were excluded.

```text
Group A sender 1: a677aa2235e0
Group A sender 2: a677aa2235e0
Group A sender 3: unavailable (only two other participants)
Group B:          05e10cbd6530
Direct:           Subtitle absent
```

Conclusion: `ANCS_STABLE_GROUP_IDENTITY_VERIFIED`, `VERIFIED_HARDWARE`. The
normalized Subtitle HMAC is a stable privacy-safe local conversation key for the
controlled incoming/history-correlation cases. It is not an address accepted by
MAP or Accessory Notifications and does not enable group reply.

## Apple Accessory Notifications feasibility

Official Apple documentation for the newer Accessory Notifications and Accessory
Transport Extension frameworks establishes:

- `AccessoryNotification` contains `identifier`, `sourceName`,
  `threadIdentifier`, and `actions`.
- An action can have type `textInput(placeholder:)`.
- `NotificationResponse` carries `sourceIdentifier`, `notificationIdentifier`,
  `actionIdentifier`, and optional `userText`.
- The extension sends a response to iOS with `sendResponse(_:)`.
- Notification forwarding uses an AccessorySetupKit accessory plus data-provider,
  transport-security, and transport extensions. Transport preference is
  Bluetooth, local network, then internet.
- Required extension entitlements are
  `com.apple.developer.accessory-data-provider`,
  `com.apple.developer.accessory-transport-security`, and
  `com.apple.developer.accessory-transport-extension`.
- Development testing is allowed on iPhone in any region; customer use is
  restricted to devices located in the EU and signed into an EU-region Apple
  Account.

The current Apple DocC metadata introduces these frameworks at iOS 26.5. A live
proof therefore requires an iPhone running iOS 26.5 or later, a Mac capable of
running current Xcode with the iOS 26.5 SDK, valid signing/provisioning for all
three extension entitlements, and an AccessorySetupKit-compatible transport
prototype. This AnalogConnect environment is Linux and has no `xcodebuild` or
Swift toolchain. No Mac is available under the project constraints.

Apple documents the generic fields and response mechanism but does not promise
that Messages supplies a non-nil/stable `threadIdentifier` or exposes its Reply
action as `textInput`. Those are hardware-test questions. Apple documentation
also does not establish that a free Personal Team can provision all three new
extension entitlements; account eligibility remains unverified until tested in
Xcode or clarified by Apple.

Classification:
`ACCESSORY_NOTIFICATIONS_BLOCKED: no compatible Mac/Xcode iOS 26.5 build and
signing environment; Messages-specific thread/action behavior is untested`.

Official sources:

- https://developer.apple.com/documentation/accessorynotifications
- https://developer.apple.com/documentation/accessorynotifications/accessorynotification
- https://developer.apple.com/documentation/accessorynotifications/notificationresponse
- https://developer.apple.com/documentation/accessorytransportextension/receiving-ios-notifications-on-an-accessory
- https://developer.apple.com/documentation/ios-ipados-release-notes/ios-ipados-26_4-release-notes
- https://developer.apple.com/xcode/system-requirements

## Reply feasibility

| Candidate target | Result |
| --- | --- |
| MAP `ConversationID` | unavailable |
| MAP complete recipient set | unavailable |
| MAP event conversation target | unavailable |
| ANCS notification UID | notification-scoped; not a MAP PushMessage target |
| ANCS group name/participants | not verified |
| Safe group reply test | not currently possible |

No real group message should be sent until either a stable MAP ConversationID or
a verified complete recipient set is available. Pushing to only the latest
sender would create a private message and must never be labeled “Reply to group.”

## Required domain model

The backend must make reply safety explicit:

```text
PrivateTarget(address)
GroupTarget(conversation_id | verified_complete_participant_set)
UnknownTarget
```

ANCS detection changes context, not target:

```text
MAP message + correlated ANCS group signal
                  |
                  v
      GROUP + UnknownTarget
```

It must not manufacture `GroupTarget` from the MAP sender, an ANCS notification
UID, Title/Subtitle lengths, timing alone, or a partial participant list.

## Recommended implementation for group detection

### 1. Add an ANCS notification consumer to the Pi bridge

Port the bounded diagnostic into a supervised component. It should own or share a
proper BlueZ LE connection, subscribe to Notification Source/Data Source, request
only the approved metadata attributes, and never request or log Message content.
The current direct-GATT diagnostic proves the protocol path but is not production
connection management.

### 2. Represent context separately from reply target

Suggested internal types:

```text
ConversationKind = PRIVATE | GROUP | AMBIGUOUS
DetectionSource = MAP | ANCS_SUBTITLE | CORRELATED
DetectionConfidence = VERIFIED_RULE | HEURISTIC | UNKNOWN
ReplyTarget = PrivateTarget | GroupTarget | UnknownTarget
```

For the verified rule in this report:

```text
Messages app + non-empty Subtitle => GROUP
Messages app + empty Subtitle     => PRIVATE
missing/late/conflicting ANCS      => AMBIGUOUS
```

This rule must be guarded by device/OS compatibility evidence and fail to
`AMBIGUOUS` if ANCS is unavailable or metadata does not match the tested shape.

### 3. Correlate ANCS notifications with MAP messages transiently

Create a `ConversationContextResolver` with a short in-memory correlation window.
Potential privacy-safe signals are:

- arrival-time proximity
- sender identity HMAC using an ephemeral or protected application key
- message length
- transient body HMAC if both paths expose the body in memory

Do not persist plaintext notification values or message bodies. Do not use time
alone when multiple messages arrive close together. Unmatched, competing, or
expired candidates resolve to `AMBIGUOUS`.

### 4. Persist only minimal context

The message/conversation store may persist:

- `conversation_kind`
- detection source and confidence
- correlation timestamp/version
- whether the reply target is verified

It should not persist ANCS Title/Subtitle plaintext solely for detection. If the
product later needs a displayed group label, that requires a separate privacy
decision and a test proving what the field semantically contains.

### 5. Expose fail-closed API behavior

For a detected group without a verified target, return:

```text
kind: GROUP
reply_target: UNKNOWN
can_reply_to_group: false
can_message_sender_privately: true
```

For absent or conflicting context, return `AMBIGUOUS`, never `PRIVATE` by
default.

### 6. UI behavior for later milestone

Do not implement this during Raspberry Pi feasibility work. When UI work resumes:

- Private + `PrivateTarget`: normal Reply.
- Group + verified `GroupTarget`: Reply to group.
- Group + `UnknownTarget`: show group context, disable group reply, and offer an
  explicitly labeled “Message sender privately.”
- Ambiguous: state that conversation context is unknown and do not show a normal
  Reply action.

## Test plan for the production detector

Automated tests should cover:

1. ANCS fragment reassembly across arbitrary Data Source boundaries.
2. Duplicate Added/Modified events for the same notification UID.
3. Message-body attribute is absent from every Control Point request.
4. Direct Subtitle absent maps to private context only when correlation succeeds.
5. Group Subtitle present maps to group context but leaves target unknown.
6. Missing ANCS, timeouts, competing candidates, and conflicting metadata map to
   ambiguous.
7. Logs and errors contain no raw attributes, identifiers, addresses, or bodies.
8. Reconnect/backoff does not disrupt working MAP/PBAP/HFP profiles.

Required hardware regression matrix:

- at least five direct notifications
- at least five messages across two named groups
- at least five messages across two unnamed groups
- multiple senders within one group
- locked and unlocked iPhone states
- notification previews enabled and disabled
- rapid direct/group arrival ordering
- duplicate ANCS updates
- Bluetooth loss/recovery while MAP/HFP are in use

Group detection can be promoted from this controlled proof to production support
only when the rule remains stable across that matrix. Safe group reply remains a
separate gate.

## Final checkpoint

### MAP

```text
Raw server feature mask:       0x0006027f
Client 0x29 sent:              no; correctly omitted because bit 19 is clear
Conversation ListingSize:      absent
conversation_id:               unavailable
conversation_name:             unavailable
Event Report group metadata:   unavailable
```

### ANCS

```text
BLE bearer:                    established with direct GATT
Subscribed:                    Notification Source + Data Source
Direct/group distinguishable:  yes in controlled tests
Group name/participants:       not verified
```

### Reply

```text
ConversationID target available:          no
Verified recipient-set target available:  no
Safe reply test possible:                 no
```

### Classification

```text
ANCS_GROUP_DETECTION_ONLY
```

## Next action

Run `ANCS-UNNAMED-GROUP-IDENTITY-001` to compare the same unnamed-group Subtitle
across two senders and a diagnostic restart. Do not parse or reorder Apple's
localized participant label without observed evidence.

## Implemented group-threading software

- `VERIFIED_AUTOMATED`: version-one normalization and full SHA-256 IDs are stable
  without a process secret and reject empty direct Subtitles.
- `VERIFIED_AUTOMATED`: encrypted storage keeps group metadata, plaintext display
  Subtitle, observed senders, NotificationUID assignment, timestamps, conflict
  state, and future aliases/splits separately from immutable message IDs.
- `VERIFIED_AUTOMATED`: one proven correlation rewrites only the message's
  conversation key; different senders assigned to the same ID aggregate into one
  thread, while the ANCS Subtitle remains the title.
- `VERIFIED_AUTOMATED`: API and Android support stable `ancs-v1-…` IDs and explicit
  group/ambiguous reply-disabled states. Private conversations retain their
  existing identity and private-send behavior.
- `VERIFIED_AUTOMATED`: a supervised production BlueZ GATT listener now feeds the
  daemon boundary and triggers MAP synchronization. The Python direct-GATT probe
  remains diagnostic only; live end-to-end Android group threading is not claimed.
- `VERIFIED_AUTOMATED`: the reusable ANCS protocol consumer implements strict
  Notification Source decoding, one-at-a-time metadata requests, bounded Data
  Source fragment reassembly, Messages-only filtering, Added/Modified replay
  suppression, bounded UID queues, action-label capture, and reconnect backoff.
  Its transport-neutral supervisor orders both subscriptions, resets partial state
  on disconnect, and schedules bounded retries. It never requests the Message-body
  attribute. The BlueZ bearer and daemon task now feed this core; real iPhone
  coexistence and notification delivery remain hardware-pending.
- `NOT VERIFIED`: group reply targeting.

Future rename test: `ANCS-GROUP-RENAME-001`. A renamed group is expected to hash
to a new ID; no automatic merge should occur until hardware evidence shows whether
ANCS exposes a safe old/new association.
