# AnalogConnect — Upstream Research Sources

Date: 2026-07-31

## 1. BlueZ

| Field | Value | Evidence |
|---|---|---|
| Canonical repo | https://git.kernel.org/pub/scm/bluetooth/bluez.git | `DOCUMENTED` |
| GitHub mirror | https://github.com/bluez/bluez | `DOCUMENTED` |
| License | GPL-2.0 (daemon), LGPL-2.1 (libraries) | `DOCUMENTED` |
| Latest stable | 5.87 (July 3, 2026) | `DOCUMENTED` |
| ARM64 support | Yes — packaged for aarch64, ships with Pi OS | `DOCUMENTED` |

### Profile support

| Profile | Support | Evidence |
|---|---|---|
| MAP client (MAS) | Yes — `obexd/client/map.c`, D-Bus API `org.bluez.obex.MessageAccess1` | `DOCUMENTED` |
| PBAP client (PCE) | Yes — `obexd/client/pbap.c`, D-Bus API `org.bluez.obex.PhonebookAccess1` | `DOCUMENTED` |
| HFP HF (hands-free) | Yes — via oFono integration or `bluez-alsa` | `DOCUMENTED` |
| SCO audio routing | Configurable via `SCORouting=HCI` in `main.conf` | `DOCUMENTED` |

### Permissions

- `bluetoothd` requires root/specific capabilities
- `obexd` runs in user session (no root needed)
- `bluetooth` group grants D-Bus access by default

### Recommendation

Direct use for MAP/PBAP. HFP requires oFono or similar for AT command handling.

---

## 2. PipeWire

| Field | Value | Evidence |
|---|---|---|
| Canonical repo | https://gitlab.freedesktop.org/pipewire/pipewire | `DOCUMENTED` |
| GitHub mirror | https://github.com/PipeWire/pipewire | `DOCUMENTED` |
| License | MIT | `DOCUMENTED` |
| Installed version | 1.4.2 | `VERIFIED_AUTOMATED` |

### Bluetooth support

| Feature | Status | Since |
|---|---|---|
| HFP HF (hands-free) | Supported | 0.3.24 |
| HFP AG (audio gateway) | Supported | 0.3.26 |
| Bidirectional SCO audio | Supported (mSBC + CVSD) | 0.3.24 |
| Telephony D-Bus API | Supported (oFono-compatible) | 1.4.0 |
| A2DP | Supported (SBC, AAC, AptX, LDAC, Opus) | 0.2.x |

### iPhone compatibility

- Pi must act as HFP HF (not AG) — iPhone defaults to AG role
- mSBC (16 kHz wideband) requires both sides to support it
- eSCO may downgrade to CVSD (8 kHz) if negotiation fails

### Configuration

- `bluez5.roles = [ hfp_hf ]` in `monitor.bluez.properties`
- `bluez5.hfphsp-backend = "native"` (default)

### Recommendation

Direct use. PipeWire 1.4+ is well-suited for Bluetooth SCO audio routing.

---

## 3. WirePlumber

| Field | Value | Evidence |
|---|---|---|
| Canonical repo | https://gitlab.com/pipewire/wireplumber | `DOCUMENTED` |
| License | MIT | `DOCUMENTED` |
| Installed version | 0.5.8 | `VERIFIED_AUTOMATED` |

### Recommendation

Direct use as PipeWire session manager.

---

## 4. gnufood/imsg

| Field | Value | Evidence |
|---|---|---|
| Repo | https://github.com/gnufood/imsg | `VERIFIED_AUTOMATED` |
| License | MIT | `VERIFIED_AUTOMATED` |
| Latest version | v0.3.1 (July 8, 2026) | `VERIFIED_AUTOMATED` |
| Language | Rust | `VERIFIED_AUTOMATED` |
| Age | 5 weeks old, 7 releases | `INFERRED` |

### Profile support

| Profile | Status | Evidence |
|---|---|---|
| MAP client | Full support — list, get, send, delete, MNS notifications | `VERIFIED_AUTOMATED` |
| PBAP client | Full support — contacts, lookup, pagination | `VERIFIED_AUTOMATED` |
| HFP | NOT supported (listed in ROADMAP as "Research") | `VERIFIED_AUTOMATED` |

### Key features

- Encrypted local SQLite database (SQLCipher)
- Keyring-backed encryption key
- MNS notification server for live MAP events
- Local Bluetooth RFCOMM only — no cloud relay for primary path
- Hub/spoke via QUIC (iroh) — optional, not required

### Build requirements

- Rust 1.89.0+ (pinned via `rust-toolchain.toml`)
- `just` task runner
- `libsqlite3-dev` or `sqlcipher`
- BlueZ (`bluetoothd`)

### Maturity assessment

- Very young but actively developed with CI-enforced quality
- No automated Bluetooth integration tests possible in CI
- `unsafe` forbidden workspace-wide
- 100% doc coverage for store crate

### Recommendation

Use for MAP and PBAP client functionality. Not suitable for HFP — needs separate solution.

---

## 5. oFono (Fallback)

| Field | Value | Evidence |
|---|---|---|
| Canonical repo | git://git.kernel.org/pub/scm/network/ofono/ofono.git | `DOCUMENTED` |
| License | GPL-2.0 | `DOCUMENTED` |
| Latest release | 2.19 (May 12, 2026) | `DOCUMENTED` |

### Profile support

| Profile | Status | Evidence |
|---|---|---|
| HFP AG | Yes — `plugins/hfp_ag_bluez5.c` | `DOCUMENTED` |
| HFP HF | Yes — but only BlueZ4 plugin, not ported to BlueZ5 | `DOCUMENTED` |
| MAP/PBAP | Not implemented — handled by BlueZ obexd | `DOCUMENTED` |

### Critical limitation

**Requires at least one enabled modem** before exposing HFP interfaces. Workaround: `ofono-phonesim` (dummy modem simulator). Adds significant complexity.

### iPhone compatibility

- Works with iPhone via HFP
- Audio routing issues reported (silent calls, transport errors)
- Requires "Show Notifications" enabled on iPhone for MAP

### Recommendation

**Fallback only.** oFono introduces unnecessary complexity for Milestone 0. Use only if BlueZ native HFP proves insufficient for call control. PipeWire 1.4+ has its own telephony API that may provide a simpler path.

---

## Architecture Decision

Based on research:

| Function | Recommended Component | Confidence |
|---|---|---|
| MAP messaging | gnufood/imsg (Rust) | HIGH |
| PBAP contacts | gnufood/imsg (Rust) | HIGH |
| HFP call control | PipeWire telephony API + BlueZ | MEDIUM |
| SCO audio routing | PipeWire + WirePlumber | HIGH |
| Bluetooth transport | BlueZ 5.82+ | HIGH |

### Missing capability

- No single component provides MAP + PBAP + HFP + SCO in one package
- imsg handles MAP + PBAP but not HFP
- PipeWire handles HFP + SCO but not MAP/PBAP
- A thin integration layer may be needed to coordinate these
