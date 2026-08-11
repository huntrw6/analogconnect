# Developer diagnostics

`Settings → Developer Tools` provides aggregate connection/component state and the pending
hardware-validation checklist. Raw legacy controls remain one level deeper.

Run `scripts/diagnostic-bundle.sh` for a sanitized archive. It includes build/version data,
aggregate service state, unauthenticated health status, migration names, feature evidence, and
pending tests. It deliberately excludes journals, Bluetooth addresses, contacts, phone numbers,
message content, tokens, keys, and pairing data.

Run `scripts/validate.sh` for the complete Rust/vendor/Android/lint gate and
`scripts/install-android.sh` to upgrade and launch exactly one authorized Android device.
