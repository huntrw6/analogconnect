# Control-Plane Security Foundation

## Current policy

- `GET /api/v1/health` is public and contains no private state.
- Every other API endpoint requires `Authorization: Bearer ...`.
- Tokens must contain 32 through 256 bytes.
- Equal-length token comparisons use a constant-time operation.
- Tokens are redacted from `Debug` output and are never logged.
- The daemon refuses to start when no token is configured.
- The plaintext listener is restricted to loopback; non-loopback addresses fail startup.
- Providing both TLS path variables switches the listener to HTTPS and permits an
  explicitly configured LAN bind address.
- Partial TLS configuration, unreadable/invalid PEM data, and insecure Unix
  private-key permissions fail startup.
- Android accepts cleartext HTTP only for loopback and requires HTTPS elsewhere.
- The HTTPS listener supports TLS 1.2 for Android 8.1 compatibility and TLS 1.3
  for newer clients; older TLS versions remain disabled.

## Configuration

Provide the token through `ANALOGCONNECT_API_TOKEN`. It is a credential and must
not be committed, copied into diagnostics, or placed in a shared command history.

The default remains plaintext on `127.0.0.1:8787`. To enable HTTPS, set both:

- `ANALOGCONNECT_TLS_CERT_PATH`: PEM leaf certificate followed by any intermediate
  certificates needed by clients.
- `ANALOGCONNECT_TLS_KEY_PATH`: matching PEM private key. On Unix it must be a
  regular file with no group or other permission bits (for example mode `0600`).

Then set `ANALOGCONNECT_LISTEN_ADDR` to the exact desired socket address. The
daemon never enables a second plaintext listener and never infers LAN exposure.
The certificate must contain the endpoint hostname or IP address in its Subject
Alternative Name. Android treats the explicitly enrolled leaf pin as the trust
anchor, checks the leaf validity period, and retains the platform hostname check;
the leaf does not also need to chain to a platform-trusted CA. Enter the
64-character SHA-256 digest of the leaf certificate's DER encoding in the Android
enrollment screen.

Provision a new private leaf outside the repository with:

```bash
scripts/tls-provision.sh --output /private/path/analogconnect-tls \
  --host 192.0.2.10
```

Use the Pi's actual stable LAN address instead of the documentation-only example.
The tool refuses existing or repository-local output, supports repeated `--host`
arguments, creates a mode-`0600` key, and prints the leaf pin. Certificate rotation
requires generating a new leaf and explicitly replacing the Android enrollment pin.

## Address-independent discovery

The Pi service template at `config/avahi/analogconnect.service` advertises
`_analogconnect._tcp` on port 8787. Android 8.1 does not resolve `.local` names
through its ordinary URL resolver on the target hardware, so the client must use
`NsdManager` to resolve the service immediately before connecting. The resolved
address is transport routing only: the enrolled leaf pin remains the trust anchor,
and the certificate DNS identity comes from the resolved mDNS hostname rather than
from the mutable IP address.

- `DOCUMENTED`: Android API 16 and later provides `NsdManager` discovery and
  per-service resolution callbacks.
- `VERIFIED_HARDWARE`: direct `operat.local` URL resolution fails on the target
  Android 8.1 phone even while the same phone can reach the Pi by IP.
- `VERIFIED_AUTOMATED`: the API-27 Android build discovers `_analogconnect._tcp`,
  resolves the current address immediately before use, and keeps the resolved mDNS
  hostname as the separate TLS certificate identity.
- `UNKNOWN`: Android NSD resolution and reconnect after a real DHCP address change.

## Staged token rotation

1. Set the new credential as `ANALOGCONNECT_API_TOKEN`.
2. Temporarily set the old credential as `ANALOGCONNECT_API_PREVIOUS_TOKEN`.
3. Restart the daemon and update the Android enrollment to the new credential.
4. Remove `ANALOGCONNECT_API_PREVIOUS_TOKEN` and restart to revoke the old token.

Both tokens use the same validation and constant-time comparison policy. The token
set evaluates both candidates without short-circuiting and its diagnostics are
fully redacted. Rotation values must never be placed in repository files or logs.

## Remaining work

- one-time Android enrollment authorization
- automatic expiration and one-time enrollment revocation
- Android hardware-backed credential storage where available
- authenticated WebSocket sessions
- credential-guessing rate limits before authentication

Bearer authentication alone is not permission to expose the daemon on a LAN;
the complete TLS configuration and a correctly provisioned certificate are required.

## Call-media session authorization

Call media does not reuse the long-lived control-plane token. The server-side
foundation issues a distinct 256-bit credential and 128-bit opaque session ID
from the operating system random source. Grants last no more than five minutes,
expire against a monotonic clock, and can be revoked immediately on call teardown.
Presented credentials are strict fixed-length hexadecimal and their decoded bytes
are compared in constant time. A registry permits only one current call-media
grant and one claimed connection. Issuing a replacement revokes the old grant;
dropping a connection lease permits a reconnect while the grant remains valid.
An existing lease observes expiry, replacement, or teardown revocation.

Enrollment and grant `Debug` output redact both values. Random-source and parsing
errors are fixed classifications that contain no candidate material. The session
endpoint returns enrollment material only once to an already authenticated client
and must never log or persist it.

`POST /api/v1/audio/sessions` is the authenticated issuance boundary. It returns
a one-minute enrollment only when the daemon state reports both an active call and
active SCO, shares the authenticated mutation quota, and logs only a fixed event
plus lifetime. The current plaintext server remains loopback-only, so this route
does not authorize LAN exposure; it will move unchanged behind the TLS listener.
Credential responses require `Cache-Control: no-store` and `Pragma: no-cache`;
the Android HTTP client also disables URL caching and sends a no-store request.

- `VERIFIED_AUTOMATED`: deterministic fixtures cover correct, malformed, and
  incorrect credentials; expiry, lifetime bounds, revocation, random-source
  failure, and redaction.
- `VERIFIED_AUTOMATED`: the Raspberry Pi operating-system source produces distinct
  credentials that authorize their corresponding grants without printing them.
- `VERIFIED_AUTOMATED`: one-session and one-connection enforcement, reconnect after
  release, replacement revocation, teardown revocation, and registry/lease
  redaction pass deterministic tests.
- `VERIFIED_AUTOMATED`: the API-27 Android credential object accepts the exact
  server ID/token shape, enforces the same five-minute lifetime ceiling, expires
  against a caller-supplied monotonic clock, and redacts diagnostics and errors.
- `VERIFIED_AUTOMATED`: the issuance endpoint rejects inactive call/SCO state,
  requires bearer authentication, returns only the three contract fields, and
  produces a credential that the server registry can claim. Tests require both
  anti-caching response headers.
- `VERIFIED_AUTOMATED`: the API-27 client compiles a matching request path that
  requires HTTP 201, bounds the response to 1 KiB, rejects missing or extra fields,
  validates all values, and timestamps expiry with Android's monotonic clock. The
  transient result is returned only in memory and is never logged or persisted.
- `VERIFIED_AUTOMATED`: HTTPS listener selection, explicit LAN binding, partial
  configuration rejection, and private-key permission checks.
- `VERIFIED_HARDWARE`: Android 8.1 requires TLS 1.2 on the target phone; a
  TLS-1.3-only listener reaches the Pi but fails the handshake.
- `VERIFIED_HARDWARE`: the real Android 8.1 phone reached the Pi over Wi-Fi,
  accepted the exact pinned self-signed leaf, and authenticated to the HTTPS API.
- `UNKNOWN`: network-connection binding and revocation during a real call.

## Enforced cleartext boundary

- `VERIFIED_AUTOMATED`: every non-loopback `ANALOGCONNECT_LISTEN_ADDR` is rejected
  unless both TLS path variables are present; complete TLS configuration selects
  an HTTPS-only listener.
- `VERIFIED_AUTOMATED`: Android accepts `http://` only for `localhost`,
  `127.0.0.1`, or `::1`; all other endpoint hosts must use `https://`.
- `VERIFIED_AUTOMATED`: Android HTTPS requires an enrolled SHA-256 leaf-certificate
  pin as its explicit trust anchor, uses constant-time digest comparison, checks
  certificate validity dates, retains platform hostname verification, and redacts
  pin diagnostics.

This makes the LAN restriction fail closed rather than relying only on operator
discipline while certificate provisioning is still pending.

## Mutation abuse controls

- `VERIFIED_AUTOMATED`: authenticated SMS and HFP mutations share a bounded quota
  of ten accepted attempts per sixty-second window; excess requests receive 429.
- `VERIFIED_AUTOMATED`: the limiter stores only timestamps and aggregate counts.
- `VERIFIED_AUTOMATED`: successful mutation audit events contain only a fixed event
  name and never recipient, body, dial target, DTMF, or call identity fields.

Pre-authentication credential-guessing controls remain pending and must avoid
creating an unauthenticated global denial-of-service lever.

- `VERIFIED_AUTOMATED`: staged current/previous credential rotation accepts both
  during migration and removal of the previous environment value revokes it on restart.
