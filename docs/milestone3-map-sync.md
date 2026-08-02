# Milestone 3 — MAP Synchronization Orchestration

## Architecture decision

AnalogConnect delegates MAP persistence and protocol cursors to `imsg` rather than
creating a second message database. The installed `imsg` 0.3.1 implementation
already provides encrypted storage, incremental per-folder cursors, outbox state,
and broker notification events.

AnalogConnect owns only orchestration and privacy-safe aggregate health:

```text
imsg broker Watch event ----> relevant event? ----> imsg sync --folder inbox
          |                         |
          | silence timeout         +---- no payload logging
          v
bounded polling fallback ----------+
          |
          v
aggregate state/counters API
```

## Policy

- Start in polling mode; notification support is not assumed.
- Any broker event proves notification liveness.
- New, shifted, deleted, read-state, and delivery-state events trigger sync.
- Memory-state and unknown events refresh liveness but do not trigger sync.
- Notification silence returns the scheduler to polling mode.
- The imsg command adapter suppresses command output and exposes redacted errors.
- The API never returns message addresses, handles, bodies, or thread content.

## Evidence

- `VERIFIED_AUTOMATED`: scheduler, liveness, fallback, coordinator, backoff, and
  aggregate API behavior are covered by unit/API tests.
- `DOCUMENTED`: imsg 0.3.1 IPC types and store source define Watch events,
  encrypted persistence, folder cursors, and sync behavior.
- `UNKNOWN`: real-iPhone notification delivery and recovery behavior.
