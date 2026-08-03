# AnalogConnect Android client

This is a dependency-free Android 8.1 (API 27) client foundation. It currently
provides local enrollment settings and a privacy-safe daemon health/status check.
The bearer token is encrypted with an Android Keystore AES/GCM key and neither
the token nor API response bodies are logged.

The Raspberry Pi daemon is still loopback-bound, so phone-to-Pi networking is
not enabled in the current milestone. Installing and launching this APK checks
only Android toolchain and device compatibility.

## Build

```bash
ANDROID_SDK_ROOT=/home/operat/Android/Sdk ./android/build.sh
```

The unsigned intermediate, generated debug key, and final APK remain under
`android/.build/`, which is ignored by Git. The installable artifact is
`android/.build/analogconnect-debug.apk`.

## Install

With one authorized Android device attached:

```bash
adb install -r android/.build/analogconnect-debug.apk
```

Do not commit the generated keystore or any enrollment token.
