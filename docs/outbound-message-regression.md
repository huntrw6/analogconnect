# Outbound-message regression

## Status

- `VERIFIED_HARDWARE`: an earlier Android-originated SMS passed through the Pi and
  iPhone and reached the intended recipient.
- `FAILED`: the pre-correction operator retest did not work.
- `VERIFIED_HARDWARE`: texting worked correctly again after installing the narrow
  imsg writable-path exceptions, confirming the service sandbox/store mismatch.
- `DOCUMENTED`: the failed result text/HTTP status was not retained, but the
  controlled configuration change and successful retest isolated the cause.

## Privacy-safe path trace

```text
Android confirmation UI
  -> pinned HTTPS POST /api/v1/messages
  -> authentication, rate limit, and input validation
  -> analogconnectd ImsgMapBackend
  -> imsg send
  -> imsg encrypted store/broker
  -> Bluetooth MAP PushMessage
  -> iPhone SMS transport
```

- `VERIFIED_AUTOMATED`: Android request construction, API authentication and
  validation, redacted errors, and a mock MAP backend pass automated tests.
- `DOCUMENTED`: a backend failure maps to HTTP 502 and Android displays only the
  HTTP status, so a controlled retry can distinguish API/transport failure without
  retaining message content.
- `DOCUMENTED`: `imsg 0.3.1` calls `load_with_store` for every `send`, initializes
  its keyring, and opens the SQLCipher database before choosing stored-outbox or
  live-send behavior.
- `DOCUMENTED`: the imsg store declares that its database path must be writable.
  The broker can also create a log under the per-user state directory.
- `DOCUMENTED`: the installed daemon service is active with
  `ProtectHome=read-only`; at the time of the failed retest it had no
  `ReadWritePaths` exceptions.
- `INFERRED`: this sandbox mismatch is the leading cause. It is temporally
  plausible because persistent service hardening was added after outbound
  messaging was first implemented and hardware-verified.
- `VERIFIED_AUTOMATED`: on 2026-08-09 the tracked and installed service gained
  narrow exceptions for imsg's data and state directories, retained the read-only
  home policy, restarted successfully, and remained active.
- `VERIFIED_HARDWARE`: real SMS delivery works after the correction.

## Competing hypotheses

- `UNKNOWN`: a stale or disconnected MAP broker/session.
- `UNKNOWN`: iPhone MAP permission or reconnect behavior changed.
- `UNKNOWN`: Android enrollment/discovery failed, although working calls make a
  broad connectivity failure less likely if both were tested in the same session.
- `UNKNOWN`: the API accepted the push but the iPhone or carrier did not deliver
  it.

## Isolation sequence

1. Record only the Android's fixed result category (`HTTP 401`, `422`, `429`,
   `502`, timeout, or accepted); never record recipient/body.
2. If it is not `502`, follow the corresponding auth, validation, rate-limit, or
   network branch before touching MAP.
3. If it is `502`, compare a direct privacy-safe MAP capability check with the
   service context. Do not send a message during this check.
4. With approval, grant the daemon only the specific imsg data/state directories
   it needs, reload/restart the user service, and repeat one deliberate send.
5. If the sandbox change does not isolate the problem, inspect broker state and
   MAP reconnect behavior using aggregate output only.
6. Add an automated service-policy regression check and a backend diagnostic code
   that distinguishes spawn/store/broker/MAP failures without including private
   values.

## Fix acceptance criteria

- `VERIFIED_AUTOMATED`: service-policy tests prove imsg's required data/state
  paths are writable while the remainder of the home directory stays read-only.
- `VERIFIED_AUTOMATED`: backend/API tests expose a fixed privacy-safe failure class
  and never subprocess output or message fields.
- `VERIFIED_HARDWARE`: three deliberate sends succeed across a fresh app launch,
  an idle/reconnect interval, and a Pi reboot, without duplicate delivery.
- `VERIFIED_HARDWARE`: one controlled failure is surfaced honestly and can be
  retried without losing the compose text or sending twice.
