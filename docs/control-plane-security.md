# Control-Plane Security Foundation

## Current policy

- `GET /api/v1/health` is public and contains no private state.
- Every other API endpoint requires `Authorization: Bearer ...`.
- Tokens must contain 32 through 256 bytes.
- Equal-length token comparisons use a constant-time operation.
- Tokens are redacted from `Debug` output and are never logged.
- The daemon refuses to start when no token is configured.
- The plaintext listener is restricted to loopback; non-loopback addresses fail startup.
- Android accepts cleartext HTTP only for loopback and requires HTTPS elsewhere.

## Configuration

Provide the token through `ANALOGCONNECT_API_TOKEN`. It is a credential and must
not be committed, copied into diagnostics, or placed in a shared command history.

## Remaining work

- one-time Android enrollment
- credential rotation and revocation
- Android hardware-backed credential storage where available
- authenticated WebSocket sessions
- transport encryption and explicit LAN exposure policy
- rate limiting and audit events that contain no private payloads

The bearer foundation is not permission to expose the daemon on a LAN yet.

## Enforced cleartext boundary

- `VERIFIED_AUTOMATED`: every non-loopback `ANALOGCONNECT_LISTEN_ADDR` is rejected
  while the daemon has no TLS listener.
- `VERIFIED_AUTOMATED`: Android accepts `http://` only for `localhost`,
  `127.0.0.1`, or `::1`; all other endpoint hosts must use `https://`.

This makes the LAN restriction fail closed rather than relying only on operator
discipline while certificate provisioning is still pending.
