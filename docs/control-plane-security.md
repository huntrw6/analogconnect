# Control-Plane Security Foundation

## Current policy

- `GET /api/v1/health` is public and contains no private state.
- Every other API endpoint requires `Authorization: Bearer ...`.
- Tokens must contain 32 through 256 bytes.
- Equal-length token comparisons use a constant-time operation.
- Tokens are redacted from `Debug` output and are never logged.
- The daemon refuses to start when no token is configured.
- The listener remains loopback-only by default.

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
