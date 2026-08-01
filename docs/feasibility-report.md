# AnalogConnect — Feasibility Report

Date: 2026-07-31
Milestone: 0 — iPhone Bluetooth Feasibility

## Capability Matrix

| Capability | Status | Evidence | Test ID | Notes |
|---|---|---|---|---|
| MAP message listing | `VERIFIED_HARDWARE` | imsg list inbox succeeded | MH-MAP-001 | Messages visible with timestamps |
| MAP message retrieval | `VERIFIED_HARDWARE` | imsg get works | MH-MAP-001 | Full message body accessible |
| MAP notifications | NOT TESTED | Daemon mode not tested | — | Need `imsg daemon` test |
| MAP send | NOT TESTED | Not attempted | — | Requires test phone number |
| MAP reply | NOT TESTED | Not attempted | — | Requires test phone number |
| PBAP contact listing | `VERIFIED_HARDWARE` | imsg contacts --list succeeded | MH-PBAP-001 | 456 contacts retrieved |
| PBAP test contact | `VERIFIED_HARDWARE` | Contact names and handles visible | MH-PBAP-001 | |
| PBAP phone number | NOT TESTED | Not attempted | — | Requires --get with handle |
| PBAP E.164 normalization | NOT TESTED | Not attempted | — | |
| HFP call event | UNKNOWN | UUID advertised but not tested | — | Need HFP connection test |
| HFP answer | UNKNOWN | Not tested | — | |
| HFP hangup | UNKNOWN | Not tested | — | |
| HFP dial | UNKNOWN | Not tested | — | |
| HFP DTMF | UNKNOWN | Not tested | — | |
| SCO speaker audio | UNKNOWN | Not tested | — | |
| SCO microphone audio | UNKNOWN | Not tested | — | |
| Profile coexistence | UNKNOWN | Not tested | — | |
| Automatic reconnection | UNKNOWN | Not tested | — | |

## System Readiness

| Component | Status | Version |
|---|---|---|
| Raspberry Pi | Ready | Pi 5 Model B, 16GB RAM |
| OS | Ready | Debian 13 (trixie), aarch64 |
| BlueZ | Ready | 5.82 |
| PipeWire | Ready | 1.4.2 |
| WirePlumber | Ready | 0.5.8 |
| imsg | Ready | 0.3.1 |
| Rust | Ready | 1.97.1 |
| ShellCheck | Ready | 0.10.0 |
| obexd | Not installed | imsg uses own OBEX |

## Paired Device Status

| Property | Value |
|---|---|
| Name | illuminary-cinema |
| Icon | phone |
| Paired | yes |
| Trusted | yes |
| Connected | yes |
| MAP UUID | advertised |
| PBAP UUID | advertised |
| HFP UUID | advertised |

## HFP Registration Analysis

### Remote iPhone UUIDs (from BlueZ Device1)

| UUID | Service | Status |
|------|---------|--------|
| `0000111f` | HFP Audio Gateway | present |
| `0000111e` | HFP Hands-Free | absent (expected — iPhone is not a headset) |
| `00001108` | HSP Headset | absent |
| `00001112` | HSP Audio Gateway | absent |
| `00001132` | MAP | present |
| `0000112f` | PBAP | present |

- `ServicesResolved`: true
- Device form factor: phone
- Bluetooth class: `0x007a020c`

### Local Pi Registration (from D-Bus trace)

- Local HFP HF (`0000111e`) registered at `/Profile/HFPHF` — `VERIFIED_AUTOMATED`
- Local HFP AG (`0000111f`) registered at `/Profile/HFPAG` — `VERIFIED_AUTOMATED`
- Both profiles accepted by BlueZ with no errors

### PipeWire Profile Enumeration

- `headset-head-unit`: **absent** from EnumProfile
- `headset-audio-gateway`: **absent** from EnumProfile
- `audio-gateway`: present (description: "Audio Gateway (A2DP Source & HSP/HFP AG)")
- Active profile: `audio-gateway`
- `bluez5.auto-connect`: `[ hfp_hf hsp_hs a2dp_sink hfp_ag hsp_ag a2dp_source ]`

### HFP ConnectProfile Test Results

`VERIFIED_AUTOMATED`: BlueZ `ConnectProfile(0000111f)` succeeds — "Connection successful"
`VERIFIED_AUTOMATED`: RFCOMM connection established — btmon confirms SABM/UA frames
`VERIFIED_AUTOMATED`: HFP AT negotiation completes successfully:
- `AT+BRSF=695` → `+BRSF:4079` → OK
- `AT+BAC=1,2,3` → OK (codec negotiation)
- `AT+CIND=?` → OK (indicator mapping)
- `AT+CIND?` → `+CIND: 1,0,0,3` → OK
- `AT+CMER=3,0,0,1` → OK
- `AT+CHLD=?` → `+CHLD: (0,1,1x...)` → OK
- `AT+CLIP=1` → OK

`VERIFIED_AUTOMATED`: `/Profile/HFPHF/NewConnection` IS invoked by BlueZ — WirePlumber received RFCOMM fd (inode 1091667)
`VERIFIED_AUTOMATED`: `bluez5.hfphsp-backend` NOT set in default WirePlumber config
`VERIFIED_AUTOMATED`: `bluez5.profile = "off"` — headset-head-unit not activated
`VERIFIED_AUTOMATED`: `headset-head-unit` does not appear in EnumProfile after connection
`VERIFIED_AUTOMATED`: No HFP transport or SCO objects created

### Failure Layer (final)

`PIPEWIRE_HFP_BACKEND_NOT_CONFIGURED` — BlueZ correctly invokes `/Profile/HFPHF/NewConnection` and delivers the RFCOMM fd to WirePlumber. But WirePlumber's spa-bluez5 does not activate the native HFP backend because `bluez5.hfphsp-backend = native` is NOT set in the default config. Without this, WirePlumber receives the RFCOMM fd but cannot process it into a `headset-head-unit` profile. The previous classification of `BLUEZ_PROFILE_MATCHING_FAILURE` was incorrect — the callback IS delivered. The fix is to enable `bluez5.hfphsp-backend = native` in `/etc/wireplumber/wireplumber.conf.d/51-bluez-hfp.conf`.

### Previous Configuration Issues (resolved)

- `~/.config/wireplumber/wireplumber.conf.d/51-bluez-hfp.conf.disabled` — quoted-string `bluez5.roles` syntax
- `~/.config/wireplumber/main.lua.d/51-bluez-hfp.lua.disabled` — WirePlumber 0.4 Lua format
- `/etc/wireplumber/wireplumber.conf.d/51-bluez-hfp.conf.disabled` — quoted-string `bluez5.roles` syntax
- `~/.config/wireplumber/wireplumber.conf.d/90-analogconnect-hfp-test.conf` — removed

## Blockers

1. ~~iPhone not trusted~~ — RESOLVED
2. ~~obexd not installed~~ — imsg uses own OBEX, works fine
3. ~~Bluetooth group~~ — active in current session
4. **`bluez5.hfphsp-backend` not configured** — WirePlumber receives RFCOMM fd but doesn't activate HFP backend

## Key Findings

1. MAP works via imsg without needing bluez-obexd
2. PBAP works via imsg without needing bluez-obexd
3. iOS requires manual permission grant (Show Notifications, Share Contacts) before first connection
4. iOS permission prompt appears after first connection attempt
5. imsg connects via its own OBEX implementation (bluer crate)
6. MAP channel: RFCOMM 2, PBAP channel: RFCOMM 13
7. iPhone advertises HFP AG (`111f`) — correct for Pi acting as hands-free
8. WirePlumber native backend registers both HF and AG with BlueZ — `VERIFIED_AUTOMATED`
9. PipeWire EnumProfile does not expose `headset-head-unit` — only `audio-gateway` (Pi-as-AG)
10. Custom WirePlumber config fragments with quoted-string `bluez5.roles` are rejected
11. `override.bluez5.roles = [ hfp_hf ]` had no effect on EnumProfile
12. Explicit `ConnectProfile(111f)` succeeds — HFP RFCOMM connection established
13. HFP AT negotiation completes successfully — all commands return OK
14. NewConnection IS delivered at `/Profile/HFPHF` — WirePlumber receives RFCOMM fd — `VERIFIED_AUTOMATED`
15. `bluez5.hfphsp-backend` NOT set in default config — spa-bluez5 doesn't activate HFP backend
16. HFP RFCOMM and AT negotiation succeed — connection is established at Bluetooth level
17. MAP and PBAP remain functional after HFP connection attempt
18. Fix: enable `bluez5.hfphsp-backend = native` in `/etc/wireplumber/wireplumber.conf.d/`
