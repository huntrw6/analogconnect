# Milestone 6 — Audio Bridge Foundation

## Scope

This slice implements the in-memory portion of the bidirectional call-audio bridge.
It does not bind PipeWire nodes, capture a call, record samples, or choose the
Android network codec.

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

## Synthetic benchmark

```bash
cargo run --release --quiet --bin audio-bench
```

The benchmark moves synthetic silence through the uplink queue and reports only
throughput, real-time multiple, and drop count.

## Evidence

- `VERIFIED_AUTOMATED`: format, redaction, overflow, direction independence, and
  aggregate API tests pass.
- `UNKNOWN`: live PipeWire capture/playback, codec conversion, Android transport,
  end-to-end latency, and human-confirmed intelligibility.
