# Phase H: Completely Captured HFP Profile Cycle

## Summary

**Classification: `HFP_CONTROL_CHANNEL_READY_FOR_ACTIVE_CALL_TEST`**

The AT Service Level Connection (SLC) completed successfully with full WirePlumber trace logging. All AT commands were exchanged and acknowledged. The HFP control plane is fully established and ready for an active-call test.

The Service Level Connection and Audio Connection are separate procedures. No Audio Connection was requested during this idle test. The absence of unsolicited codec-selection responses during idle is expected behavior — codec negotiation and SCO are initiated only when audio is needed.

## Evidence Labels

| Finding | Label |
|---------|-------|
| Fresh NewConnection reached current WirePlumber | `VERIFIED_AUTOMATED` |
| WirePlumber accepted the RFCOMM descriptor | `VERIFIED_AUTOMATED` |
| AT+BRSF completed | `VERIFIED_AUTOMATED` |
| AT+BAC completed | `VERIFIED_AUTOMATED` |
| AT+CIND=? completed | `VERIFIED_AUTOMATED` |
| AT+CIND? completed | `VERIFIED_AUTOMATED` |
| AT+CMER completed | `VERIFIED_AUTOMATED` |
| AT+CHLD completed | `VERIFIED_AUTOMATED` |
| AT+CLIP completed | `VERIFIED_AUTOMATED` |
| AT+CCWA completed | `VERIFIED_AUTOMATED` |
| AT+CMEE completed | `VERIFIED_AUTOMATED` |
| AT+CLCC completed | `VERIFIED_AUTOMATED` |
| call=0, callsetup=0, callheld=0 | `VERIFIED_AUTOMATED` |
| RFCOMM remained connected | `VERIFIED_AUTOMATED` |
| No HFP transport in post-test PipeWire state | `VERIFIED_AUTOMATED` |
| No SCO source or sink in post-test PipeWire state | `VERIFIED_AUTOMATED` |
| `headset-head-unit` not expected for Pi-as-HF | `DOCUMENTED` |

## AT SLC Sequence (from WirePlumber trace log)

All at `Aug 01 14:35:22`:

```
RFCOMM >> AT+BRSF=695          → +BRSF:4079, OK
RFCOMM >> AT+BAC=1,2,3          → OK
RFCOMM >> AT+CIND=?             → +CIND: ("service",(0-1)),("call",(0-1)),("callsetup",(0-3)),("battchg",(0-5)),("signal",(0-5)),("roam",(0-1)),("callheld",(0-2)), OK
RFCOMM >> AT+CIND?              → +CIND: 1,0,0,4,2,0,0, OK
RFCOMM >> AT+CMER=3,0,0,1       → OK
RFCOMM >> AT+CHLD=?             → +CHLD: (0,1,1x,2,2x,3), OK
                                 → telephony_ag_register: registered AudioGateway
RFCOMM >> AT+CLIP=1             → OK
RFCOMM >> AT+CCWA=1             → OK
RFCOMM >> AT+CMEE=1             → OK
RFCOMM >> AT+CLCC               → OK
```

## Key Findings

### 1. AT SLC Completed Successfully

The full AT SLC sequence was captured with WirePlumber trace logging (`wpctl set-log-level T`). Every AT command received a valid response from the iPhone AG. `telephony_ag_register` was called after AT+CHLD completed.

### 2. HFP Control Plane Fully Established

The following were all verified:
- `Connected=true`
- `ServicesResolved=true`
- Current WirePlumber owns the HFP RFCOMM connection
- SLC complete
- Call indicators synchronized (call=0, callsetup=0, callheld=0)
- RFCOMM alive
- `audio-gateway` profile present

### 3. No Audio Connection Requested

The Service Level Connection and Audio Connection are separate procedures. No Audio Connection was requested during this idle test. The AG may initiate codec setup (`+BCS:<codec>`) without receiving `AT+BCC`. Either the AG or the HF can trigger codec selection — both are valid per the HFP specification.

The absence of unsolicited codec-selection responses during idle is expected behavior. Codec negotiation and SCO/eSCO are initiated only when audio is actually needed (e.g., during an incoming or outgoing call).

### 4. PipeWire Profile State

- EnumProfile: `off` + `audio-gateway` (no `headset-head-unit` — correct per source analysis)
- Active Profile: `audio-gateway`
- No SCO nodes, no HFP transport nodes in post-test PipeWire state
- RFCOMM DLC active: DLCI 16, MTU 1015, DCNT 26 bytes sent, 245 bytes received

### 5. What May Appear During Active Call

Do not require before the call:
- `headset-head-unit` — may appear only when a remote device connects to our AG profile
- An HFP audio transport — may be created only when audio is requested
- SCO nodes — may appear only when audio is routed to the Pi
- `+BCS` — may be sent only when the AG initiates codec selection

## Implications

1. **Control plane ready**: The HFP control channel is fully established and ready for an active-call test
2. **Audio connection separate**: Codec negotiation and SCO setup are separate from the SLC and are triggered by call events
3. **No WirePlumber bug**: The WirePlumber code correctly handles `+BCS` when received; the AG sends it when audio is needed
4. **`audio-gateway` profile is correct**: The Pi acts as HF connecting to iPhone's AG profile

## btmon Note

btmon was started but the btsnoop file was only 400 bytes (header only). The btmon process died before capturing RFCOMM traffic. The WirePlumber system journal (`journalctl _PID=<WP_PID>`) provided the complete AT exchange trace instead.

The captured logs did not contain evidence of transport creation, but log-text absence does not prove that `rfcomm_new_transport()` never executed unless the exact installed source has an unconditional log statement inside every invocation.

## Files

- `test-results/phaseH-slc-capture/at-slc-sequence.txt` — Clean AT exchange from WirePlumber trace
- `test-results/phaseH-slc-capture/wireplumber-bt-trace.txt` — Full bluetooth-related WirePlumber journal
- `test-results/phaseH-slc-capture/enum-profile-before.txt` — Initial EnumProfile
- `test-results/phaseH-slc-capture/profile-before.txt` — Initial Active Profile
- `test-results/phaseH-slc-capture/rfcomm-before.txt` — Initial RFCOMM state
- `test-results/phaseH-slc-capture/timestamps.txt` — Phase timestamps
