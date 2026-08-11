#!/usr/bin/env bash
set -euo pipefail

workspace=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
apk="$workspace/android/.build/analogconnect-debug.apk"

command -v adb >/dev/null
[[ -f "$apk" ]] || "$workspace/android/build.sh"
device_count=$(adb devices | awk 'NR > 1 && $2 == "device" { count++ } END { print count + 0 }')
[[ "$device_count" -eq 1 ]] || {
    echo "expected exactly one authorized Android device; found $device_count" >&2
    exit 1
}
adb install -r "$apk"
adb shell pm grant com.analogconnect.client android.permission.RECORD_AUDIO
adb shell am force-stop com.analogconnect.client
adb shell monkey -p com.analogconnect.client -c android.intent.category.LAUNCHER 1 >/dev/null
echo "ANDROID_INSTALL=PASS"
