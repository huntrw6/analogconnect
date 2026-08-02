# AnalogConnect

A Raspberry Pi bridge that lets an Android 8.1 slider phone act as a companion device for a nearby iPhone.

## Status

Milestone 0 feasibility and Milestone 1 backend foundations are complete.
Milestone 2 contact synchronization is implemented for hardware-free testing and
awaits a privacy-controlled iPhone validation. See `docs/current-state.md` for
the authoritative project status.

## Backend development

Run the hardware-free daemon skeleton on its loopback-only default address:

```bash
cargo run -p analogconnectd
```

Then query:

```bash
curl http://127.0.0.1:8787/api/v1/health
curl http://127.0.0.1:8787/api/v1/status
curl http://127.0.0.1:8787/api/v1/contacts/summary
curl http://127.0.0.1:8787/api/v1/messages/summary
curl http://127.0.0.1:8787/api/v1/audio/summary
```

The daemon includes a privacy-safe `imsg` PBAP adapter and SQLite contact store,
but does not trigger hardware synchronization automatically or expose contact
records through the unauthenticated API. It does not expose LAN control.

## License

MIT
