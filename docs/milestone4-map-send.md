# Milestone 4 — Outbound MAP messaging

## Current design

The authenticated control API accepts one transient recipient/body pair, validates
it, and delegates synchronously to `imsg send`. The daemon does not persist the
request and returns only aggregate acceptance or a redacted error. The Android UI
requires an explicit confirmation dialog before submitting a message and clears
the body after transport acceptance.

## Privacy and safety properties

- Authentication occurs before JSON parsing.
- Request bodies larger than 4096 bytes are rejected.
- Recipients are limited to bounded phone-number-like input; bodies are limited to
  1–2000 bytes and reject NUL bytes.
- Rust `Debug`, API success, validation errors, and transport errors never echo the
  recipient or body.
- Child-process stdout and stderr are discarded.
- Android does not log the request and requires a visible confirmation step.

## Known limitation

`imsg send` accepts recipient and body as command-line arguments. Consequently,
those values can briefly be visible to other same-machine process inspection while
the child runs. This is `DOCUMENTED` from the installed `imsg 0.3.1` CLI contract.
A future direct IPC/library interface should remove this exposure before the Pi is
treated as a multi-user host.

## Evidence

- `VERIFIED_AUTOMATED`: input validation, redacted `Debug`, auth-before-parse,
  aggregate-only success, and mock transport invocation pass Rust tests.
- `VERIFIED_AUTOMATED`: the API-27 Android compose/confirmation client builds and
  its APK signature verifies.
- `VERIFIED_HARDWARE`: one deliberately reviewed Android request was accepted by
  the iPhone MAP transport and the intended recipient confirmed SMS receipt. No
  recipient, body, message handle, or timestamp was retained in project evidence.
- `UNKNOWN`: sent-folder reflection, failure recovery, locked-iPhone behavior,
  MMS, and attachments.

No automated test invokes the real `imsg` sender. A real send always requires a
deliberate human-confirmed hardware procedure.
