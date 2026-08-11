# Physical call controls

Evidence: `VERIFIED_AUTOMATED` for mapping discovery and state-machine tests;
`UNKNOWN` for real call-key delivery until a controlled hardware call.

Raw hardware capture established that the target phone's green key emits Linux
`KEY_SEND` and its combined red End/Power key emits `KEY_POWER`. This supersedes
unused `ENDCALL` entries in its static MediaTek keylayout.

AnalogConnect therefore applies one state-authoritative contract:

- Incoming: green answers; red rejects.
- Dialing/ringing/active: red hangs up; green is harmless.
- Active only: physical digits, star, and pound send one DTMF command.
- Idle/ending/ended/failed: red is harmless; repeated and key-up events are ignored.
- Idle green opens the AnalogConnect dialer without placing a call.

Live call screens contain no touch targets for answer, reject, hangup, mute,
speaker, or DTMF. Android reserves the actual Call and Power keys before app
dispatch. `scripts/android-call-keys.py` therefore reads their raw events through
the Pi's existing trusted ADB link and forwards only key codes to a receiver
guarded by Android's signature `DUMP` permission. The receiver re-reads backend
call state before every real command and fails closed. A short Power press maps
to End; a held Power press is not forwarded and retains the native power menu.
The accessibility fallback requests no window content.

For persistent operation, install the monitor as
`~/.local/bin/analogconnect-android-call-keys` and enable the supplied user unit
`config/systemd/analogconnect-android-keys.service`. It restarts across ADB/device
reconnects without requiring Android root or modifying the phone keylayout.

`VERIFIED_HARDWARE`: raw green and red codes were captured; short red produced
one forwarded End code, while held red produced none and opened the native Power
off/Restart menu. Real HFP command effects remain pending a controlled call.

Mute and speaker have no safe, intuitive dedicated physical mapping and remain
automatic rather than stealing a system/navigation key.
