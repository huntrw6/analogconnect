# AnalogConnect Android client

This is a dependency-free Android 8.1 (API 27) client foundation. It currently
provides local enrollment settings and a privacy-safe daemon health/status check.
The bearer token is encrypted with an Android Keystore AES/GCM key and neither
the token nor API response bodies are logged. Token entry is hidden by default
and has an explicit visibility switch for easier entry on physical keyboards.
Non-loopback endpoints require HTTPS and an explicitly enrolled SHA-256 leaf
certificate pin. The pin is not a credential, but diagnostics still redact it.

The APK declares microphone/audio-routing permissions for the call-audio milestone,
but the current UI never requests microphone permission or starts capture. The
audio device adapter remains inactive until a short-lived media transport exists.

The Raspberry Pi daemon defaults to loopback. It permits an explicitly configured
LAN listener only when both TLS certificate paths are present; the Android client
then requires the enrolled leaf pin and a matching certificate hostname or IP SAN.
See `docs/control-plane-security.md` for provisioning and enrollment.

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
