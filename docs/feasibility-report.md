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

- Local HFP HF (`0000111e`) registered at `/Profile/HFPHF` — `VERIFIED_AUTOMATED` (earlier WirePlumber instance only)
- Local HFP AG (`0000111f`) registered at `/Profile/HFPAG` — `VERIFIED_AUTOMATED` (earlier WirePlumber instance only)
- Current WirePlumber HFP registration status — `UNKNOWN` (requires D-Bus trace retest with correct methodology)

### PipeWire Profile Enumeration

- `headset-head-unit`: **absent** from EnumProfile
- `headset-audio-gateway`: **absent** from EnumProfile
- `audio-gateway`: present (description: "Audio Gateway (A2DP Source & HSP/HFP AG)")
- Active profile: `audio-gateway`
- `bluez5.auto-connect`: `[ hfp_hf hsp_hs a2dp_sink hfp_ag hsp_ag a2dp_source ]`

### HFP ConnectProfile Test Results

`VERIFIED_AUTOMATED`: BlueZ `ConnectProfile(0000111f)` returns success — D-Bus call returned without error
`VERIFIED_AUTOMATED`: RFCOMM connection established — btmon confirms SABM/UA frames (Phase 7d, 4b)
`VERIFIED_AUTOMATED`: HFP AT negotiation completes successfully (Phase 7d, 4b):
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

`INCONCLUSIVE`: `bluez5.roles=[hfp_hf]` isolation fragment caused no PipeWire device to appear before a verified fresh HFP connection was completed. Device absence is expected behavior when A2DP roles are disabled and no HFP connection exists, not proof of configuration failure.

`UNKNOWN`: Whether the current WirePlumber process registers HFP Profile1 interfaces. Previous test used invalid ObjectManager inspection on the wrong D-Bus service.

`UNKNOWN`: Whether a manual `RegisterProfile(111e)` failure was due to stale registration or the expected behavior of a one-shot busctl call (registration removed when caller exits).

### Failure Layer

`HFP_CURRENT_REGISTRATION_AND_CONNECT_STATE_UNRESOLVED` — Whether the current WirePlumber process registered /Profile/HFPHF is unknown. Whether ConnectProfile(111f) triggers RFCOMM is untested. The rejected D-Bus method_return has not been correlated to a specific method call.

#### Superseded classifications (do not cite)

- ~~`PIPEWIRE_HFP_BACKEND_NOT_CONFIGURED`~~ — withdrawn. Default is already `native`.
- ~~`BLUEZ_PROFILE_MATCHING_FAILURE`~~ — withdrawn. The NewConnection callback IS delivered at the correct path.
- ~~`PIPEWIRE_HFP_BACKEND_NOT_CONFIGURED`~~ (duplicate) — see above.
- ~~"WirePlumber registers 0 Profile1 interfaces"~~ — withdrawn. ObjectManager on `org.bluez` does not list client-owned Profile1 objects.
- ~~"Stale UUID registration persists"~~ — withdrawn. One-shot busctl registration is not persistent.
- ~~"D-Bus rejects WirePlumber method_returns"~~ — withdrawn. "0 matched rules" log does not by itself prove the actual method reply was rejected.
- ~~"BlueZ never opens RFCOMM channel 8"~~ — withdrawn. Phase 7d btmon proves RFCOMM SABM sent by local Pi, UA received, full AT negotiation completed.
- ~~`HFP_CONTROL_PREVIOUSLY_VERIFIED_CURRENT_STATE_REQUIRES_RETEST`~~ — superseded by `HFP_RFCOMM_WORKS_ENUMPROFILE_MISSING_AFTER_SLC`.
- ~~`HFP_RFCOMM_WORKS_ENUMPROFILE_MISSING_AFTER_SLC`~~ — superseded. Phase 4 proved headset-head-unit DOES appear and SCO IS attempted, but SCO fails.
- ~~`BLOCKED`~~ — withdrawn. The classification `BLOCKED` was based on unsupported claims about D-Bus method_return rejection preventing Profile1 registration and ConnectProfile role mismatch.

#### Current classification

`HFP_CURRENT_REGISTRATION_AND_CONNECT_STATE_UNRESOLVED` — Whether the current WirePlumber process registered /Profile/HFPHF is unknown. Whether ConnectProfile(111f) triggers RFCOMM is untested. The rejected D-Bus method_return has not been correlated to a specific method call.

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
5. **SCO connection reset by iPhone** — headset-head-unit appears, SCO transport created, but SCO link fails (-104)
6. **`ServicesResolved` delayed 30+ seconds** — only resolves after manual disconnect/reconnect
7. **A2DP SET_CONFIGURATION rejected** — iPhone rejected initial A2DP codec negotiation
8. **HFP RFCOMM absent in current session** — Whether Profile1 is registered, ConnectProfile triggers RFCOMM, or BlueZ state is the cause is unresolved

## Key Findings

1. MAP works via imsg without needing bluez-obexd
2. PBAP works via imsg without needing bluez-obexd
3. iOS requires manual permission grant (Show Notifications, Share Contacts) before first connection
4. iOS permission prompt appears after first connection attempt
5. imsg connects via its own OBEX implementation (bluer crate)
6. MAP channel: RFCOMM 2, PBAP channel: RFCOMM 13
7. iPhone advertises HFP AG (`111f`) — correct for Pi acting as hands-free
8. PipeWire EnumProfile does not expose `headset-head-unit` — only `audio-gateway` (Pi-as-AG)
9. Explicit `ConnectProfile(111f)` returns success — HFP RFCOMM connection established (earlier tests)
10. HFP AT negotiation completes successfully — all commands return OK
11. NewConnection IS delivered at `/Profile/HFPHF` — WirePlumber receives RFCOMM fd (earlier instance)
12. `bluez5.roles=[hfp_hf]` absence before HFP connection is expected behavior, not proof of failure
13. Profile1 objects are client-owned — not visible in BlueZ ObjectManager
14. One-shot busctl RegisterProfile does not persist — registration removed when caller exits
15. Previous `PIPEWIRE_HFP_BACKEND_NOT_CONFIGURED` diagnosis withdrawn — default is already `native`
16. `HFP_SLC_WORKS_SCO_RESET_ENUMPROFILE_MISSING` — RFCOMM+SLC work, SCO transport created, but SCO link reset by iPhone
17. Native backend and `hfp_hf` role are WirePlumber 0.5 defaults — no explicit config needed
18. Phase 7d btmon proves local Pi initiates RFCOMM (SABM TX), not iPhone — earlier "BlueZ never opens RFCOMM" claim withdrawn
19. D-Bus `method_return` rejection observed — may indicate security policy issue preventing Profile1 handshake completion
20. Phase 4 restart: headset-head-unit DID appear (evidence: `spa.bluez5.sink.sco` error), but SCO link was reset by iPhone (-104)
21. Phase 4 restart: `ServicesResolved` false for 30+ seconds — SDP delayed after WirePlumber restart
22. Phase 4 restart: A2DP `SET_CONFIGURATION rejected: Configuration not supported (41)` — iPhone rejected codec
23. `ConnectProfile(111f)` uses the correct remote UUID (HFP Audio Gateway), not a role mismatch
24. D-Bus `method_return` rejection from WirePlumber to bluetoothd observed but not proven to prevent Profile1 registration
