# Milestone 6 — Audio Bridge Foundation

## Scope

This slice implements the in-memory portion of the bidirectional call-audio bridge.
It does not bind PipeWire nodes, capture a call, record samples, or choose the
Android network codec.

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

## Synthetic benchmark

```bash
cargo run --release --quiet --bin audio-bench
```

The benchmark moves synthetic silence through the uplink queue and reports only
throughput, real-time multiple, and drop count.

## Evidence

- `VERIFIED_AUTOMATED`: format, redaction, overflow, direction independence, and
  aggregate API tests pass.
- `VERIFIED_AUTOMATED`: binary round-trip, malformed-packet rejection, pre-playout
  reordering, missing/late/duplicate accounting, and bounded future latency pass.
- `VERIFIED_AUTOMATED`: dependency-free Rust and Android API-27 codecs match the
  same cross-platform golden header vector and both reject malformed input.
- `VERIFIED_AUTOMATED`: the Android jitter buffer matches the Pi's startup,
  reorder, missing, duplicate, late, and overflow behavior under synthetic tests.
- `UNKNOWN`: live PipeWire capture/playback, codec conversion, Android transport,
  end-to-end latency, and human-confirmed intelligibility.
