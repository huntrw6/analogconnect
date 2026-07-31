# AnalogConnect — Raspberry Pi Environment Audit

Date: 2026-07-31

## System

- `VERIFIED_AUTOMATED`: Model: Raspberry Pi 5 Model B Rev 1.1
- `VERIFIED_AUTOMATED`: OS: Debian GNU/Linux 13 (trixie)
- `VERIFIED_AUTOMATED`: Architecture: aarch64 (64-bit ARM)
- `VERIFIED_AUTOMATED`: Kernel: 6.18.34+rpt-rpi-2712 (SMP PREEMPT, Debian patch)
- `VERIFIED_AUTOMATED`: RAM: 15Gi total, 13Gi available
- `VERIFIED_AUTOMATED`: Storage: 58Gi total, 47Gi available (/dev/mmcblk0p2)

## USB Devices

- `VERIFIED_AUTOMATED`: Logitech Unifying Receiver (046d:c52b)
- `VERIFIED_AUTOMATED`: Keychron V1 keyboard (3434:0311)
- No external Bluetooth adapters detected (using onboard controller)

## Bluetooth

- `VERIFIED_AUTOMATED`: Controller: hci0 — <REDACTED_BLUETOOTH_ADDRESS> (public)
- `VERIFIED_AUTOMATED`: Manufacturer: 0x0131 (305) — likely Cypress/Infineon
- `VERIFIED_AUTOMATED`: BT Version: 0x09 (Bluetooth 5.x)
- `VERIFIED_AUTOMATED`: RF-kill: Neither soft nor hard blocked
- `VERIFIED_AUTOMATED`: Bluetooth service: active (running), enabled at boot
- `VERIFIED_AUTOMATED`: BlueZ version: 5.82 (bluetoothctl and bluetoothd)
- `VERIFIED_AUTOMATED`: Controller powered: yes, pairable: yes
- `VERIFIED_AUTOMATED`: Roles: central, peripheral

### Advertised Bluetooth Profiles (on controller)

- `VERIFIED_AUTOMATED`: Handsfree (HFP HF) — UUID 0000111e
- `VERIFIED_AUTOMATED`: Handsfree Audio Gateway (HFP AG) — UUID 0000111f
- `VERIFIED_AUTOMATED`: Audio Source (A2DP) — UUID 0000110a
- `VERIFIED_AUTOMATED`: Audio Sink (A2DP) — UUID 0000110b
- `VERIFIED_AUTOMATED`: A/V Remote Control — UUID 0000110e
- `VERIFIED_AUTOMATED`: A/V Remote Control Target — UUID 0000110c
- `VERIFIED_AUTOMATED`: SIM Access — UUID 0000112d
- `VERIFIED_AUTOMATED`: PnP Information — UUID 00001200
- `VERIFIED_AUTOMATED`: Generic Access Profile — UUID 00001800
- `VERIFIED_AUTOMATED`: Generic Attribute Profile — UUID 00001801
- `VERIFIED_AUTOMATED`: Device Information — UUID 0000180a
- `VERIFIED_AUTOMATED`: Vendor specific — UUID 03b80e5a

### Missing UUIDs of interest

- `UNKNOWN`: MAP (Message Access Profile) — UUID 00001132 — NOT in controller list
- `UNKNOWN`: PBAP (Phonebook Access Profile) — UUID 0000112f — NOT in controller list
- `UNKNOWN`: MAP MAS (Message Access Server) — UUID 00001134 — NOT in controller list

Note: MAP and PBAP client roles may only appear after BlueZ profile plugins are loaded and configured, or may not be supported by this controller's default configuration. Further investigation needed.

### D-Bus

- `VERIFIED_AUTOMATED`: BlueZ D-Bus tree available: `/org/bluez`, `/org/bluez/hci0`, `/org/bluez/test`

## Audio

- `VERIFIED_AUTOMATED`: PipeWire version: 1.4.2
- `VERIFIED_AUTOMATED`: WirePlumber version: 0.5.8
- `VERIFIED_AUTOMATED`: PipeWire daemon running with active clients
- `VERIFIED_AUTOMATED`: Sink: Built-in Audio Digital Stereo (HDMI) at 40% volume
- `VERIFIED_AUTOMATED`: Audio devices: Built-in Audio (ALSA) x2
- `INFERRED`: No Bluetooth audio sink/source currently connected
- `BLOCKED`: `pactl` not installed — cannot query PulseAudio compat layer details
- `UNKNOWN`: SCO audio routing through PipeWire not yet tested

## Development Tools

- `VERIFIED_AUTOMATED`: Python 3.13.5
- `VERIFIED_AUTOMATED`: Git 2.47.3
- `VERIFIED_AUTOMATED`: GCC 14.2.0 (Debian)
- `VERIFIED_AUTOMATED`: Meson 1.7.0
- `VERIFIED_AUTOMATED`: Ninja 1.12.1
- `VERIFIED_AUTOMATED`: pkg-config 1.8.1
- `UNKNOWN`: Rust/Cargo — NOT INSTALLED
- `UNKNOWN`: CMake — NOT INSTALLED
- `UNKNOWN`: ShellCheck — NOT INSTALLED

## User Groups

- `VERIFIED_AUTOMATED`: Groups: operat adm dialout cdrom sudo audio video plugdev games users netdev lpadmin gpio i2c spi render input
- `UNKNOWN`: No `bluetooth` group — may need to be added for non-root Bluetooth access

## Readiness Summary

### Ready

- Raspberry Pi 5 with sufficient RAM and storage
- Debian 13 aarch64 — fully supported platform
- Bluetooth controller powered and unblocked
- BlueZ 5.82 with D-Bus interfaces
- PipeWire 1.4.2 with WirePlumber 0.5.8
- GCC, Meson, Ninja, pkg-config for building C projects
- Python 3 for scripting

### Missing / Needs Installation

- ShellCheck (for script linting — needed for Phase 0C)
- Rust/Cargo (may be needed for some build dependencies)
- CMake (may be needed for some build dependencies)
- `pactl` (for PulseAudio compatibility queries)
- Possibly `bluetooth` group membership for non-root operations

### Needs Investigation

- MAP and PBAP profile availability on this BlueZ configuration
- SCO audio routing through PipeWire
- Whether BlueZ OBEX MAP and PBAP clients are installed and functional
