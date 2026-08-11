# Messaging API v2 design

## Purpose

API v2 will support the real conversation interface without turning terminal
output into an application protocol or placing telephone data in URLs and logs.
Version 1 remains available for the installed engineering client while v2 is
built and migrated.

## Source of truth

- `DOCUMENTED`: imsg 0.3.1 owns an encrypted SQLCipher store with public Rust
  queries for thread summaries, paginated messages, outgoing states, and message
  lookup.
- `DOCUMENTED`: those typed rows include private addresses and message bodies, so
  they may cross only the authenticated pinned-TLS response boundary and must not
  implement revealing `Debug` output in AnalogConnect.
- `INFERRED`: linking the imsg config/keyring/store crates behind an
  AnalogConnect-owned repository interface is safer and more stable than parsing
  `imsg threads` or `imsg list` display output.
- `VERIFIED_AUTOMATED`: the typed adapter opens a synthetic encrypted imsg store
  and performs concurrent thread/history reads with stable mapping and redacted
  diagnostics.
- `UNKNOWN`: concurrent reads against the real store while the live imsg broker
  updates its WAL database.

## Resources

### `GET /api/v2/conversations`

Returns a bounded newest-first page of conversation summaries:

```json
{
  "items": [
    {
      "conversation_id": "opaque-daemon-scoped-id",
      "display_address": "private-response-value",
      "is_group": true,
      "reply_supported": false,
      "latest_unix_millis": 0,
      "message_count": 0,
      "unread_count": 0,
      "latest_outgoing_state": null
    }
  ],
  "next_cursor": null
}
```

The response requires authentication and `Cache-Control: no-store`. Address and
participant/contact display values are private response data and never appear in paths,
queries, diagnostics, metrics, or test fixtures.

### `POST /api/v2/conversations/messages`

Uses a bounded JSON body rather than a private URL component:

```json
{
  "conversation_id": "opaque-daemon-scoped-id",
  "cursor": null,
  "limit": 50
}
```

The response contains newest-first message items with an opaque local ID,
direction, private peer/sender display value, timestamp, body, read state, and outgoing state. It never exposes MAP
handles, database paths, Bluetooth addresses, or sync internals.

### `POST /api/v2/messages`

Extends the current send operation with a required 128-bit random operation ID.
The durable implementation records queued, sending, sent-unconfirmed,
sent-confirmed, failed-retryable, failed-permanent, or unknown. An uncertain
outcome cannot be retried automatically until sent-folder reconciliation resolves
it.

## Identity and pagination

- Conversation IDs are random daemon-scoped aliases mapped to canonical private
  participant-set keys in memory. Restart invalidates them and clients refetch
  the conversation page.
- Direct and group identities prefer the opaque MAP message-listing
  `conversation_id`. The bMessage participant set/peer is a compatibility fallback;
  identities are never guessed from timestamps or message bodies.
- Group replies remain disabled until multi-recipient MAP push and sent-folder
  reconciliation are implemented and hardware-verified.
- Message IDs are opaque encodings of local store identity, not MAP handles.
- Cursors are opaque, bounded, and validated before repository access.
- Page size defaults to 50 and is capped at 100.
- Ordering uses `(timestamp, local identity)` so equal timestamps cannot skip or
  duplicate rows across pages.

## Privacy and failure behavior

- Authentication occurs before private JSON parsing or repository access.
- Private responses set `Cache-Control: no-store` and `Pragma: no-cache`.
- Errors contain only fixed codes such as `message_store_unavailable`,
  `conversation_expired`, `sync_backing_off`, and `invalid_cursor`.
- No raw imsg, SQLCipher, keyring, SQLite, MAP, or broker error crosses the API.
- Android keeps conversation pages in memory and does not write message bodies to
  logs, saved-instance state, notifications without consent, or repository tests.

## Migration sequence

1. ~~Add synthetic domain rows, opaque-ID registry, cursor codec, and repository
   contract tests.~~ `VERIFIED_AUTOMATED`
2. ~~Add an in-memory repository to API tests and implement the authenticated v2
   routes with no-store headers.~~ `VERIFIED_AUTOMATED`
3. ~~Add the imsg config/keyring/store adapter and synthetic encrypted-store tests.~~
   `VERIFIED_AUTOMATED`
4. Validate real-store concurrent access and fixed errors without displaying real
   data.
5. ~~Build Android conversation controllers and screens against a fake v2 client.~~
   `VERIFIED_AUTOMATED`
6. Hardware-test aggregate counts/order first, then visually validate private
   content on-device without capturing or committing it.
