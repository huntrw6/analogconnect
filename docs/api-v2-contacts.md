# Contacts API v2

`POST /api/v2/contacts/search` returns a bounded authenticated contact page from
the latest PBAP snapshot. Search text stays in the JSON body so names never enter
URLs, access logs, diagnostics, or metrics. Responses set `Cache-Control:
no-store` and `Pragma: no-cache`.

Each item contains an optional display name and one or more private presentation
phone numbers. Android keeps the original number as the call/message target even
when it renders the matched name. Conversation summaries similarly carry both
`display_address` and nullable `display_name`; presentation never replaces routing
identity.

The daemon pulls a full contact snapshot at startup and every fifteen minutes.
A failed PBAP refresh preserves the last in-memory successful snapshot and emits
only a fixed aggregate warning. The snapshot is intentionally memory-only at this
stage, so a daemon restart fails closed to numbers until PBAP refresh succeeds.
