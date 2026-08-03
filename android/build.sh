#!/usr/bin/env bash
set -euo pipefail

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
sdk_root=${ANDROID_SDK_ROOT:-/home/operat/Android/Sdk}
platform_jar="$sdk_root/platforms/android-27/android.jar"
r8_jar="$sdk_root/tools/r8-8.3.41/r8lib.jar"
build_dir="$script_dir/.build"
generated_dir="$build_dir/generated"
classes_dir="$build_dir/classes"
dex_dir="$build_dir/dex"
test_classes_dir="$build_dir/test-classes"
unsigned_apk="$build_dir/analogconnect-unsigned.apk"
aligned_apk="$build_dir/analogconnect-aligned.apk"
output_apk="$build_dir/analogconnect-debug.apk"
keystore="$build_dir/debug.keystore"

for required in aapt javac jar java zip zipalign apksigner keytool; do
    command -v "$required" >/dev/null || {
        echo "missing required command: $required" >&2
        exit 1
    }
done

for required_file in "$platform_jar" "$r8_jar"; do
    [[ -f "$required_file" ]] || {
        echo "missing required file: $required_file" >&2
        exit 1
    }
done

rm -rf -- "$generated_dir" "$classes_dir" "$dex_dir" "$test_classes_dir"
rm -f -- "$unsigned_apk" "$aligned_apk" "$output_apk"
mkdir -p -- "$generated_dir" "$classes_dir" "$dex_dir" "$test_classes_dir"

javac -d "$test_classes_dir" \
    "$script_dir/src/com/analogconnect/client/Endpoint.java" \
    "$script_dir/src/com/analogconnect/client/AudioPacketCodec.java" \
    "$script_dir/src/com/analogconnect/client/AudioJitterBuffer.java" \
    "$script_dir/src/com/analogconnect/client/AudioDeviceConfig.java" \
    "$script_dir/src/com/analogconnect/client/CertificatePin.java" \
    "$script_dir/test/com/analogconnect/client/EndpointTest.java" \
    "$script_dir/test/com/analogconnect/client/AudioPacketCodecTest.java" \
    "$script_dir/test/com/analogconnect/client/AudioJitterBufferTest.java" \
    "$script_dir/test/com/analogconnect/client/CertificatePinTest.java" \
    "$script_dir/test/com/analogconnect/client/AudioDeviceConfigTest.java"
java -cp "$test_classes_dir" com.analogconnect.client.EndpointTest
java -cp "$test_classes_dir" com.analogconnect.client.AudioPacketCodecTest
java -cp "$test_classes_dir" com.analogconnect.client.AudioJitterBufferTest
java -cp "$test_classes_dir" com.analogconnect.client.CertificatePinTest
java -cp "$test_classes_dir" com.analogconnect.client.AudioDeviceConfigTest

aapt package -f -m -J "$generated_dir" -M "$script_dir/AndroidManifest.xml" \
    -S "$script_dir/res" -I "$platform_jar"

mapfile -t java_sources < <(find "$script_dir/src" "$generated_dir" -name '*.java' -type f | sort)
javac -source 8 -target 8 -bootclasspath "$platform_jar" \
    -d "$classes_dir" "${java_sources[@]}"

jar cf "$build_dir/classes.jar" -C "$classes_dir" .
java -cp "$r8_jar" com.android.tools.r8.D8 --min-api 27 \
    --lib "$platform_jar" --output "$dex_dir" "$build_dir/classes.jar"

aapt package -f -M "$script_dir/AndroidManifest.xml" -S "$script_dir/res" \
    -I "$platform_jar" -F "$unsigned_apk"
(cd "$dex_dir" && zip -q -j "$unsigned_apk" classes.dex)
zipalign -f 4 "$unsigned_apk" "$aligned_apk"

if [[ ! -f "$keystore" ]]; then
    keytool -genkeypair -keystore "$keystore" -storepass android -keypass android \
        -alias androiddebugkey -dname 'CN=Android Debug,O=Android,C=US' \
        -keyalg RSA -keysize 2048 -validity 10000 >/dev/null 2>&1
fi

apksigner sign --ks "$keystore" --ks-pass pass:android --key-pass pass:android \
    --out "$output_apk" "$aligned_apk"
apksigner verify --verbose "$output_apk"
echo "ANDROID_BUILD=PASS apk=$output_apk"
