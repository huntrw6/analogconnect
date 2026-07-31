# AnalogConnect — Milestone 0 Hardware Test Plan

Date: 2026-07-31

## Overview

This plan tests Bluetooth feasibility between a Raspberry Pi 5 and an iPhone. Tests are ordered from least to most invasive. Stop at any BLOCKED result and report before proceeding.

## Prerequisites

- Raspberry Pi 5 with BlueZ 5.82, PipeWire 1.4.2, WirePlumber 0.5.8
- iPhone (any model supporting Bluetooth MAP, PBAP, HFP)
- iPhone Bluetooth enabled and discoverable
- iPhone unlocked during tests
- Pi connected to same network as iPhone (not required, but helpful for debugging)

## Safety

- Do not jailbreak or modify the iPhone
- Do not install any apps on the iPhone
- Do not share pairing keys or Bluetooth addresses
- All test outputs will redact addresses by default

---

## Test 1 — Pairing

**Test ID**: MH-PAIR-001

**Purpose**: Verify that the Raspberry Pi can pair with the iPhone via Bluetooth.

**Prerequisites**:
- iPhone Bluetooth on and discoverable
- No previous pairing between these devices

**Pi command**:
```bash
bluetoothctl
```

**iPhone action**: None (iPhone should already be discoverable)

**Steps**:
1. On Pi: `scan on`
2. Wait for iPhone to appear in scan results (look for iPhone name)
3. On Pi: `pair <device-address>`
4. On iPhone: Accept pairing request when prompted
5. On iPhone: Tap "Pair" when asked
6. On Pi: `trust <device-address>`
7. On Pi: `info <device-address>` — verify "Paired: yes", "Trusted: yes"
8. On Pi: `quit`

**Expected result**: Device shows as Paired: yes, Trusted: yes, Connected: no (or yes if auto-connect occurs)

**Data that may be exposed**: Device name, Bluetooth address

**Logs collected**: None (manual observation)

**Pass condition**: `bluetoothctl info <device>` shows "Paired: yes" and "Trusted: yes"

**Failure condition**: Pairing fails, timeout, or iPhone rejects pairing

**Cleanup**: `bluetoothctl remove <device-address>` to unpair

**Evidence label**: `VERIFIED_HARDWARE` (after test)

---

## Test 2 — Profile Discovery

**Test ID**: MH-PROF-001

**Purpose**: Verify which Bluetooth profiles the iPhone advertises for this pairing.

**Prerequisites**: Test 1 passed (devices paired)

**Pi command**:
```bash
analogconnect inspect-device --device <MAC>
```

**iPhone action**: None

**Expected result**: Output shows UUID list from device. Look for:
- HFP (0000111e or 0000111f)
- MAP (00001132)
- PBAP (0000112f)
- A2DP (0000110a or 0000110b)

**Data that may be exposed**: Device name, profile UUIDs

**Logs collected**: inspect-device output

**Pass condition**: At least HFP UUID is detected

**Failure condition**: No profiles detected, or device not found

**Cleanup**: None

**Evidence label**: `VERIFIED_HARDWARE` (after test)

---

## Test 3 — Basic MAP Access

**Test ID**: MH-MAP-001

**Purpose**: Test MAP (Message Access Profile) connection to iPhone.

**Prerequisites**:
- Test 1 passed
- imsg installed (`cargo install imsg`)
- iPhone "Show Notifications" enabled in Bluetooth settings for this device

**Pi commands**:
```bash
# Start OBEX daemon if not running
systemctl --user start obex

# Connect via MAP UUID
obexctl
> connect <device-address> 00001132-0000-1000-8000-00805f9b34fb
> quit
```

**iPhone action**: None (may see brief connection indicator)

**Expected result**: obexctl connects successfully, MAP session established

**Data that may be exposed**: Message count, message handles (not message bodies)

**Logs collected**: obexctl output, bluetoothd logs

**Pass condition**: obexctl shows "Connection successful" or MAP session active

**Failure condition**: Connection refused, timeout, or MAP not supported

**Cleanup**: `obexctl > disconnect`

**Evidence label**: `VERIFIED_HARDWARE` (after test)

---

## Test 4 — Basic PBAP Access

**Test ID**: MH-PBAP-001

**Purpose**: Test PBAP (Phonebook Access Profile) connection to iPhone.

**Prerequisites**:
- Test 1 passed
- imsg installed

**Pi commands**:
```bash
# Connect via PBAP UUID
obexctl
> connect <device-address> 0000112f-0000-1000-8000-00805f9b34fb
> quit
```

**iPhone action**: None

**Expected result**: obexctl connects successfully, PBAP session established

**Data that may be exposed**: Contact count, vCard handles (not contact names)

**Logs collected**: obexctl output, bluetoothd logs

**Pass condition**: obexctl shows connection successful

**Failure condition**: Connection refused, timeout, or PBAP not supported

**Cleanup**: `obexctl > disconnect`

**Evidence label**: `VERIFIED_HARDWARE` (after test)

---

## Test 5 — Basic HFP Call Detection

**Test ID**: MH-HFP-001

**Purpose**: Test HFP (Hands-Free Profile) connection and call detection.

**Prerequisites**:
- Test 1 passed
- PipeWire configured for HFP HF role
- Another phone available to call the iPhone

**Pi commands**:
```bash
# Check PipeWire Bluetooth status
wpctl status

# Check BlueZ D-Bus for HFP
busctl tree org.bluez | grep hfp
```

**iPhone action**: Receive an incoming call from another phone

**Expected result**: Pi detects incoming call via HFP, PipeWire shows SCO audio node

**Data that may be exposed**: Call state (incoming/active/ended), phone number format

**Logs collected**: PipeWire logs, bluetoothd logs, kernel Bluetooth messages

**Pass condition**: Pi shows call state change, SCO audio node appears in PipeWire

**Failure condition**: No HFP connection, no call detection, no audio routing

**Cleanup**: Decline/end the test call

**Evidence label**: `VERIFIED_HARDWARE` (after test)

---

## Test 6 — Basic SCO Audio Detection

**Test ID**: MH-SCO-001

**Purpose**: Test bidirectional SCO audio routing during an active call.

**Prerequisites**:
- Test 5 passed (HFP connection active)
- Call in progress

**Pi commands**:
```bash
# Check PipeWire SCO nodes
wpctl status | grep -i sco

# Check audio routing
pw-top
```

**iPhone action**: Stay on the call, speak into iPhone microphone

**Expected result**: PipeWire shows SCO source (microphone) and SCO sink (speaker) nodes. Audio flows bidirectionally.

**Data that may be exposed**: Audio levels (not recorded), codec information

**Logs collected**: PipeWire logs, audio routing info

**Pass condition**: Both SCO source and sink nodes active in PipeWire

**Failure condition**: Only one direction works, no audio, codec negotiation fails

**Cleanup**: End the call

**Evidence label**: `VERIFIED_HARDWARE` (after test)

---

## Future tests (post-Milestone 0)

These tests are beyond Milestone 0 scope but listed for reference:

- Automatic reconnection after Bluetooth interruption
- Multiple profile simultaneous operation
- Call transfer between iPhone and Pi
- Audio quality assessment
- Long-duration stability testing
- Error recovery testing

## Test result template

After each test, record:

```
## Test Result: <TEST-ID>

**Date**: <date>
**Result**: PASS / FAIL / BLOCKED / NOT_SUPPORTED
**Evidence label**: <label>
**Finding**: <what was observed>
**Issues**: <any problems encountered>
**Cleanup performed**: <yes/no>
```
