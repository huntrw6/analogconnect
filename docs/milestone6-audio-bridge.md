# Milestone 6 — Audio Bridge Foundation

## Scope

This slice implements the in-memory portion of the bidirectional call-audio bridge
and the process boundary that binds a live PipeWire SCO source/sink pair. It does
not yet transfer frames over the Android network transport or record samples.

It also implements the versioned framed-PCM diagnostic wire format and a bounded
sequence-aware jitter buffer required by ADR 0002. This is a benchmark baseline,
not the selected production transport.

## Format invariants

- HFP wideband PCM: 16 kHz, mono, 120 samples per 7.5 ms frame.
- HFP narrowband PCM: 8 kHz, mono, 60 samples per 7.5 ms frame.
- Frame construction rejects mismatched sample counts.

## Queue policy

Uplink and downlink use independent bounded queues. When a producer outruns a
consumer, the oldest frame is discarded. This intentionally prefers a small audio
gap over accumulating conversational latency.

Only aggregate counters are observable:

- current depth
- frames enqueued/dequeued
- frames dropped
- maximum observed in-memory queue latency

Sample buffers are never serialized, logged, or included in `Debug` output.

## Diagnostic wire framing

- Fixed `ACAP` magic and protocol version 1.
- Explicit narrowband or wideband HFP format identifier.
- Big-endian sequence and capture-time fields.
- Sequence numbers are restricted to the shared non-negative signed-63-bit range
  so Rust and Android ordering semantics remain identical.
- Little-endian signed 16-bit PCM payload with exact format-derived length.
- Unknown versions/formats, reserved-bit changes, and malformed payloads fail closed.

The jitter buffer reorders by sequence, waits for a bounded target depth, drops
far-future frames on overflow, rejects late/duplicate frames, and reports only
aggregate received/emitted/lost/reordered-health counters.

Playout is an explicit `tick`, not an unpaced queue poll. Before target depth is
reached a tick does not advance state. Once started, callers tick exactly once per
7.5 ms HFP frame; an absent frame advances the playout sequence and counts one
missing-frame underflow even if the queue is empty. A later arrival for that slot
is then correctly classified as late. Rust and Android implement the same rule.

`FramedPcmMediaBridge` is the transport-neutral diagnostic seam. It encodes
PipeWire downlink frames into ACAP packets and accepts uplink packet bytes only
after strict decode and active-format validation, then feeds the bounded jitter
buffer for ordered PipeWire playout. Its diagnostics contain only format and
aggregate jitter counters. A future RTP or WebRTC adapter can replace packet
carriage without changing PipeWire framing or playout policy.

## Synthetic benchmark

```bash
cargo run --release --quiet --bin audio-bench
```

The benchmark moves synthetic silence through the uplink queue and reports only
throughput, real-time multiple, and drop count.

## Privacy-safe SCO node validation

During an active test call:

```bash
cargo run --quiet --bin sco-validate
```

The locator selects only PipeWire nodes whose official `factory.name` is
`api.bluez5.sco.source` or `api.bluez5.sco.sink`. It ignores address, path, name,
and description properties. It retains `object.serial`, because `pw-cat --target`
accepts a node serial or name rather than the transient global object ID. Output
contains only whether exactly one pair exists; serials are not printed.
The runtime observer distinguishes a cleanly absent pair (`inactive`) from a valid
pair (`sco_active`); malformed, partial, or duplicate state fails closed as an
audio error.
`pw-dump` is bounded to two seconds and 16 MiB; stdout is drained only into
transient memory, stderr is discarded, and a stalled child is killed and reaped
without including snapshot data in the error.

## Live PipeWire process boundary

`PwCatSession` starts one capture process against the SCO source and one playback
process against the SCO sink. Both use raw signed 16-bit mono PCM, an explicit
8 kHz or 16 kHz HFP rate, and a 7.5 ms requested latency. Audio crosses anonymous
stdin/stdout pipes only. Stderr is discarded to prevent private PipeWire metadata
from entering application logs, a partial startup tears down the first process,
and dropping the session terminates and reaps both children.

This boundary does not invoke a shell and accepts only locator-produced numeric
serials plus one of the two fixed HFP formats. No automated test starts a real
audio stream.

The adjacent PCM adapters reassemble short reads into exact 7.5 ms frames, assign
monotonic downlink sequence numbers, reject an EOF that bisects a frame, and write
uplink samples only when their format matches the active stream. Conversion is
explicitly little-endian to match the framed diagnostic codec and Raspberry Pi.
The session can transfer both framed directions into independently movable stream
owners so capture and playback workers do not share a lock or audio buffer.

## Evidence

- `VERIFIED_AUTOMATED`: format, redaction, overflow, direction independence, and
  aggregate API tests pass.
- `VERIFIED_AUTOMATED`: binary round-trip, malformed-packet rejection, pre-playout
  reordering, missing/late/duplicate accounting, and bounded future latency pass.
- `VERIFIED_AUTOMATED`: dependency-free Rust and Android API-27 codecs match the
  same cross-platform golden header vector and both reject malformed input.
- `VERIFIED_AUTOMATED`: the Android jitter buffer matches the Pi's startup,
  reorder, missing, duplicate, late, and overflow behavior under synthetic tests.
- `VERIFIED_AUTOMATED`: pre-start polling does not create loss, while an empty
  post-start playout tick counts one underflow and makes a later packet for that
  elapsed slot late on both Rust and Android.
- `VERIFIED_AUTOMATED`: SCO discovery fixtures return only a numeric source/sink
  serial pair, ignore private properties, and fail closed on missing or ambiguous
  nodes.
- `VERIFIED_AUTOMATED`: runtime audio status requires both an exact SCO node pair
  and WirePlumber's live AudioGateway transport state, preventing stale nodes
  alone from authorizing call media.
- `VERIFIED_HARDWARE`: active iPhone calls produced exactly one source/sink pair
  and an active gateway transport.
- `FAILED`: SCO remained active after one Pi-originated hangup and required an
  HFP-profile cycle; automatic recovery for this condition remains pending.
- `VERIFIED_AUTOMATED`: capture/playback command construction binds the correct
  source/sink direction for both HFP rates, fixes mono signed-16-bit framing and
  7.5 ms latency, and rejects non-HFP formats.
- `VERIFIED_AUTOMATED`: PCM adapters reconstruct partial reads, preserve signed
  little-endian samples, sequence consecutive frames, reject truncated input and
  format changes, and expose no sample values in errors.
- `VERIFIED_AUTOMATED`: the transport-neutral framed bridge round-trips downlink
  packets, reorders uplink packets, rejects malformed or mid-session format
  changes, validates jitter configuration, and exposes no samples in diagnostics.
- `VERIFIED_AUTOMATED`: the Android API-27 audio-device adapter compiles for 8/16
  kHz mono 7.5 ms frames, uses voice-communication capture/playback, restores prior
  routing on stop, conditionally enables platform echo/noise processing, cleans up
  partial starts, and makes stop/close idempotent with fixed redacted failures.
- `UNKNOWN`: real-phone microphone permission, device initialization, earpiece
  routing, echo/noise processing effectiveness, and sustained frame I/O.
- `UNKNOWN`: real-call `pw-cat` process operation, codec conversion, Android
  transport, end-to-end latency, and human-confirmed intelligibility.
