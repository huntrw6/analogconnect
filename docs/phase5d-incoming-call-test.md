# Phase 5D — Controlled Incoming-Call Test

## Status: BLOCKED — EnumProfile absent

## Pre-test verification

| Requirement | Result | Evidence |
|---|---|---|
| iPhone Connected=true | ✅ | D-Bus: `org.bluez.Device1.Connected` = true |
| ServicesResolved=true | ✅ | D-Bus: `org.bluez.Device1.ServicesResolved` = true |
| HFP RFCOMM channel 8 connected | ✅ | `/sys/kernel/debug/bluetooth/rfcomm` shows active session, dlci 16, channel 8 |
| HFP service-level connection complete | ✅ | Phase E btmon: full AT negotiation completed (BRSF→BAC→CIND→CMER→CHLD→CLIP→CCWA→CMEE→CLCC all OK) |
| headset-head-unit present or available | ❌ | Phase F: EnumProfile contains only `off` and `audio-gateway` — `headset-head-unit` absent |
| Active Profile is headset-head-unit | ❌ | Phase F: Active Profile is `audio-gateway` (index 65536) |
| HFP transport objects exist | ❌ | Phase F: No HFP transport objects |
| SCO sink/source nodes exist | ❌ | Phase F: No SCO nodes |
| call=0 | ✅ | Phase E btmon: `+CIND: 1,0,0,5,2,0,0` (call=0) |
| callsetup=0 | ✅ | Phase E btmon: `+CIND: 1,0,0,5,2,0,0` (callsetup=0) |
| callheld=0 | ✅ | Phase E btmon: `+CIND: 1,0,0,5,2,0,0` (callheld=0) |

## Classification

`HFP_SLC_NOT_REFLECTED_IN_CONNECTED_PROFILES` — The HFP service-level control negotiation is verified at the HCI level (RFCOMM channel 8 established, full AT negotiation completed, no disconnect observed). However, `headset-head-unit` is absent from the PipeWire EnumProfile. The active Profile parameter is `audio-gateway` (Pi-as-AG), which does not provide HFP hands-free audio.

## Recommended next action

Enable temporary detailed SPA Bluetooth logging for one controlled reconnection to determine why `headset-head-unit` is absent from EnumProfile despite the completed SLC. The exact logging method, risk, and rollback must be shown before execution.
