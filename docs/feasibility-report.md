# AnalogConnect — Feasibility Report

Date: 2026-07-31
Milestone: 0 — iPhone Bluetooth Feasibility

## Capability Matrix

| Capability | Status | Evidence | Test ID | Notes |
|---|---|---|---|---|
| MAP notifications | BLOCKED | Not tested — iPhone not trusted | — | Need trust first |
| MAP retrieval | BLOCKED | Not tested — iPhone not trusted | — | Need trust first |
| MAP send | BLOCKED | Not tested — iPhone not trusted | — | Need trust first |
| PBAP listing | BLOCKED | Not tested — iPhone not trusted | — | Need trust first |
| PBAP test contact | BLOCKED | Not tested — iPhone not trusted | — | Need trust first |
| HFP call event | UNKNOWN | UUID advertised but not tested | — | Need HFP connection test |
| HFP answer | UNKNOWN | Not tested | — | |
| HFP hangup | UNKNOWN | Not tested | — | |
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
| Trusted | **no** |
| Connected | yes |
| MAP UUID | advertised |
| PBAP UUID | advertised |
| HFP UUID | advertised |

## Blockers

1. **iPhone not trusted** — must run `bluetoothctl trust <address>` before MAP/PBAP access
2. **obexd not installed** — imsg uses own OBEX implementation (may work, needs testing)
3. **Bluetooth group** — not active in current session (requires re-login)
