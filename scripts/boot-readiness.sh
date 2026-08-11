#!/usr/bin/env bash
set -u

failures=0

pass() {
    printf 'PASS %s\n' "$1"
}

fail() {
    printf 'FAIL %s\n' "$1"
    failures=$((failures + 1))
}

check_value() {
    local label=$1
    local expected=$2
    shift 2
    local actual
    actual=$("$@" 2>/dev/null) || actual=
    if [[ $actual == "$expected" ]]; then
        pass "$label"
    else
        fail "$label"
    fi
}

check_value daemon-enabled enabled systemctl --user is-enabled analogconnectd.service
check_value daemon-active active systemctl --user is-active analogconnectd.service
check_value daemon-home-protection read-only \
    systemctl --user show analogconnectd.service -p ProtectHome --value
check_value daemon-imsg-write-paths \
    "$HOME/.local/share/imsg $HOME/.local/state/imsg" \
    systemctl --user show analogconnectd.service -p ReadWritePaths --value
check_value bluetooth-enabled enabled systemctl is-enabled bluetooth.service
check_value bluetooth-active active systemctl is-active bluetooth.service
check_value pipewire-enabled enabled systemctl --user is-enabled pipewire.service
check_value wireplumber-enabled enabled systemctl --user is-enabled wireplumber.service
check_value avahi-enabled enabled systemctl is-enabled avahi-daemon.service
check_value avahi-active active systemctl is-active avahi-daemon.service

linger=$(loginctl show-user "$(id -un)" -p Linger --value 2>/dev/null) || linger=
if [[ $linger == yes ]]; then
    pass user-lingering
else
    fail user-lingering
fi

environment_file=${ANALOGCONNECT_ENV_FILE:-"$HOME/.config/analogconnect/daemon.env"}
if [[ -f $environment_file && $(stat -c %a "$environment_file" 2>/dev/null) == 600 ]]; then
    pass environment-permissions
else
    fail environment-permissions
fi

if awk -F= '$1 == "ANALOGCONNECT_LISTEN_ADDR" && $2 == "0.0.0.0:8787" {found=1}
        END {exit !found}' "$environment_file" 2>/dev/null; then
    pass address-independent-listener
else
    fail address-independent-listener
fi

if awk -F= '
    $1 == "ANALOGCONNECT_API_TOKEN" && length($2) > 0 {token=1}
    $1 == "ANALOGCONNECT_TLS_CERT_PATH" && length($2) > 0 {cert=1}
    $1 == "ANALOGCONNECT_TLS_KEY_PATH" && length($2) > 0 {key=1}
    END {exit !(token && cert && key)}' "$environment_file" 2>/dev/null; then
    pass required-private-settings-present
else
    fail required-private-settings-present
fi

if ((failures == 0)); then
    printf 'BOOT_READINESS=PASS\n'
    exit 0
fi

printf 'BOOT_READINESS=FAIL failures=%d\n' "$failures"
exit 1
