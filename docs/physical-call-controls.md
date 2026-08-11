# Physical call controls

Evidence: `VERIFIED_AUTOMATED` for mapping discovery and state-machine tests;
`UNKNOWN` for real call-key delivery until a controlled hardware call.

The target Android 8.1 phone exposes dedicated native call keys. Its installed
MediaTek keylayouts map green keys (scan 231/61) to Android `CALL`, red keys
(107/62) to `ENDCALL`, and keyboard scan 227/523 to `STAR`/`POUND`.

AnalogConnect therefore applies one state-authoritative contract:

- Incoming: green answers; red rejects.
- Dialing/ringing/active: red hangs up; green is harmless.
- Active only: physical digits, star, and pound send one DTMF command.
- Idle/ending/ended/failed: red is harmless; repeated and key-up events are ignored.
- Idle green opens the AnalogConnect dialer without placing a call.

Live call screens contain no touch targets for answer, reject, hangup, mute,
speaker, or DTMF. A key-filter accessibility service is used because Android
routes native Call/End keys before ordinary activity dispatch. It requests no
window content and filters only Call/End plus active-call DTMF. The service must
remain enabled under Android Accessibility. Proximity screen-off is supplemental.

Mute and speaker have no safe, intuitive dedicated physical mapping and remain
automatic rather than stealing a system/navigation key.
