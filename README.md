# AnalogConnect

A Raspberry Pi bridge that lets an Android 8.1 slider phone act as a companion device for a nearby iPhone.

## Status

Milestone 0 is complete. Milestone 0A baseline cleanup and Milestone 1 backend
development are in progress. See `docs/current-state.md` for the authoritative
project status.

## Backend development

Run the hardware-free daemon skeleton on its loopback-only default address:

```bash
cargo run -p analogconnectd
```

Then query:

```bash
curl http://127.0.0.1:8787/api/v1/health
curl http://127.0.0.1:8787/api/v1/status
```

The current daemon does not connect to Bluetooth hardware or expose LAN control.

## License

MIT
