# AnalogConnect

AnalogConnect turns an Android 8.1 slider phone into a local companion for a
nearby iPhone. The iPhone remains the cellular device; a Raspberry Pi bridges
messages, contacts, calls, and call audio to a familiar Android interface.

```text
iPhone
  ├─ MAP  → message history and direct sending
  ├─ ANCS → notification context and group identity
  ├─ PBAP → contacts
  └─ HFP  → call control and two-way audio
             ↓
       Raspberry Pi / analogconnectd
             ↓ authenticated local API
       Android 8.1 (API 27)
```

## Features and status

### Messaging

- Conversation list, searchable direct and group threads, message bubbles,
  unread state, direct composition, Android notifications, and deep links.
- MAP provides history, incoming messages, and private outbound sending.
- ANCS provides privacy-safe incoming context. A named group's Subtitle becomes
  its title; an unnamed group's participant-generated Subtitle is retained as
  its title. Normalized Subtitles produce stable `ancs-v1-*` conversation IDs.
- Different senders correlated to the same group remain in one thread. Competing
  evidence is persisted as ambiguous and fails closed.
- Group replies remain disabled because exact group-reply targeting has not been
  verified on hardware.

The ANCS protocol consumer, supervised production BlueZ GATT bearer, reconnect
logic, and ANCS/MAP correlation pipeline are `VERIFIED_AUTOMATED`. Live ANCS
coexistence with MAP, PBAP, and HFP is hardware verification pending.

### Calls

HFP calling and bidirectional audio are `VERIFIED_HARDWARE`, including incoming
and outgoing calls, physical Answer, Decline, active-call End, physical DTMF,
proximity screen blanking, clean teardown, and intelligible audio both ways.
Live call screens deliberately expose no touch-activated call controls, reducing
accidental cheek input. The green Call and red End/Power hardware buttons retain
their conventional roles; holding Power still reaches native power behavior.

### Android application

The API-27 app provides Messages, Calls, Contacts, and Settings, plus onboarding,
light/dark themes, connection recovery, an Android-Keystore-protected offline
cache, notification channels, and Developer Tools. Synthetic/demo data is
isolated from production persistence. The Pi remains authoritative for sends and
live call state.

### Privacy and security

AnalogConnect is local-first: no cloud service, iPhone companion app, Mac, or
CarPlay integration is required. Message storage is encrypted, Android cache data
uses AES-GCM with Android Keystore keys, API mutations are authenticated, and LAN
operation uses TLS certificate pinning. Diagnostics redact phone numbers,
Bluetooth addresses, message bodies, credentials, tokens, and pairing material.

## Supported environment

- Raspberry Pi running Linux, BlueZ, PipeWire/WirePlumber, Rust, and `imsg`
- Paired/trusted iPhone providing MAP, ANCS, PBAP, and HFP
- Android 8.1 / API 27 companion phone; the physical-key integration targets the
  validated slider handset and may need adaptation for different hardware keys

## Build and run

The repository's service setup expects a configured `imsg` installation,
Bluetooth trust, private daemon environment file, TLS material for LAN use, and
Android enrollment. See the deployment and security documentation before using
real devices.

```bash
# Complete release validation (Rust, vendor crates, Android, signing, schemas,
# Python, and shell checks)
scripts/validate.sh

# Build the daemon and signed debug APK
cargo build --release -p analogconnectd --bin analogconnectd
android/build.sh

# Upgrade one connected Android device without clearing app data
scripts/install-android.sh
```

For loopback-only backend development, provide a private token of at least 32
bytes without committing it or placing it in shared shell history:

```bash
read -rsp "AnalogConnect API token: " ANALOGCONNECT_API_TOKEN
export ANALOGCONNECT_API_TOKEN
cargo run -p analogconnectd
```

## Current limitations

- Production BlueZ ANCS integration is implemented but awaits live coexistence
  and end-to-end iPhone validation.
- Group reply and ANCS positive-action invocation are disabled pending exact-target
  hardware proof.
- Unnamed-group reconnect stability, same-name collisions, and rename behavior
  still require controlled iPhone tests.
- Recents does not claim historical iPhone call logs; truthful observed-call
  attribution remains incomplete.

## Documentation

- [Current evidence and status](docs/current-state.md)
- [Product roadmap](docs/product-roadmap.md)
- [Production ANCS transport](docs/ancs-production-transport.md)
- [Group detection and identity](docs/group-chat-detection-report.md)
- [Android product UI](docs/android-product-ui.md)
- [Physical call controls](docs/physical-call-controls.md)
- [Daemon deployment](docs/daemon-service.md)
- [Control-plane security](docs/control-plane-security.md)
- [Developer diagnostics](docs/developer-diagnostics.md)
- [Pending hardware tests](docs/pending-hardware-tests.md)

## License

GNU General Public License v3.0. See [LICENSE](LICENSE).
