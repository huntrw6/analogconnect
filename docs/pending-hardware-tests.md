# Pending hardware tests

This checklist contains only tests that require the operator or physical phones.
No listed feature should be treated as hardware-verified until its result is added
to `docs/current-state.md` with an evidence label.

## Sustained call audio

- Run a call for at least five minutes with the latest latency trimmer.
- Observe `buffer`, `holds`, `late`, `overflow`, `trims`, and `pace` around one,
  three, and five minutes.
- Confirm clarity, popping, and subjective delay through earpiece and speakerphone.

## Plug-in-and-use restart

- With both phones available, reboot the Pi without changing enrollment.
- Confirm the daemon becomes healthy without an interactive Pi login.
- Confirm Android discovers the newly assigned address and authenticates with its
  existing certificate pin and token.
- Confirm the trusted iPhone reconnects HFP, MAP, and PBAP without pairing again.
- Place one call and send one deliberate message after recovery.

## Experimental Android Phone integration

- Confirm the app preserves the saved enrollment after the settings-store migration.
- Enable **Register AnalogBridge calling account**, open calling-account settings,
  and report whether Android shows **AnalogBridge iPhone**.
- Do not make it the default for all calls until ordinary cellular and emergency
  calling behavior is confirmed unchanged.
- From a deliberately selected test contact, choose the AnalogBridge account and
  confirm the iPhone places the intended call.
- During that call, verify dialing/active/ended UI, earpiece, speakerphone, DTMF,
  and hang-up.
- Disable the switch and confirm the account disappears without affecting contacts
  or the normal cellular calling account.

## Not ready for hardware testing

- Incoming calls through Android Telecom.
- Native Contacts synchronization; its account and sync boundaries are packaged
  but intentionally refuse activation.
- AnalogBridge conversation inbox and message notifications.
