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

- Local HFP HF (`0000111e`) registered at `/Profile/HFPHF` — `VERIFIED_AUTOMATED` (old WirePlumber instance only)
- Local HFP AG (`0000111f`) registered at `/Profile/HFPAG` — `VERIFIED_AUTOMATED` (old WirePlumber instance only)
- After bluetoothd + wireplumber restart: 0 Profile1 interfaces registered — `VERIFIED_AUTOMATED`
- WirePlumber registers A2DP MediaEndpoints but NOT HFP Profile1 interfaces — `VERIFIED_AUTOMATED`

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

### HFP Isolation Test Results

`VERIFIED_AUTOMATED`: `bluez5.roles=[hfp_hf]` isolation fragment causes Bluetooth device to disappear from PipeWire entirely — no bluez5 device in `wpctl status` or `pw-cli ls Device`.

`VERIFIED_AUTOMATED`: After bluetoothd + WirePlumber restart, 0 Profile1 interfaces registered. WirePlumber only registers A2DP MediaEndpoints (ldac, aptx_hd, aptx, sbc, etc.) but NOT HFP Profile1 interfaces.

`VERIFIED_AUTOMATED`: `RegisterProfile(111e)` fails with "UUID already registered" — stale registration persists across bluetoothd restart. BlueZ internal state retains the registration even after process restart.

`VERIFIED_AUTOMATED`: D-Bus `method_return` from WirePlumber to bluetoothd rejected — "0 matched rules". `requested_reply="0"` indicates D-Bus daemon does not recognize this as a reply to an incoming method call.

### Failure Layer

`HFP_CONTROL_CONNECTED_PIPEWIRE_AUDIO_PROFILE_MISSING` — BlueZ correctly invokes `/Profile/HFPHF/NewConnection` and delivers the RFCOMM fd to WirePlumber. HFP AT negotiation completes successfully (Pi sends HF commands, iPhone responds as AG). But `headset-head-unit` does not appear in PipeWire EnumProfile and no HFP transport is created.

#### Superseded classifications (do not cite)

- ~~`PIPEWIRE_HFP_BACKEND_NOT_CONFIGURED`~~ — withdrawn. Based on (1) malformed fragment syntax (`["bluez5.hfphsp-backend"]` Lua-table key syntax instead of SPA-JSON) and (2) the incorrect assumption that the native backend was not running. Successful AT negotiation indicates the native backend was already active. The official default for `bluez5.hfphsp-backend` is already `native`. The malformed fragment test did not prove the hypothesis.
- ~~`BLUEZ_PROFILE_MATCHING_FAILURE`~~ — withdrawn. The NewConnection callback IS delivered at the correct path.

#### Neutral classification

`HFP_CONTROL_CONNECTED_PIPEWIRE_AUDIO_PROFILE_MISSING` — HFP Hands-Free service-level connection succeeds but `headset-head-unit` is absent from PipeWire EnumProfile. No HFP transport or SCO nodes are created. The failure could be at the profile construction layer, transport creation layer, or the expectation about profile availability timing.

### Previous Configuration Issues (resolved)

- `~/.config/wireplumber/wireplumber.conf.d/51-bluez-hfp.conf.disabled` — quoted-string `bluez5.roles` syntax
- `~/.config/wireplumber/main.lua.d/51-bluez-hfp.lua.disabled` — WirePlumber 0.4 Lua format
- `/etc/wireplumber/wireplumber.conf.d/51-bluez-hfp.conf.disabled` — quoted-string `bluez5.roles` syntax
- `~/.config/wireplumber/wireplumber.conf.d/90-analogconnect-hfp-test.conf` — removed
- `/etc/wireplumber/wireplumber.conf.d/51-bluez-hfp.conf` — malformed (Lua-table key syntax). Removed → `.invalid-disabled`.
- `~/.config/wireplumber/wireplumber.conf.d/90-analogconnect-hfp-isolation.conf` — caused device disappearance. Removed.

## Blockers

1. ~~iPhone not trusted~~ — RESOLVED
2. ~~obexd not installed~~ — imsg uses own OBEX, works fine
3. ~~Bluetooth group~~ — active in current session
4. ~~`bluez5.hfphsp-backend` not configured~~ — SUPERSCEEDED. Default is already `native`.
5. **WirePlumber does NOT register HFP Profile1 interfaces after restart** — only A2DP MediaEndpoints registered. `VERIFIED_AUTOMATED`.
6. **Stale BlueZ UUID `111e` registration** — persists across bluetoothd restart, blocks manual re-registration. `VERIFIED_AUTOMATED`.
7. **`bluez5.roles=[hfp_hf]` causes device disappearance** — cannot use isolation fragment for testing. `VERIFIED_AUTOMATED`.
8. **D-Bus policy rejects WirePlumber method_returns** — "0 matched rules" for method_return to bluetoothd. `VERIFIED_AUTOMATED`.

## Key Findings

1. MAP works via imsg without needing bluez-obexd
2. PBAP works via imsg without needing bluez-obexd
3. iOS requires manual permission grant (Show Notifications, Share Contacts) before first connection
4. iOS permission prompt appears after first connection attempt
5. imsg connects via its own OBEX implementation (bluer crate)
6. MAP channel: RFCOMM 2, PBAP channel: RFCOMM 13
7. iPhone advertises HFP AG (`111f`) — correct for Pi acting as hands-free
8. PipeWire EnumProfile does not expose `headset-head-unit` — only `audio-gateway` (Pi-as-AG)
9. Explicit `ConnectProfile(111f)` succeeds — HFP RFCOMM connection established
10. HFP AT negotiation completes successfully — all commands return OK
11. NewConnection IS delivered at `/Profile/HFPHF` — WirePlumber receives RFCOMM fd — `VERIFIED_AUTOMATED` (old instance only)
12. After clean restart: WirePlumber registers A2DP MediaEndpoints but NOT HFP Profile1 interfaces — `VERIFIED_AUTOMATED`
13. After clean restart: 0 Profile1 interfaces in ObjectManager — `VERIFIED_AUTOMATED`
14. `bluez5.roles=[hfp_hf]` causes Bluetooth device to disappear from PipeWire entirely — `VERIFIED_AUTOMATED`
15. Stale UUID `111e` registration persists across bluetoothd restart — `VERIFIED_AUTOMATED`
16. D-Bus rejects WirePlumber method_returns to bluetoothd — "0 matched rules" — `VERIFIED_AUTOMATED`
17. Malformed system fragment removed — was using Lua-table key syntax in SPA-JSON context
18. `HFP_CONTROL_CONNECTED_PIPEWIRE_AUDIO_PROFILE_MISSING` — current neutral classification
19. Previous `PIPEWIRE_HFP_BACKEND_NOT_CONFIGURED` diagnosis withdrawn — default is already `native`
20. Native backend and `hfp_hf` role are WirePlumber 0.5 defaults — no explicit config needed
