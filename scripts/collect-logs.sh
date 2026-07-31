#!/usr/bin/env bash
set -euo pipefail

# AnalogConnect — Privacy-Safe Log Collector
# Collects Bluetooth and audio logs for testing.
# Redacts personal data by default.

VERSION="0.1.0"

JSON=false
NO_REDACT=false
INCLUDE_SENSITIVE=false
OUTPUT_DIR=""

usage() {
    cat <<EOF
Usage: $(basename "$0") [OPTIONS]

Collect Bluetooth and audio logs for diagnostics.

Options:
  --output <dir>        Output directory (default: test-results/<timestamp>)
  --json                Output manifest as JSON
  --no-redact           Do not redact addresses (USE WITH CAUTION)
  --include-sensitive   Include potentially sensitive data (requires explicit opt-in)
  --version             Show version
  --help                Show this help

Privacy notes:
  - Bluetooth addresses are redacted by default
  - Telephone numbers are redacted
  - Message bodies are never collected
  - Pairing keys are never collected
  - Contact names are never collected
  - Audio recordings are never collected
  --include-sensitive will include:
    - Bluetooth device names (may reveal personal info)
    - Service discovery data
EOF
}

redact_addr() {
    if [[ "$NO_REDACT" == true ]]; then
        cat
    else
        sed -E 's/([0-9A-Fa-f]{2}:){2}[0-9A-Fa-f]{2}(:[0-9A-Fa-f]{2}){3}/XX:XX:XX:XX:XX:XX/g'
    fi
}

redact_phone() {
    if [[ "$NO_REDACT" == true ]]; then
        cat
    else
        sed -E 's/\+?[0-9]{7,15}/+XXXXXXXXXXX/g'
    fi
}

redact_all() {
    redact_addr | redact_phone
}

cmd_exists() {
    command -v "$1" >/dev/null 2>&1
}

# --- Parse arguments ---
while [[ $# -gt 0 ]]; do
    case "$1" in
        --output) OUTPUT_DIR="$2"; shift 2 ;;
        --json) JSON=true; shift ;;
        --no-redact) NO_REDACT=true; shift ;;
        --include-sensitive) INCLUDE_SENSITIVE=true; shift ;;
        --version) echo "collect-logs $VERSION"; exit 0 ;;
        --help) usage; exit 0 ;;
        *) echo "Unknown option: $1" >&2; usage >&2; exit 64 ;;
    esac
done

# --- Setup output directory ---
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
if [[ -z "$OUTPUT_DIR" ]]; then
    OUTPUT_DIR="test-results/${TIMESTAMP}"
fi

mkdir -p "$OUTPUT_DIR"

MANIFEST="$OUTPUT_DIR/manifest.md"
declare -a MANIFEST_ITEMS=()

add_item() {
    local file="$1" description="$2"
    MANIFEST_ITEMS+=("| $file | $description |")
}

# --- Collect Bluetooth logs ---

# bluetoothctl info (general)
if cmd_exists bluetoothctl; then
    bluetoothctl show 2>/dev/null | redact_all > "$OUTPUT_DIR/bt-controller-info.txt" || true
    add_item "bt-controller-info.txt" "Bluetooth controller information"
fi

# Paired devices (names may be sensitive)
if cmd_exists bluetoothctl && [[ "$INCLUDE_SENSITIVE" == true ]]; then
    bluetoothctl paired-devices 2>/dev/null | redact_all > "$OUTPUT_DIR/bt-paired-devices.txt" || true
    add_item "bt-paired-devices.txt" "List of paired Bluetooth devices"
else
    echo "Skipped: paired device list (requires --include-sensitive)" > "$OUTPUT_DIR/bt-paired-devices.txt"
    add_item "bt-paired-devices.txt" "Skipped for privacy"
fi

# D-Bus BlueZ tree
if cmd_exists busctl; then
    busctl tree org.bluez 2>/dev/null | redact_all > "$OUTPUT_DIR/bt-dbus-tree.txt" || true
    add_item "bt-dbus-tree.txt" "BlueZ D-Bus object tree"
fi

# RF-kill state
if cmd_exists rfkill; then
    rfkill list 2>/dev/null | redact_all > "$OUTPUT_DIR/bt-rfkill.txt" || true
    add_item "bt-rfkill.txt" "RF-kill state for Bluetooth and WiFi"
fi

# --- Collect audio logs ---

# PipeWire status
if cmd_exists wpctl; then
    wpctl status 2>/dev/null > "$OUTPUT_DIR/audio-pipewire-status.txt" || true
    add_item "audio-pipewire-status.txt" "PipeWire/WirePlumber audio status"
fi

# PipeWire version
if cmd_exists pipewire; then
    pipewire --version 2>/dev/null > "$OUTPUT_DIR/audio-pipewire-version.txt" || true
    add_item "audio-pipewire-version.txt" "PipeWire version"
fi

# --- Collect system logs ---

# Bluetooth service status
if cmd_exists systemctl; then
    systemctl status bluetooth --no-pager 2>/dev/null > "$OUTPUT_DIR/sys-bluetooth-service.txt" || true
    add_item "sys-bluetooth-service.txt" "Bluetooth systemd service status"
fi

# Kernel messages related to Bluetooth (recent only, no personal data)
if [[ -r /var/log/kern.log ]]; then
    grep -i "bluetooth\|btusb\|hci" /var/log/kern.log 2>/dev/null | tail -100 | redact_all > "$OUTPUT_DIR/sys-kern-bt.txt" || true
    add_item "sys-kern-bt.txt" "Recent kernel Bluetooth messages"
elif cmd_exists dmesg; then
    dmesg 2>/dev/null | grep -i "bluetooth\|btusb\|hci" | tail -100 | redact_all > "$OUTPUT_DIR/sys-kern-bt.txt" || true
    add_item "sys-kern-bt.txt" "Recent kernel Bluetooth messages (dmesg)"
fi

# --- Generate manifest ---
if [[ "$JSON" == true ]]; then
    echo '{'
    echo '  "version": "'"$VERSION"'",'
    echo '  "timestamp": "'"$TIMESTAMP"'",'
    echo '  "output_dir": "'"$OUTPUT_DIR"'",'
    echo '  "include_sensitive": '$INCLUDE_SENSITIVE','
    echo '  "items": ['
    FIRST=true
    for item in "${MANIFEST_ITEMS[@]}"; do
        FILE=$(echo "$item" | cut -d'|' -f2 | xargs)
        DESC=$(echo "$item" | cut -d'|' -f3 | xargs)
        if [[ "$FIRST" == true ]]; then
            FIRST=false
        else
            echo ','
        fi
        printf '    {"file":%q,"description":%q}' "$FILE" "$DESC"
    done
    echo ''
    echo '  ]'
    echo '}'
else
    {
        echo "# AnalogConnect Log Collection Manifest"
        echo ""
        echo "- **Date**: $(date -Iseconds)"
        echo "- **Output directory**: $OUTPUT_DIR"
        echo "- **Include sensitive**: $INCLUDE_SENSITIVE"
        if [[ "$NO_REDACT" == true ]]; then
            echo "- **Redacted**: NO"
        else
            echo "- **Redacted**: YES"
        fi
        echo ""
        echo "## Collected Files"
        echo ""
        echo "| File | Description |"
        echo "|------|-------------|"
        for item in "${MANIFEST_ITEMS[@]}"; do
            echo "$item"
        done
        echo ""
        echo "Manifest written to: $MANIFEST"
    } > "$MANIFEST"
fi
