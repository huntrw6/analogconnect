# Phase I — Active Incoming Call and SCO Audio Test

**Classification: HFP_SCO_BIDIRECTIONAL_PACKET_FLOW_VERIFIED** `VERIFIED_AUTOMATED`

**Date:** 2026-08-01
**Git:** `00c2c12`

## Summary

Active incoming call transport test completed successfully. iPhone initiated codec negotiation (`+BCS:2`), WirePlumber confirmed mSBC (`AT+BCS=2`), an eSCO connection was established, and SCO packets were observed in both directions. Call indicators transitioned correctly, SCO cleanly tore down after hangup, RFCOMM remained connected, and MAP/PBAP commands worked post-call. No manual listening result was recorded, so intelligible audio in either direction remains `UNKNOWN`.

## Test Sequence

### I1: Setup
- Created `test-results/phaseI-active-call/`
- `wpctl set-log-level T` enabled trace logging
- Monitors started: btmon (btsnoop + stdout), pw-mon, dbus-monitor

### I2: Call Events

#### First Call (incoming, brief)
| Time | Event | Source |
|------|-------|--------|
| 15:04:04 | `+CIEV: 3,1` (callsetup=1 — incoming call) | RFCOMM |
| 15:04:04 | `AT+CLCC` sent | WirePlumber |
| 15:04:04 | `+CLCC: 1,1,4,0,0` (state=4, incoming) | RFCOMM |
| 15:04:04 | `+BCS:2` (iPhone selects mSBC) | RFCOMM |
| 15:04:04 | `AT+BCS=2` → OK (codec confirmed) | WirePlumber |
| 15:04:04 | HCI Connect Request (eSCO) → Accept → Complete | btmon |
| 15:04:04 | `AT+VGS=10` → OK (speaker volume) | WirePlumber |
| 15:04:05 | `AT+VGM=15` → OK (mic volume) | WirePlumber |
| 15:04:04–15:05:43 | SCO Data RX/TX flowing (Handle 6, dlen 60) | btmon |
| 15:05:43 | SCO transport stopped and released | WirePlumber |

#### Second Call (incoming, answered, ended)
| Time | Event | Source |
|------|-------|--------|
| 15:05:52 | `+CIEV: 3,1` (callsetup=1 — incoming call) | RFCOMM |
| 15:05:52 | `AT+CLCC` → `+CLCC: 1,1,4,0,0` (state=4, incoming) | WirePlumber |
| 15:05:52 | `telephony_call_register: /org/pipewire/Telephony/ag1/call1` | WirePlumber |
| 15:06:00 | AG indicator update: `call = 1` (answered) | WirePlumber |
| 15:06:00 | AG indicator update: `callsetup = 0` (setup complete) | WirePlumber |
| 15:06:12 | `+CIEV: 2,0` (call=0 — ended) | RFCOMM |
| 15:06:12 | `telephony_call_unregister: removing Call: /org/pipewire/Telephony/ag1/call1` | WirePlumber |
| 15:06:12 | `AT+CLCC` sent (post-hangup query) | WirePlumber |
| 15:06:14 | `sco-source: transport stop, do_stop` | WirePlumber |
| 15:06:14 | `sco-sink: transport stop, do_stop` | WirePlumber |
| 15:06:14 | Transport released | WirePlumber |

### I3: Post-Hangup Verification

| Item | Result |
|------|--------|
| RFCOMM DLC | Local and remote addresses redacted; DLCI 16, MTU 1015 — alive |
| HCI connections | ACL + LE only (no SCO) |
| SCO debugfs | Empty |
| PipeWire profile | `off` (expected — no active HFP transport) |
| MAP folders | `inbox, sent, outbox, deleted` (exit 0) |
| MAP inbox list | Working (exit 0) |
| PBAP contacts | Working (exit 0) |

## Decision Table

| Criterion | Status | Evidence |
|-----------|--------|----------|
| iPhone sends `+BCS` (codec negotiation) | **YES** | `+BCS:2` received at 15:04:04 |
| WirePlumber confirms with `AT+BCS` | **YES** | `AT+BCS=2` sent at 15:04:04 |
| eSCO connection established | **YES** | `HCI Connect Request → Accept → Synchronous Connect Complete` |
| Bidirectional SCO data flowing | **YES** | SCO Data RX + TX on Handle 6 |
| Call indicators transition | **YES** | `callsetup=1 → call=1 → callsetup=0 → call=0` |
| SCO teardown after hangup | **YES** | `transport_stop`, `do_stop`, transport released |
| RFCOMM survives hangup | **YES** | DLC still alive post-call |
| MAP works post-call | **YES** | Folders and inbox listed (exit 0) |
| PBAP works post-call | **YES** | Contacts pulled (exit 0) |

## Classification

**HFP_SCO_BIDIRECTIONAL_PACKET_FLOW_VERIFIED** — All packet-level criteria met.

- iPhone initiates codec negotiation with `+BCS:<codec>`
- WirePlumber (Pi as HF) confirms with `AT+BCS=<codec>`
- eSCO established with mSBC codec
- SCO packets flow in both directions during the call
- SCO cleanly torn down after hangup
- RFCOMM retained for MAP/PBAP

## Implications for Milestone 0

Phase I proves the Raspberry Pi can:
1. Receive HFP call indicators from iPhone
2. Participate in codec negotiation (HF role)
3. Establish eSCO audio connection
4. Exchange SCO packets in both directions
5. Cleanly tear down audio after hangup
6. Retain RFCOMM for data services (MAP/PBAP)

**Milestone 0 established feasibility for all four Bluetooth areas:**
- MAP message access: `VERIFIED_HARDWARE`
- PBAP contact access: `VERIFIED_HARDWARE`
- HFP call indicators and SLC: `VERIFIED_AUTOMATED` (Phase E, H, I)
- HFP codec, eSCO, and bidirectional SCO packets: `VERIFIED_AUTOMATED` (Phase I)
- Human-confirmed intelligible call audio: `UNKNOWN`
