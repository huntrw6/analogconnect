# Phase H: Completely Captured HFP Profile Cycle

## Summary

**Classification: `HFP_SLC_COMPLETED_BUT_NO_CODEC_NEGOTIATION`**

The AT Service Level Connection (SLC) completed successfully with full WirePlumber trace logging. All AT commands were exchanged and acknowledged. However, the iPhone AG did not initiate codec negotiation (`AT+BCS`), so `rfcomm_new_transport()` was never called and no HFP transport was created.

## Evidence Labels

| Finding | Label |
|---------|-------|
| AT SLC completed (all AT commands exchanged) | `VERIFIED_AUTOMATED` |
| WirePlumber trace log captured full AT exchange | `VERIFIED_AUTOMATED` |
| iPhone BRSF:4079 with codec negotiation bit set | `VERIFIED_AUTOMATED` |
| No `rfcomm_new_transport` called | `VERIFIED_AUTOMATED` |
| No HFP transport in PipeWire | `VERIFIED_AUTOMATED` |
| iPhone AG defers AT+BCS until call audio needed | `INFERRED` |
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

### 2. No Codec Negotiation Initiated

After AT+CLCC → OK, the iPhone AG did NOT send `AT+BCS=n` to select a codec. This is the critical gap:

- The Pi (HF) sent `AT+BAC=1,2,3` indicating support for CVSD, mSBC, and LC3
- The iPhone (AG) acknowledged with OK
- But the iPhone never followed up with `AT+BCS` to select a codec
- Therefore `rfcomm_new_transport()` was never called
- Therefore no HFP transport was created in PipeWire

### 3. Why rfcomm_new_transport Was Never Called

In `backend-native.c`, `rfcomm_new_transport()` is called from `rfcomm_hfp_hf()` (line 1941) when:
- The iPhone sends `+BCS:n` (codec selection response)
- The Pi sends `AT+BCS=n` in response

Neither event occurred. The iPhone deferred codec negotiation.

### 4. PipeWire Profile State Unchanged

- EnumProfile: `off` + `audio-gateway` (no `headset-head-unit` — correct per source analysis)
- Active Profile: `audio-gateway`
- No SCO nodes, no HFP transport nodes
- RFCOMM DLC active: DLCI 16, MTU 1015, DCNT 26 bytes sent, 245 bytes received

### 5. iPhone AG Behavior

The iPhone BRSF:4079 indicates codec negotiation is supported (bit 9 set). The `device_supports_codec` confirmed `has msbc/esco transport`. But the iPhone chose not to initiate codec negotiation during idle SLC establishment. This appears to be by design — the iPhone defers SCO audio setup until call audio is actually needed.

## Implications

1. **SCO is intentionally deferred**: The iPhone does not set up SCO audio during idle SLC establishment
2. **Call trigger required**: Codec negotiation (and therefore SCO setup) likely requires an actual call event (incoming/outgoing)
3. **No WirePlumber bug**: The WirePlumber code correctly handles `+BCS` when received; the issue is that the iPhone never sends it
4. **`audio-gateway` profile is correct**: The Pi acts as HF connecting to iPhone's AG profile

## btmon Note

btmon was started but the btsnoop file was only 400 bytes (header only). The btmon process died before capturing RFCOMM traffic. The WirePlumber system journal (`journalctl _PID=<WP_PID>`) provided the complete AT exchange trace instead.

## Files

- `test-results/phaseH-slc-capture/at-slc-sequence.txt` — Clean AT exchange from WirePlumber trace
- `test-results/phaseH-slc-capture/wireplumber-bt-trace.txt` — Full bluetooth-related WirePlumber journal
- `test-results/phaseH-slc-capture/enum-profile-before.txt` — Initial EnumProfile
- `test-results/phaseH-slc-capture/profile-before.txt` — Initial Active Profile
- `test-results/phaseH-slc-capture/rfcomm-before.txt` — Initial RFCOMM state
- `test-results/phaseH-slc-capture/timestamps.txt` — Phase timestamps
