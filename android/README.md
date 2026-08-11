# AnalogConnect Android client

This is a dependency-free Android 8.1 (API 27) companion client. It provides
local enrollment, privacy-safe daemon health/status checks, call controls/audio,
outbound compose, and an initial interactive conversation list/thread interface.
The dedicated **Open calls** screen polls aggregate state and presents only valid
dial, answer, reject, end, DTMF, speaker, duration, and automatic-audio controls
for the current call phase.
The **Open contacts** screen searches and incrementally loads the private PBAP
snapshot, displays names with their original phone targets, and can prefill the
dedicated call screen. Conversation rows show uniquely matched contact names while
retaining the number internally for explicit private replies.
The bearer token is encrypted with an Android Keystore AES/GCM key and neither
the token nor API response bodies are logged. Token entry is hidden by default
and has an explicit visibility switch for easier entry on physical keyboards.
Non-loopback endpoints require HTTPS and an explicitly enrolled SHA-256 leaf
certificate pin. The pin is not a credential, but diagnostics still redact it.
Saving endpoint or certificate changes with an empty token field preserves the
existing encrypted token. Clearing it requires the separate confirmed action.
The client refreshes `_analogconnect._tcp` discovery whenever it enters the
foreground, replacing a stale saved routing address without changing credentials.

The **Open conversations** screen reads authenticated API v2 conversation and
message pages from imsg's encrypted store, displays unread counts and thread
history, and supports confirmed send/retry with duplicate suppression. Private
addresses and bodies are shown only in the UI and are not logged or placed in
saved-instance state. The current slice displays the first 100 conversations and
messages and visibly reports when another page is available. Participant-set
metadata keeps group messages together and labels each received group message by
sender. Group replies fail closed until multi-recipient MAP push is implemented
and hardware-verified; direct replies retain the existing confirmed send flow.
On the tested iPhone, MAP exposes neither complete participant sets nor usable
conversation identities, so group messages can still appear as separate direct
rows. The app deliberately does not guess from private content or timing.

The APK requests microphone permission only when call audio is explicitly started;
the audio device adapter remains inactive until a short-lived media transport exists.

The Raspberry Pi daemon defaults to loopback. It permits an explicitly configured
LAN listener only when both TLS certificate paths are present; the Android client
then requires the enrolled leaf pin and a matching certificate hostname or IP SAN.
See `docs/control-plane-security.md` for provisioning and enrollment.

## Build

```bash
ANDROID_SDK_ROOT=/home/operat/Android/Sdk ./android/build.sh
```

The unsigned intermediate, generated debug key, and final APK remain under
`android/.build/`, which is ignored by Git. The installable artifact is
`android/.build/analogconnect-debug.apk`.

## Install

With one authorized Android device attached:

```bash
adb install -r android/.build/analogconnect-debug.apk
```

Do not commit the generated keystore or any enrollment token.
