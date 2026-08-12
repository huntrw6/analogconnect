<h1>
  <img src="https://cdn.phototourl.com/free/2026-08-12-5009f1e5-01d1-4dce-8f17-e987f733cc49.png" width="48" align="absmiddle">
  AnalogBridge
</h1>

A Pi bridge that lets an Android 8 device act as a companion phone.

The iPhone remains the cellular phone. A Raspberry Pi connects it to an Android
8.1 phone over Bluetooth and secure local Wi-Fi.

## Current capabilities

- Connects a Raspberry Pi to an iPhone for contacts, messages, calls, and call audio.
- Lets the Android phone send a message through the iPhone.
- Shows iPhone call state and provides answer, reject, hang-up, dial, DTMF, and mute controls.
- Carries live microphone and call audio in both directions.
- Supports earpiece and speakerphone playback on Android.
- Automatically discovers the Pi again when its local network address changes.
- Protects Pi-to-Android traffic with HTTPS certificate pinning and authentication.
- Keeps credentials, message contents, contact details, phone numbers, and audio out of logs.

Real-device testing has verified message delivery, call controls, two-way intelligible
audio, speakerphone routing, and automatic Pi address rediscovery. Sustained-call
latency and audio smoothing are still being tuned. See
[`docs/current-state.md`](docs/current-state.md) for detailed evidence and remaining work.

## Backend development

Set a private token of at least 32 bytes, then run the daemon on its loopback-only
default address. Never commit the token or place it in shared shell history.

```bash
read -rsp "AnalogConnect API token: " ANALOGCONNECT_API_TOKEN
export ANALOGCONNECT_API_TOKEN
cargo run -p analogconnectd
```

Then query:

```bash
curl http://127.0.0.1:8787/api/v1/health
curl -H "Authorization: Bearer $ANALOGCONNECT_API_TOKEN" http://127.0.0.1:8787/api/v1/status
curl -H "Authorization: Bearer $ANALOGCONNECT_API_TOKEN" http://127.0.0.1:8787/api/v1/contacts/summary
curl -H "Authorization: Bearer $ANALOGCONNECT_API_TOKEN" http://127.0.0.1:8787/api/v1/messages/summary
curl -H "Authorization: Bearer $ANALOGCONNECT_API_TOKEN" http://127.0.0.1:8787/api/v1/audio/summary
```

The daemon includes a privacy-safe `imsg` PBAP adapter and SQLite contact store,
but does not trigger hardware synchronization automatically or expose contact
records through the API. Every endpoint except health requires constant-time bearer
authentication. Plaintext remains loopback-only. See
[`docs/control-plane-security.md`](docs/control-plane-security.md) for the explicit
HTTPS configuration required before binding to a LAN address.

## License

GNU General Public License v3.0. See [`LICENSE`](LICENSE).
