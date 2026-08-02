# ADR 0001: Pi-to-Android Control Protocol

- Status: Accepted for Milestone 1
- Date: 2026-08-02

## Context

The Android client needs snapshots, commands, and ordered live events without
depending on BlueZ object paths, MAP handles, or PipeWire node IDs.

## Decision

Use a versioned local protocol with:

- HTTP/JSON REST for snapshots and commands under `/api/v1`.
- WebSocket JSON events at `/api/v1/events`.
- Stable internal UUIDs for contacts, conversations, messages, calls, and audio sessions.
- An event envelope containing `protocol_version`, `event_id`, `sequence`,
  `timestamp`, `type`, `resource_id`, and `payload`.
- OpenAPI and JSON Schema artifacts generated or checked from shared Rust models.
- Enrollment and authenticated dangerous commands before Android hardware deployment.

The initial endpoints are `GET /api/v1/health` and `GET /api/v1/status`. Bluetooth
implementation details remain inside daemon adapters.

## Consequences

- REST snapshots make reconnect and reconciliation explicit.
- WebSocket sequence numbers allow clients to detect missed events and refetch state.
- JSON is easy to inspect on Android 8.1 but requires strict validation and bounded payloads.
- Protocol-breaking changes require a new versioned namespace.

## Alternatives considered

- gRPC: strong schemas, but adds Android and HTTP/2 complexity for the initial local system.
- WebSocket-only RPC: fewer transports, but makes recovery snapshots and command semantics less clear.
- Exposing D-Bus remotely: rejected because it leaks unstable implementation details and is unsuitable as the LAN security boundary.
