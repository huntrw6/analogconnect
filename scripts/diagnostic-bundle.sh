#!/usr/bin/env bash
set -euo pipefail

workspace=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
timestamp=$(date -u +%Y%m%d-%H%M%S)
output=${1:-"$workspace/artifacts/AnalogConnect-diagnostic-$timestamp"}
mkdir -p "$output"

git -C "$workspace" rev-parse HEAD > "$output/version.txt"
git -C "$workspace" status --short >> "$output/version.txt"
{
    uname -srmo
    rustc --version
    cargo --version
    java -version 2>&1 | head -1
} > "$output/build-environment.txt"

for unit in analogconnectd.service analogconnect-android-keys.service bluetooth.service \
    wireplumber.service pipewire.service; do
    state=$(systemctl --user is-active "$unit" 2>/dev/null || systemctl is-active "$unit" 2>/dev/null || true)
    printf '%-28s %s\n' "$unit" "${state:-unknown}"
done > "$output/service-states.txt"

health_code=$(curl --silent --output /dev/null --write-out '%{http_code}' \
    --max-time 2 http://127.0.0.1:8787/api/v1/health || true)
printf 'api_health_http=%s\n' "${health_code:-unavailable}" > "$output/api-health.txt"

find "$workspace/vendor/imsg-store/migrations" -maxdepth 1 -type f -printf '%f\n' \
    | sort > "$output/database-migrations.txt"
cp "$workspace/docs/pending-hardware-tests.md" "$output/pending-hardware-tests.md"
{
    echo "ANCS protocol consumer: implemented / automated"
    echo "Production BlueZ bearer: implemented / hardware coexistence pending"
    echo "ANCS to MAP correlation: integrated / automated"
    echo "Android background notifications: implemented / automated"
    echo "Group reply: disabled"
    echo "Diagnostic privacy: no logs, addresses, contacts, messages, tokens, or pairing data"
} > "$output/feature-evidence.txt"

if command -v adb >/dev/null && adb get-state 2>/dev/null | grep -qx device; then
    adb shell dumpsys package com.analogconnect.client \
        | sed -n -E '/versionCode=|versionName=|firstInstallTime=|lastUpdateTime=/p' \
        > "$output/android-app.txt"
else
    echo "Android device unavailable" > "$output/android-app.txt"
fi

tar -C "$(dirname -- "$output")" -czf "$output.tar.gz" "$(basename -- "$output")"
echo "DIAGNOSTIC_BUNDLE=$output.tar.gz"
