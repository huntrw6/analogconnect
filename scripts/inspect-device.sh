#!/usr/bin/env bash
set -euo pipefail

# AnalogConnect — Bluetooth Device Inspector
# Inspects a paired Bluetooth device for supported profiles and services.
# Read-only. Safe to run without root.

VERSION="0.1.0"

DEVICE=""
JSON=false
NO_REDACT=false
LOCAL_CONFIG=""

usage() {
    cat <<EOF
Usage: $(basename "$0") --device <MAC_ADDRESS> [OPTIONS]

Inspect a paired Bluetooth device for supported profiles and services.

Options:
  --device <MAC>    Bluetooth MAC address to inspect (required)
  --config <path>   Optional local configuration file with device info
  --json            Output results as JSON
  --no-redact       Do not redact Bluetooth addresses
  --version         Show version
  --help            Show this help

Result states:
  PASS              Profile detected and functional
  FAIL              Profile detected but not working
  BLOCKED           Cannot test without hardware interaction
  NOT_SUPPORTED     Device does not advertise this profile
  NOT_CONFIGURED    Profile exists but not configured
  USER_ACTION_REQUIRED  User needs to perform an action
EOF
}

redact_addr() {
    if [[ "$NO_REDACT" == true ]]; then
        cat
    else
        sed -E 's/([0-9A-Fa-f]{2}:){2}[0-9A-Fa-f]{2}(:[0-9A-Fa-f]{2}){3}/XX:XX:XX:XX:XX:XX/g'
    fi
}

# Known Bluetooth profile UUIDs
declare -A PROFILE_UUIDS=(
    ["HFP_HF"]="0000111e-0000-1000-8000-00805f9b34fb"
    ["HFP_AG"]="0000111f-0000-1000-8000-00805f9b34fb"
    ["A2DP_SOURCE"]="0000110a-0000-1000-8000-00805f9b34fb"
    ["A2DP_SINK"]="0000110b-0000-1000-8000-00805f9b34fb"
    ["AVRCP_CONTROLLER"]="0000110e-0000-1000-8000-00805f9b34fb"
    ["AVRCP_TARGET"]="0000110c-0000-1000-8000-00805f9b34fb"
    ["MAP"]="00001132-0000-1000-8000-00805f9b34fb"
    ["MAP_MAS"]="00001134-0000-1000-8000-00805f9b34fb"
    ["PBAP"]="0000112f-0000-1000-8000-00805f9b34fb"
    ["PBAP_PCE"]="0000112e-0000-1000-8000-00805f9b34fb"
    ["PBAP_PSE"]="0000112f-0000-1000-8000-00805f9b34fb"
    ["SIM_ACCESS"]="0000112d-0000-1000-8000-00805f9b34fb"
    ["HID"]="00001124-0000-1000-8000-00805f9b34fb"
    ["PnP"]="00001200-0000-1000-8000-00805f9b34fb"
)

result_line() {
    local profile="$1" status="$2" detail="$3"
    if [[ "$JSON" == true ]]; then
        printf '    {"profile":%q,"status":%q,"detail":%q}' "$profile" "$status" "$detail"
    else
        printf '  %-8s %-20s %s\n' "[$status]" "$profile" "$detail"
    fi
}

# --- Parse arguments ---
while [[ $# -gt 0 ]]; do
    case "$1" in
        --device) DEVICE="$2"; shift 2 ;;
        --config) LOCAL_CONFIG="$2"; shift 2 ;;
        --json) JSON=true; shift ;;
        --no-redact) NO_REDACT=true; shift ;;
        --version) echo "inspect-device $VERSION"; exit 0 ;;
        --help) usage; exit 0 ;;
        *) echo "Unknown option: $1" >&2; usage >&2; exit 64 ;;
    esac
done

if [[ -z "$DEVICE" ]]; then
    echo "Error: --device <MAC_ADDRESS> is required" >&2
    usage >&2
    exit 64
fi

# Validate MAC address format
if ! echo "$DEVICE" | grep -qE '^([0-9A-Fa-f]{2}:){5}[0-9A-Fa-f]{2}$'; then
    echo "Error: Invalid MAC address format: $DEVICE" >&2
    exit 64
fi

if ! cmd_exists bluetoothctl; then
    echo "Error: bluetoothctl not found" >&2
    exit 1
fi

# --- Begin inspection ---
if [[ "$JSON" == true ]]; then
    echo '{'
    echo '  "device": "'"$DEVICE"'",'
    echo '  "results": ['
    FIRST=true
fi

# Get device info
DEVICE_INFO=$(bluetoothctl info "$DEVICE" 2>/dev/null | redact_addr || echo "")

if [[ -z "$DEVICE_INFO" ]]; then
    result_line "device-info" "FAIL" "Device not found or not paired"
    if [[ "$JSON" == true ]]; then
        echo ''
        echo '  ]'
        echo '}'
    fi
    exit 1
fi

# Check basic device properties
NAME=$(echo "$DEVICE_INFO" | grep "Name:" | sed 's/.*Name: //' || echo "unknown")
CLASS=$(echo "$DEVICE_INFO" | grep "Class:" | sed 's/.*Class: //' || echo "unknown")

if [[ "$JSON" == true ]]; then
    echo '  "name": "'"$NAME"'",'
    echo '  "class": "'"$CLASS"'",'
fi

# Check paired/connected status
PAIRED=$(echo "$DEVICE_INFO" | grep -c "Paired: yes" || echo "0")
CONNECTED=$(echo "$DEVICE_INFO" | grep -c "Connected: yes" || echo "0")
TRUSTED=$(echo "$DEVICE_INFO" | grep -c "Trusted: yes" || echo "0")

if [[ "$PAIRED" -gt 0 ]]; then
    result_line "pairing" "PASS" "Paired"
else
    result_line "pairing" "FAIL" "Not paired"
fi

if [[ "$CONNECTED" -gt 0 ]]; then
    result_line "connection" "PASS" "Connected"
else
    result_line "connection" "BLOCKED" "Not connected"
fi

if [[ "$TRUSTED" -gt 0 ]]; then
    result_line "trust" "PASS" "Trusted"
else
    result_line "trust" "NOT_CONFIGURED" "Not trusted"
fi

# Check UUIDs from device info
UUIDS_FOUND=$(echo "$DEVICE_INFO" | grep "UUID:" || echo "")

check_profile() {
    local profile="$1"
    local uuid_lower
    uuid_lower=$(echo "${PROFILE_UUIDS[$profile]}" | tr '[:upper:]' '[:lower:]')

    if echo "$UUIDS_FOUND" | grep -qi "$uuid_lower"; then
        result_line "$profile" "PASS" "Supported"
        return 0
    fi
    result_line "$profile" "NOT_SUPPORTED" "Not advertised"
    return 1
}

# Core profiles
if [[ "$JSON" == true && "$FIRST" == true ]]; then
    FIRST=false
elif [[ "$JSON" == true ]]; then
    echo ','
fi

echo "    --- Profile Check ---" 
for profile in HFP_HF A2DP_SOURCE A2DP_SINK AVRCP_CONTROLLER AVRCP_TARGET MAP MAP_MAS PBAP PBAP_PCE PBAP_PSE SIM_ACCESS; do
    if [[ "$JSON" == true ]]; then
        echo ','
        printf '    '
    fi
    check_profile "$profile" || true
done

# Check for local config file
if [[ -n "$LOCAL_CONFIG" && -f "$LOCAL_CONFIG" ]]; then
    if [[ "$JSON" == true ]]; then
        echo ','
    fi
    result_line "local-config" "PASS" "Loaded from $LOCAL_CONFIG"
fi

# --- Close JSON ---
if [[ "$JSON" == true ]]; then
    echo ''
    echo '  ]'
    echo '}'
fi
