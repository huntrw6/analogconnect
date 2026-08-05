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
`PhoneAccount` descriptor. An explicit experimental switch registers or removes
the account, and a separate button opens Android's calling-account settings. The
account does not become usable merely by registering it: Android still requires
the user to enable it. If Android invokes the service before authenticated routing
is implemented, it returns a fixed failure without reading or exposing a dialed
address. The software path for an enabled outgoing account now validates the
`tel:` target without logging it, decrypts the existing enrollment token through
Android Keystore, sends the pinned-TLS dial command, and reduces aggregate daemon
state into dialing, active, and disconnected Telecom states. Once active, it
obtains a one-time media credential, starts call audio, follows Android's
earpiece/speaker route, and tears audio down with the Telecom call.

- `VERIFIED_AUTOMATED`: the inactive service and phone-account descriptor compile
  against Android API 27 and package in the signed APK.
- `VERIFIED_AUTOMATED`: registration/removal and the calling-account settings
  action compile behind explicit UI controls; no account is registered by install,
  launch, enrollment, or upgrade.
- `UNKNOWN`: whether this phone's vendor dialer presents managed third-party calls
  correctly, including incoming UI, audio routing, call history, and emergency-call
  isolation.
- `VERIFIED_AUTOMATED`: strict outgoing target validation, redacted failures,
  authenticated command construction, state polling, DTMF/hang-up concurrency,
  and automatic media-session lifecycle compile and package on API 27.
- `UNKNOWN`: Contacts-to-AnalogBridge dialing and automatic Telecom-owned call
  audio on the real phone; this build is intentionally not deployed while the
  operator is unavailable.

## Next safe steps

1. Hardware-test registration, removal, and the vendor calling-account settings.
2. Add aggregate incoming-call observation and create a Telecom incoming connection.
3. Hardware-test outgoing dial, DTMF, hang-up, call audio, and speaker routing.
4. Hardware-test alongside ordinary cellular and emergency-call behavior before
   making the integration a default.

## Contacts foundation

The API-27 APK now declares a dedicated `com.analogconnect.contacts` account type,
an authenticator protected by Android's account-authenticator binding permission,
and a read-only Contacts sync adapter protected by the sync-adapter binding
permission. The authenticator refuses account creation and the adapter skips sync
until an authenticated snapshot contract, runtime contact permission flow, atomic
replacement, and account-removal cleanup are implemented.

- `VERIFIED_AUTOMATED`: authenticator and sync-adapter metadata compile, package,
  and reference the system Contacts authority without requesting upload support.
- `DOCUMENTED`: installing or launching the APK does not request Contacts access,
  create an account, or write a contact.
- `UNKNOWN`: built-in Contacts display, deduplication, caller matching, account
  removal, and Contacts-to-Telecom dialing on the target phone.
