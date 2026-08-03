# ADR 0002: Call-Audio Transport Evaluation

- Status: Proposed; benchmark required
- Date: 2026-08-02

## Context

Call audio must bridge dynamic PipeWire SCO nodes to Android 8.1 over local Wi-Fi.
The actual slider phone's performance and audio processing behavior are not known.

## Decision

Do not select the final media transport until hardware benchmarks are complete.
Evaluate:

1. RTP with Opus as the leading small, controllable production candidate.
2. WebRTC as the reference for integrated jitter handling, echo cancellation,
   device integration, and packet-loss recovery.
3. Framed PCM over UDP only as a temporary diagnostic baseline, never as the
   production default without additional evidence.

Control signaling uses ADR 0001. Media sessions receive stable IDs and short-lived
authenticated transport credentials. Audio is streamed only and never recorded.

## Benchmark requirements

Measure one-way and round-trip latency, packet loss, underruns, overruns, CPU,
memory, echo, feedback, gain, Wi-Fi interruption recovery, SCO recreation, and
ten-minute latency growth on the actual Android phone.

## Consequences

- Milestone 1 does not take a heavyweight media dependency.
- The Pi audio bridge must isolate codec/network transport from PipeWire node discovery.
- The final choice remains `UNKNOWN` until real-device results exist.

## Current implementation evidence

- `VERIFIED_AUTOMATED`: the framed-PCM diagnostic baseline has a strict versioned
  binary codec for HFP narrowband/wideband frames and rejects malformed packets.
- `VERIFIED_AUTOMATED`: a bounded jitter buffer handles pre-playout reordering and
  aggregate loss, late, duplicate, and overflow accounting without logging samples.
- `VERIFIED_AUTOMATED`: the Rust and Android API-27 implementations share a golden
  wire-header vector, proving byte-order and field-layout interoperability.
- `VERIFIED_AUTOMATED`: media-session authorization uses distinct OS-random
  256-bit credentials, opaque 128-bit IDs, a five-minute maximum lifetime,
  monotonic expiry, immediate revocation, constant-time comparison, and redacted
  diagnostics.
- `VERIFIED_AUTOMATED`: the media registry permits one current call session and
  one claimed client, revokes replaced/ended grants, and safely releases a dropped
  client for bounded reconnection.
- `VERIFIED_AUTOMATED`: an API-27 Android counterpart validates the server's opaque
  ID, credential, and bounded lifetime without persisting or exposing them.
- `VERIFIED_AUTOMATED`: a transport-neutral bridge connects live-frame semantics
  to the diagnostic packet codec and bounded uplink jitter policy while rejecting
  malformed packets and mid-session format changes. It does not select the final
  packet carrier.
- `UNKNOWN`: live transport benchmarks and the final RTP/Opus versus WebRTC choice.
