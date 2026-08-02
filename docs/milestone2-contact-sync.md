# Milestone 2 — Contact Synchronization Design

## Scope

This milestone implements the software path from PBAP output to local contact
search and caller matching. It deliberately does not run a real phonebook pull,
expose contact records over the current unauthenticated API, or deploy a service.

## Data flow

```text
imsg contacts --raw
        |
        | sensitive stdout held in memory only
        v
privacy boundary parser
        |
        v
Contact + PhoneNumber domain records
        |
        | one SQLite transaction
        v
contacts + contact_phones
        |
        +--> case-insensitive name search
        +--> exact/unique-suffix caller matching
        +--> aggregate-only API summary
```

## Safety properties

- Contact payloads are never placed in errors or routine logs.
- `Debug` output redacts names and all phone-number content.
- A failed pull cannot erase the last successful snapshot.
- Caller matching returns no result when a suffix is ambiguous.
- SQL wildcard characters in name queries are treated literally.
- The SQLite file is forced to mode `0600` on Unix.
- Synthetic test values are assembled at runtime so no telephone number is
  stored in the repository.

## Evidence

- `VERIFIED_AUTOMATED`: parser, redaction, normalization, persistence, search,
  matching, failure preservation, and aggregate API tests pass.
- `DOCUMENTED`: the locally installed `imsg` 0.3.1 implementation renders full
  contacts as a name followed by indented phone-number lines; `--raw` disables
  its own E.164 normalization.
- `UNKNOWN`: real-iPhone full-phonebook output compatibility and performance.

## Hardware validation boundary

Hardware validation must use aggregate assertions only. A future test helper
is provided as `pbap-validate`; it consumes contact output through stdin, reports
counts and parser status, and discards raw payloads without writing them to disk
or terminal.

Run only with explicit hardware approval and the iPhone unlocked:

```bash
set -o pipefail
imsg contacts --raw | cargo run --quiet --bin pbap-validate
```

The expected output contains only `PASS`, a contact count, and a phone-field count.
