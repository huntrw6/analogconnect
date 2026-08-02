# Milestone 5 — HFP Call-Control Software Foundation

## Scope

This slice defines and validates call-control behavior without taking ownership of
the live HFP RFCOMM connection. WirePlumber currently owns the verified SLC, so a
live adapter must not be selected until its sharing/control interface is confirmed.

## Command model

- Answer an incoming call: `ATA`
- Reject or hang up: `AT+CHUP`
- Dial a validated target: `ATD...;`
- Send a validated DTMF tone: `AT+VTS=...`
- Set speaker gain: `AT+VGS=0..15`
- Set microphone gain: `AT+VGM=0..15`
- Mute microphone: send gain zero, remember the previous gain, and restore it on unmute

## Safety properties

- Dial targets accept only dial-string characters and have a bounded length.
- DTMF accepts only the HFP tone domain.
- Gain is bounded to the HFP range.
- Dial targets and DTMF values are redacted from `Debug` output.
- Commands invalid for the current call state never reach the backend.
- Local call state changes only after transport success.
- Transport errors never include AT command text.
- No mutation endpoint is exposed before Android-client authentication exists.

## Evidence

- `VERIFIED_AUTOMATED`: command validation, redaction, state gating, backend
  failure preservation, AT encoding, and mute restoration tests pass.
- `UNKNOWN`: the correct production control seam while WirePlumber owns RFCOMM.
- `UNKNOWN`: command acceptance and behavior with the real iPhone.
