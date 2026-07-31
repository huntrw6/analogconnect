#!/usr/bin/env bash
set -euo pipefail

# AnalogConnect — System Diagnostic Doctor
# Read-only system health check for Milestone 0
# Never changes system state. Safe to run without root.

VERSION="0.1.0"

JSON=false
NO_REDACT=false
EXIT_CODE=0

usage() {
    cat <<EOF
Usage: $(basename "$0") [OPTIONS]

AnalogConnect system diagnostic tool.

Options:
  --json        Output results as JSON
  --no-redact   Do not redact Bluetooth addresses (use with caution)
  --version     Show version
  --help        Show this help

Exit codes:
  0  All core diagnostics pass
  1  A required capability fails
  2  Checks are blocked or incomplete
  64 Invalid command-line use
EOF
}

redact_addr() {
    if [[ "$NO_REDACT" == true ]]; then
        cat
    else
        sed -E 's/([0-9A-Fa-f]{2}:){2}[0-9A-Fa-f]{2}(:[0-9A-Fa-f]{2}){3}/XX:XX:XX:XX:XX:XX/g'
    fi
}

check_pass() {
    local name="$1" detail="$2"
    if [[ "$JSON" == true ]]; then
        printf '  {"check":%s,"status":"pass","detail":%s}' "$(json_quote "$name")" "$(json_quote "$detail")"
    else
        printf '  PASS  %-30s %s\n' "$name" "$detail"
    fi
}

check_fail() {
    local name="$1" detail="$2"
    EXIT_CODE=1
    if [[ "$JSON" == true ]]; then
        printf '  {"check":%s,"status":"fail","detail":%s}' "$(json_quote "$name")" "$(json_quote "$detail")"
    else
        printf '  FAIL  %-30s %s\n' "$name" "$detail"
    fi
}

check_warn() {
    local name="$1" detail="$2"
    if [[ "$EXIT_CODE" -eq 0 ]]; then
        EXIT_CODE=2
    fi
    if [[ "$JSON" == true ]]; then
        printf '  {"check":%s,"status":"warn","detail":%s}' "$(json_quote "$name")" "$(json_quote "$detail")"
    else
        printf '  WARN  %-30s %s\n' "$name" "$detail"
    fi
}

check_blocked() {
    local name="$1" detail="$2"
    if [[ "$EXIT_CODE" -eq 0 ]]; then
        EXIT_CODE=2
    fi
    if [[ "$JSON" == true ]]; then
        printf '  {"check":%s,"status":"blocked","detail":%s}' "$(json_quote "$name")" "$(json_quote "$detail")"
    else
        printf '  BLOCK %-30s %s\n' "$name" "$detail"
    fi
}

cmd_exists() {
    command -v "$1" >/dev/null 2>&1
}

json_quote() {
    printf '"%s"' "$(printf '%s' "$1" | sed 's/\\/\\\\/g; s/"/\\"/g; s/	/\\t/g')"
}

# --- Parse arguments ---
while [[ $# -gt 0 ]]; do
    case "$1" in
        --json) JSON=true; shift ;;
        --no-redact) NO_REDACT=true; shift ;;
        --version) echo "doctor $VERSION"; exit 0 ;;
        --help) usage; exit 0 ;;
        *) echo "Unknown option: $1" >&2; usage >&2; exit 64 ;;
    esac
done

# --- Begin checks ---
if [[ "$JSON" == true ]]; then
    echo '{'
    echo '  "version": "'"$VERSION"'",'
    echo '  "checks": ['
    FIRST=true
fi

section() {
    local name="$1"
    if [[ "$JSON" == true ]]; then
        if [[ "$FIRST" == true ]]; then
            FIRST=false
        else
            echo '  ]},'
        fi
        echo '    {"section": '"$(json_quote "$name")"', "items": ['
        SECTION_FIRST=true
    else
        echo ""
        echo "=== $name ==="
    fi
}

section_item() {
    if [[ "$JSON" == true ]]; then
        if [[ "$SECTION_FIRST" == true ]]; then
            SECTION_FIRST=false
        else
            echo ','
        fi
        printf '      '
    fi
}

# --- System ---
section "System"

if cmd_exists uname; then
    KERNEL=$(uname -r 2>/dev/null || echo "unknown")
    ARCH=$(uname -m 2>/dev/null || echo "unknown")
    section_item
    check_pass "kernel" "$KERNEL ($ARCH)"
else
    section_item
    check_fail "kernel" "uname not available"
fi

if [[ -f /etc/os-release ]]; then
    OS_NAME=$(grep '^PRETTY_NAME=' /etc/os-release | cut -d= -f2 | tr -d '"' || echo "unknown")
    section_item
    check_pass "os" "$OS_NAME"
else
    section_item
    check_warn "os" "/etc/os-release not found"
fi

if [[ -f /proc/device-tree/model ]]; then
    MODEL=$(tr -d '\0' < /proc/device-tree/model 2>/dev/null || echo "unknown")
    section_item
    check_pass "model" "$MODEL"
else
    section_item
    check_warn "model" "Not a Raspberry Pi or /proc/device-tree/model missing"
fi

RAM_TOTAL=$(free -m 2>/dev/null | awk '/^Mem:/{print $2}' || echo "0")
if [[ "$RAM_TOTAL" -gt 0 ]]; then
    section_item
    check_pass "ram" "${RAM_TOTAL}MB total"
else
    section_item
    check_warn "ram" "Could not determine RAM"
fi

# --- Bluetooth ---
section "Bluetooth"

if cmd_exists bluetoothctl; then
    BT_VER=$(bluetoothctl --version 2>/dev/null | awk '{print $2}' || echo "unknown")
    section_item
    check_pass "bluetoothctl" "v$BT_VER"
else
    section_item
    check_fail "bluetoothctl" "not found"
fi

if cmd_exists bluetoothd || [[ -x /usr/libexec/bluetooth/bluetoothd ]]; then
    BTdaemon_VER=$(bluetoothd --version 2>/dev/null | awk '{print $NF}' || echo "unknown")
    section_item
    check_pass "bluetoothd" "v$BTdaemon_VER"
else
    section_item
    check_fail "bluetoothd" "not found"
fi

if cmd_exists rfkill; then
    RFKILL_SOFT=$(rfkill list bluetooth 2>/dev/null | grep -i "soft blocked" | awk '{print $NF}' || echo "unknown")
    RFKILL_HARD=$(rfkill list bluetooth 2>/dev/null | grep -i "hard blocked" | awk '{print $NF}' || echo "unknown")
    if [[ "$RFKILL_SOFT" == "no" && "$RFKILL_HARD" == "no" ]]; then
        section_item
        check_pass "rfkill" "not blocked"
    elif [[ "$RFKILL_SOFT" == "yes" ]]; then
        section_item
        check_fail "rfkill" "soft blocked"
    elif [[ "$RFKILL_HARD" == "yes" ]]; then
        section_item
        check_fail "rfkill" "hard blocked"
    else
        section_item
        check_warn "rfkill" "soft=$RFKILL_SOFT hard=$RFKILL_HARD"
    fi
else
    section_item
    check_warn "rfkill" "rfkill command not found"
fi

if cmd_exists systemctl; then
    BT_SERVICE=$(systemctl is-active bluetooth 2>/dev/null) || true
    if [[ "$BT_SERVICE" == "active" ]]; then
        section_item
        check_pass "bluetooth-service" "active"
    else
        section_item
        check_fail "bluetooth-service" "$BT_SERVICE"
    fi
else
    section_item
    check_warn "bluetooth-service" "systemctl not available"
fi

# Check for Bluetooth controller via bluetoothctl
if cmd_exists bluetoothctl; then
    CTL_INFO=$(bluetoothctl show 2>/dev/null | head -5 | redact_addr || echo "")
    if echo "$CTL_INFO" | grep -q "Controller"; then
        section_item
        check_pass "controller" "detected"
    else
        section_item
        check_fail "controller" "no controller found"
    fi
else
    section_item
    check_blocked "controller" "bluetoothctl not available"
fi

# Check bluetooth group membership
if id -nG 2>/dev/null | grep -qw "bluetooth"; then
    section_item
    check_pass "bluetooth-group" "user in bluetooth group"
else
    section_item
    check_warn "bluetooth-group" "user not in bluetooth group (may need for non-root access)"
fi

# --- OBEX ---
section "OBEX"

if cmd_exists obexctl; then
    section_item
    check_pass "obexctl" "available"
else
    section_item
    check_warn "obexctl" "not found (needed for MAP/PBAP)"
fi

# Check for OBEX service
if cmd_exists systemctl; then
    OBEX_SERVICE=$(systemctl is-active obex 2>/dev/null) || true
    if [[ "$OBEX_SERVICE" == "active" ]]; then
        section_item
        check_pass "obex-service" "active"
    elif [[ -z "$OBEX_SERVICE" ]]; then
        section_item
        check_warn "obex-service" "service not found"
    else
        section_item
        check_warn "obex-service" "$OBEX_SERVICE"
    fi
else
    section_item
    check_blocked "obex-service" "systemctl not available"
fi

# --- Audio ---
section "Audio"

if cmd_exists pipewire; then
    PW_VER=$(pipewire --version 2>/dev/null | head -2 | tail -1 | awk '{print $NF}' || echo "unknown")
    section_item
    check_pass "pipewire" "v$PW_VER"
else
    section_item
    check_fail "pipewire" "not found"
fi

if cmd_exists wireplumber; then
    WP_VER=$(wireplumber --version 2>/dev/null | head -2 | tail -1 | awk '{print $NF}' || echo "unknown")
    section_item
    check_pass "wireplumber" "v$WP_VER"
else
    section_item
    check_warn "wireplumber" "not found"
fi

if cmd_exists wpctl; then
    PW_STATUS=$(wpctl status 2>/dev/null | head -1 || echo "unknown")
    section_item
    check_pass "wpctl" "available"
else
    section_item
    check_warn "wpctl" "not found"
fi

if cmd_exists pactl; then
    section_item
    check_pass "pactl" "available"
else
    section_item
    check_warn "pactl" "not found"
fi

# --- D-Bus ---
section "D-Bus"

if cmd_exists busctl; then
    BLUEZ_TREE=$(busctl tree org.bluez 2>&1 | redact_addr || echo "")
    if echo "$BLUEZ_TREE" | grep -q "/org/bluez"; then
        section_item
        check_pass "bluez-dbus" "registered"
    else
        section_item
        check_warn "bluez-dbus" "org.bluez not on bus"
    fi
else
    section_item
    check_warn "busctl" "not available"
fi

# --- Development Tools ---
section "Development Tools"

for tool in python3 git gcc meson ninja pkg-config; do
    if cmd_exists "$tool"; then
        VER=$($tool --version 2>/dev/null | head -1 || echo "unknown")
        section_item
        check_pass "$tool" "$VER"
    else
        section_item
        check_warn "$tool" "not found"
    fi
done

for tool in shellcheck rustc cargo cmake; do
    if cmd_exists "$tool"; then
        VER=$($tool --version 2>/dev/null | head -1 || echo "unknown")
        section_item
        check_pass "$tool" "$VER"
    else
        section_item
        check_warn "$tool" "not found"
    fi
done

# --- Close JSON ---
if [[ "$JSON" == true ]]; then
    echo '  ]}'
    echo '  ]'
    echo '}'
fi

exit "$EXIT_CODE"
