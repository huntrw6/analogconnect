# ANCS production transport

## Implemented

- `VERIFIED_AUTOMATED`: transport-neutral notification/data-source consumer, body-free attribute
  requests, fragment bounds, UID dedupe, serialized requests, bounded queues, ordered
  subscription lifecycle, reconnect backoff, and explicit notification-action bytes.
- `VERIFIED_AUTOMATED`: NotificationUID and action labels are transient metadata; UID is never a
  conversation identity.
- `VERIFIED_HARDWARE`: the diagnostic direct-GATT probe established ANCS and supplied the group
  identity evidence recorded in `group-chat-detection-report.md`.

## Remaining production boundary

`UNKNOWN`: no daemon-owned BlueZ bearer currently executes the supervisor commands. Cached BlueZ
ANCS characteristics are present on this Pi, but unattended work did not connect the iPhone LE
bearer because doing so can disrupt the hardware-verified Classic MAP/PBAP/HFP bearer. The next
implementation must discover characteristics by UUID, start both notifications, write the control
point, stream property changes into `AncsSupervisor`, and restore Classic coexistence on shutdown.

No notification action is invoked automatically. The later Reply experiment requires explicit
user approval after observing a known notification and positive action label.
