# ANCS production transport

## Implemented

- `VERIFIED_AUTOMATED`: transport-neutral notification/data-source consumer, body-free attribute
  requests, fragment bounds, UID dedupe, serialized requests, bounded queues, ordered
  subscription lifecycle, reconnect backoff, and explicit notification-action bytes.
- `VERIFIED_AUTOMATED`: NotificationUID and action labels are transient metadata; UID is never a
  conversation identity.
- `VERIFIED_HARDWARE`: the diagnostic direct-GATT probe established ANCS and supplied the group
  identity evidence recorded in `group-chat-detection-report.md`.
- `VERIFIED_AUTOMATED`: analogconnectd owns a BlueZ D-Bus/GATT bearer that discovers the ANCS
  service and characteristics by UUID, subscribes Notification Source before Data Source, writes
  Control Point requests, feeds metadata into immediate MAP synchronization and fail-closed
  correlation, and reconnects with capped backoff. Partial subscription failures also back off.
- `VERIFIED_AUTOMATED`: the bearer never deliberately disconnects the device, so an ANCS failure
  does not intentionally tear down Classic MAP/PBAP/HFP.

## Remaining hardware boundary

`UNKNOWN`: live iPhone ANCS subscription and simultaneous MAP/PBAP/HFP coexistence have not been
tested with the production bearer. The validated release binary has been built but is not yet the
installed service binary; deployment and service restart require the repository's explicit system
change approval gate.

No notification action is invoked automatically. The later Reply experiment requires explicit
user approval after observing a known notification and positive action label.
