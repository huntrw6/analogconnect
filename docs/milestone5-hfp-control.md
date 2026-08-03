# Milestone 5 — HFP Call-Control Software Foundation

## Scope

This slice defines and validates call-control behavior without taking ownership of
the live HFP RFCOMM connection. The production adapter uses PipeWire 1.4.2's
supported `org.pipewire.Telephony` user-bus service, which preserves WirePlumber's
ownership of the verified SLC.

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
- D-Bus object discovery accepts only numeric `/agN` and `/callN` paths, rejects
  zero or multiple gateways, and never reads call identity properties.
- WirePlumber 0.5 exposes gateways through the root ObjectManager but calls only
  through the gateway's oFono-compatible `GetCalls`; discovery ignores every
  returned property value and retains only exact numeric object-map keys.
- The backend reads only each call object's non-private `State` property: answer
  requires `incoming`, DTMF requires `active`, dial requires no live calls, and
  hangup/reject require at least one live call object.
- Authentication and a 1024-byte limit are applied before command JSON parsing.
- Android dialing and hangup require an explicit confirmation dialog.
- PipeWire's Telephony interface does not expose microphone/speaker gain methods;
  the production adapter explicitly rejects those commands instead of guessing.

## Evidence

- `VERIFIED_AUTOMATED`: command validation, redaction, state gating, backend
  failure preservation, AT encoding, and mute restoration tests pass.
- `DOCUMENTED`: PipeWire 1.4.2 source defines `AudioGateway1` methods `Dial`,
  `HangupAll`, and `SendTones`, plus `Call1` methods `Answer` and `Hangup`.
- `VERIFIED_AUTOMATED`: the installed WirePlumber owns `org.pipewire.Telephony`
  and exposes the documented root manager interface.
- `VERIFIED_AUTOMATED`: numeric object discovery, ambiguity rejection, D-Bus
  command mapping, live-state gating, authenticated API behavior, redacted
  failures, and Android API-27 packaging pass automated tests.
- `VERIFIED_HARDWARE`: the corrected live discovery reports incoming and active
  iPhone calls; Pi-originated answer, DTMF `5`, and hangup were accepted and had
  their expected effects without exposing call identity.
- `VERIFIED_HARDWARE`: an active call produced exactly one SCO source/sink pair.
- `FAILED`: one answer/hangup run left the SCO transport active after call state
  returned idle; an HFP-profile disconnect/reconnect restored clean idle state.
- `VERIFIED_AUTOMATED`: read-only snapshots aggregate live call objects into HFP
  and call status with deterministic multi-call precedence and no object paths or
  identity fields in the result.
- `VERIFIED_AUTOMATED`: normal Telephony-service absence maps to disconnected/idle,
  while malformed or ambiguous live state maps to a fail-closed error.
- `VERIFIED_AUTOMATED`: WirePlumber helper calls time out after two seconds and a
  timeout or 1 MiB output-bound error contains neither arguments nor captured output.
- `UNKNOWN`: Pi-originated reject and dial, microphone/speaker gain behavior, and
  human-confirmed DTMF audibility.
