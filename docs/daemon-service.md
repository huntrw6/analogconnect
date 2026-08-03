# Daemon user service

The production daemon runs as the interactive Pi user, preserving access to the
user D-Bus, PipeWire, and WirePlumber sessions without root privileges.

Install the release binary as `~/.local/bin/analogconnectd`, install
`config/systemd/analogconnectd.service` under
`~/.config/systemd/user/analogconnectd.service`, and create
`~/.config/analogconnect/daemon.env` with mode `0600`:

```text
ANALOGCONNECT_API_TOKEN=<operator credential>
ANALOGCONNECT_LISTEN_ADDR=0.0.0.0:8787
ANALOGCONNECT_TLS_CERT_PATH=/home/USER/.config/analogconnect/tls/daemon-cert.pem
ANALOGCONNECT_TLS_KEY_PATH=/home/USER/.config/analogconnect/tls/daemon-key.pem
```

The environment file is operator-owned and must never enter the repository.
The unit restarts only failures, uses a bounded five-second delay, never runs as
root, drops every Linux capability, uses a private umask, and applies systemd
filesystem/kernel hardening. Clean shutdown remains stopped so administrative
stops do not create restart loops.

Validation commands:

```bash
systemd-analyze --user verify config/systemd/analogconnectd.service
systemctl --user daemon-reload
systemctl --user enable --now analogconnectd.service
```

- `DOCUMENTED`: systemd `Restart=on-failure` excludes clean exits and explicit
  service stops.
- `UNKNOWN`: restart after a real daemon crash, reboot startup, and recovery after
  Wi-Fi or Bluetooth loss.
