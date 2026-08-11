# Pending hardware tests

## PENDING USER/HARDWARE TESTS — ANCS group identity

### ANCS-UNNAMED-GROUP-IDENTITY-001

Status: attempted setup; the first subscribed run produced no notification event
after two messages and was discarded. No stability conclusion was drawn.

When the operator is available, reconnect the privacy-scoped ANCS diagnostic and
perform one action at a time:

1. Person A sends one message to a deliberately selected unnamed group.
2. Person B sends one message to that same unnamed group.
3. Restart only the diagnostic connection.
4. Receive one more message in the same group.

Record actual Subtitle values and compare byte equality, normalized equality, and
participant order. Classify only as `ANCS_UNNAMED_GROUP_IDENTITY_VERIFIED` or
`ANCS_UNNAMED_GROUP_IDENTITY_UNSTABLE`. Do not parse/reorder the localized label
without observed evidence.

### ANCS-GROUP-RENAME-001

Rename one controlled named group and determine whether ANCS exposes any stable
old/new association. Until then, a rename intentionally creates a new hash.

### ANCS-SAME-NAME-COLLISION-001

Create two controlled groups with the same display name and verify conflict/split
behavior without sending through the wrong thread.

### ANCS-CONTACT-RENAME-001

Change one controlled participant display name and observe unnamed-group Subtitle
and ordering behavior before defining any canonical participant parser.

## Contact names and contact list — ready

1. Open AnalogConnect and choose **Open contacts**.
2. Confirm familiar contact names appear, search finds a deliberately chosen
   contact by name, and **Load more contacts** adds another page without replacing
   the first page.
3. Select one non-sensitive test contact and confirm **Open calls** receives the
   correct number. Cancel before calling if a call is not desired.
4. Return to **Open conversations** and confirm known participants display as
   contact names. Each direct-looking row should say **Context unknown**, and its
   thread should explain that sending creates a private message.

Report only whether names/search/pagination/matching look correct, whether unknown
numbers fall back sensibly, and any fixed error text. Do not report names or phone
numbers.

## Dedicated Android call screen walkthrough — ready

Evidence needed: physical layout/focus and real call/audio behavior that cannot be
observed by repository automation.

Setup:

1. Keep the iPhone and Pi nearby with Bluetooth and Wi-Fi in their normal working
   state.
2. Open AnalogConnect on the Android and choose **Open calls**.
3. Do not share or photograph phone numbers, contact names, or call logs.

Actions and expected aggregate observations:

1. With no call, confirm the screen says **Ready to call**, shows the number field,
   and D-pad/keyboard focus can reach **Review and call**.
2. Place one ordinary test call. Confirm **Calling…** appears, then **Call in
   progress**, elapsed time advances, and audio connects automatically after the
   microphone prompt if it appears.
3. Confirm speech is intelligible both ways through the earpiece, toggle
   **Speakerphone** once, send one harmless DTMF digit only if the destination can
   safely receive it, and end the call from Android.
4. If convenient, make one incoming call and confirm **Answer** and **Reject** are
   visible. Testing either action is sufficient; an incoming call is optional for
   the first pass.

Stop conditions: stop immediately for feedback/echo, stuck loudspeaker audio,
failure to end the cellular call, repeated permission prompts, app crash, or any
effect on emergency/ordinary Android cellular calling.

Report only: whether each state label appeared, focus/layout usability, audio on
both ends, speaker result, optional DTMF result, teardown result, and any fixed UI
error text. No private identifiers or message/call content are needed.

This checklist contains only tests that require the operator or physical phones.
No listed feature should be treated as hardware-verified until its result is added
to `docs/current-state.md` with an evidence label.

## Sustained call audio

- `VERIFIED_HARDWARE`: a recent normal-duration call sounded good on both ends.
- `UNKNOWN`: long-duration stability with the latest latency trimmer remains.

- Run a call for at least five minutes with the latest latency trimmer.
- Observe `buffer`, `holds`, `late`, `overflow`, `trims`, and `pace` around one,
  three, and five minutes.
- Confirm clarity, popping, and subjective delay through earpiece and speakerphone.

## Outbound-message regression isolation — complete

- `VERIFIED_HARDWARE`: texting works correctly after granting the daemon only
  imsg's data and state writable paths.
- `VERIFIED_HARDWARE`: the controlled correction/retest isolated the prior
  read-only service sandbox as the regression cause.

Remaining message hardware coverage belongs to the product reliability matrix:
fresh app launch, long idle/reconnect, Pi reboot, honest failure/retry, duplicate
prevention, sent-folder reflection, and incoming-message notification behavior.

## Plug-in-and-use restart

- With both phones available, reboot the Pi without changing enrollment.
- Confirm the daemon becomes healthy without an interactive Pi login.
- Confirm Android discovers the newly assigned address and authenticates with its
  existing certificate pin and token.
- Confirm the trusted iPhone reconnects HFP, MAP, and PBAP without pairing again.
- Place one call and send one deliberate message after recovery.

## Conversation interface v2

- `VERIFIED_HARDWARE`: the signed APK and initial v2 daemon are installed, and the
  real encrypted store opened through the aggregate-only API; it was empty.
- `VERIFIED_HARDWARE`: the automatic inbox/sent daemon is installed and active;
  aggregate checks observed two successful syncs, zero failures, five
  conversations, and a correctly ordered five-message history page.
- Open **Open conversations** and report whether the screen loads or shows one of
  the fixed errors. Do not capture or transcribe addresses or message bodies.
- Report only the number of visible conversation rows and whether newest-looking
  conversations appear first.
- Open one deliberately selected conversation and report only the visible message
  count, whether chronological display looks correct, and whether sent/received
  labels look correct.
- Send one synthetic message from that thread, confirm recipient delivery, and
  report whether the draft cleared and the thread refreshed without duplication.
- If **more available** appears, report it; pagination controls are not yet enabled.
- Stop immediately if the screen displays raw backend/database/keyring errors or
  content from the wrong conversation.

## Experimental Android Phone integration

- Confirm the app preserves the saved enrollment after the settings-store migration.
- Enable **Register AnalogBridge calling account**, open calling-account settings,
  and report whether Android shows **AnalogBridge iPhone**.
- Do not make it the default for all calls until ordinary cellular and emergency
  calling behavior is confirmed unchanged.
- From a deliberately selected test contact, choose the AnalogBridge account and
  confirm the iPhone places the intended call.
- During that call, verify dialing/active/ended UI, earpiece, speakerphone, DTMF,
  and hang-up.
- Disable the switch and confirm the account disappears without affecting contacts
  or the normal cellular calling account.

## Not ready for hardware testing

- Incoming calls through Android Telecom.
- Native Contacts synchronization; its account and sync boundaries are packaged
  but intentionally refuse activation.
- Background message notifications (the foreground conversation inbox is ready
  for the hardware test above).
