# Android system integration

## Product direction

AnalogBridge will use a hybrid Android integration:

- **Contacts:** publish iPhone contacts through a dedicated Android account and
  sync adapter so they appear in the built-in Contacts app. Contact rows must be
  removed cleanly when the account is disconnected, and no contact data may enter
  logs or repository fixtures.
- **Calls:** evaluate a managed Android Telecom `PhoneAccount` and
  `ConnectionService` so the system Phone UI can present AnalogBridge calls. The
  existing in-app controls remain the fallback because vendor Android 8.1 dialers
  may not support a non-cellular call provider consistently.
- **Messages:** build an AnalogBridge conversation UI and notifications. The stock
  Messages app assumes Android's cellular SMS provider and cannot reliably route
  outgoing messages through the iPhone/Pi bridge.

## Current slice

The APK declares an `AnalogConnectionService` protected by Android's
`BIND_TELECOM_CONNECTION_SERVICE` permission and can construct a stable managed
`PhoneAccount` descriptor. It is intentionally not registered with
`TelecomManager`; if Android invokes the service unexpectedly, it returns a fixed
failure without reading or exposing a dialed address.

- `VERIFIED_AUTOMATED`: the inactive service and phone-account descriptor compile
  against Android API 27 and package in the signed APK.
- `DOCUMENTED`: Android Telecom registration requires an explicit future user
  action and target-phone validation before AnalogBridge may handle calls.
- `UNKNOWN`: whether this phone's vendor dialer presents managed third-party calls
  correctly, including incoming UI, audio routing, call history, and emergency-call
  isolation.

## Next safe steps

1. Add an explicit experimental Phone integration toggle.
2. Register the account only after confirmation and open Android's calling-account
   settings for the user to enable it.
3. Bridge aggregate incoming/active/ended state into a Telecom `Connection` without
   placing phone numbers in diagnostics.
4. Route Telecom answer, reject, disconnect, and DTMF callbacks through the existing
   authenticated call-command API.
5. Hardware-test alongside ordinary cellular and emergency-call behavior before
   making the integration a default.
