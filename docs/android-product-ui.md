# Android product UI

Status: `VERIFIED_AUTOMATED` on API 27; physical layout inspection used isolated demo data.

## Structure

- `WelcomeActivity` owns one-time beginner onboarding.
- Messages, Calls, Contacts, and Settings use a fixed bottom navigation bar.
- `ConversationController` and `ContactController` keep network work outside Activities.
- `OfflineCache` stores only last-known-good read models, encrypted with an Android Keystore
  AES-GCM key. It is never used as send authority or recipient routing authority.
- `DemoFixtures` is selected before API/cache access and never writes the production cache or Pi
  store.
- `AnalogNotifications` owns API-27 channels, private/group message deep links, and incoming-call
  notification presentation.

Appearance is explicit (`device`, `light`, or `dark`). Group and ambiguous conversations never
construct a send composer. Developer diagnostics remain reachable through Settings without
appearing in the beginner journey.

Physical screenshots are local-only under ignored `artifacts/screenshots/`.
