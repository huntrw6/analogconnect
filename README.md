# AnalogConnect

A Raspberry Pi bridge that lets an Android 8.1 slider phone act as a companion device for a nearby iPhone.

## Status

Milestone 0 Bluetooth feasibility is complete. Contact synchronization, inbound
and outbound messaging, authenticated Android control, HFP call controls, and the
PipeWire call-audio foundation are implemented at progressively validated stages.
See `docs/current-state.md` for authoritative evidence and remaining hardware gates.

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

MIT
