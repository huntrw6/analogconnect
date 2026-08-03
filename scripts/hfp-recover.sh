#!/usr/bin/env bash
set -euo pipefail

readonly HFP_AG_UUID="0000111f-0000-1000-8000-00805f9b34fb"
readonly TELEPHONY_SERVICE="org.pipewire.Telephony"
readonly TELEPHONY_ROOT="/org/pipewire/Telephony"
readonly HELPER_TIMEOUT_SECONDS=5

usage() {
    cat <<'EOF'
Usage: hfp-recover.sh --confirm

Recover a stuck idle HFP/SCO transport by cycling only the already-paired
Audio Gateway profile. Refuses to run while any call exists or unless the live
gateway transport is active. Device identifiers are never printed.
EOF
}

fail() {
    printf 'HFP_RECOVERY=FAILED reason=%s\n' "$1" >&2
    exit 1
}

[[ "${1:-}" == "--confirm" && $# -eq 1 ]] || {
    usage >&2
    exit 64
}

for command in busctl bluetoothctl timeout rg sort sed; do
    command -v "$command" >/dev/null 2>&1 || fail "required_helper_unavailable"
done

managed_objects=$(timeout "$HELPER_TIMEOUT_SECONDS" busctl --json=short --user call \
    "$TELEPHONY_SERVICE" "$TELEPHONY_ROOT" \
    org.freedesktop.DBus.ObjectManager GetManagedObjects 2>/dev/null) || \
    fail "telephony_unavailable"
mapfile -t gateways < <(
    printf '%s\n' "$managed_objects" |
        rg -o '/org/pipewire/Telephony/ag[0-9]+' |
        sort -u
)
[[ ${#gateways[@]} -eq 1 ]] || fail "gateway_state_ambiguous"
gateway=${gateways[0]}

calls=$(timeout "$HELPER_TIMEOUT_SECONDS" busctl --json=short --user call \
    "$TELEPHONY_SERVICE" "$gateway" org.ofono.VoiceCallManager GetCalls \
    2>/dev/null) || fail "call_state_unavailable"
if printf '%s\n' "$calls" | rg -q '/org/pipewire/Telephony/ag[0-9]+/call[0-9]+'; then
    fail "call_in_progress"
fi

transport_state=$(timeout "$HELPER_TIMEOUT_SECONDS" busctl --user get-property \
    "$TELEPHONY_SERVICE" "$gateway" \
    org.pipewire.Telephony.AudioGatewayTransport1 State 2>/dev/null) || \
    fail "transport_state_unavailable"
[[ "$transport_state" == 's "active"' ]] || fail "transport_not_stuck"

address_reply=$(timeout "$HELPER_TIMEOUT_SECONDS" busctl --user get-property \
    "$TELEPHONY_SERVICE" "$gateway" org.pipewire.Telephony.AudioGateway1 Address \
    2>/dev/null) || fail "gateway_address_unavailable"
gateway_address=$(printf '%s\n' "$address_reply" |
    sed -n 's/^s "\([0-9A-Fa-f:]\{17\}\)"$/\1/p')
[[ "$gateway_address" =~ ^([0-9A-Fa-f]{2}:){5}[0-9A-Fa-f]{2}$ ]] || \
    fail "gateway_address_invalid"

device_info=$(timeout "$HELPER_TIMEOUT_SECONDS" bluetoothctl info "$gateway_address" \
    2>/dev/null) || fail "paired_gateway_unavailable"
printf '%s\n' "$device_info" | rg -q '^[[:space:]]*Paired: yes$' || \
    fail "gateway_not_paired"
printf '%s\n' "$device_info" | rg -qi "$HFP_AG_UUID" || \
    fail "gateway_profile_missing"

disconnected=false
reconnect_if_needed() {
    if [[ "$disconnected" == true ]]; then
        timeout "$HELPER_TIMEOUT_SECONDS" bluetoothctl connect \
            "$gateway_address" "$HFP_AG_UUID" >/dev/null 2>&1 || true
    fi
}
trap reconnect_if_needed EXIT

disconnect_result=$(timeout "$HELPER_TIMEOUT_SECONDS" bluetoothctl disconnect \
    "$gateway_address" "$HFP_AG_UUID" 2>&1) || fail "profile_disconnect_failed"
printf '%s\n' "$disconnect_result" | rg -q 'Successful disconnected' || \
    fail "profile_disconnect_failed"
disconnected=true
connect_result=$(timeout "$HELPER_TIMEOUT_SECONDS" bluetoothctl connect \
    "$gateway_address" "$HFP_AG_UUID" 2>&1) || fail "profile_reconnect_failed"
printf '%s\n' "$connect_result" | rg -q 'Connection successful' || \
    fail "profile_reconnect_failed"
disconnected=false
trap - EXIT

printf 'HFP_RECOVERY=PASS profile_cycle=1 pairing_changed=0\n'
