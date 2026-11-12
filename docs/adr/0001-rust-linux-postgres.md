# ADR 0001: Rust, Linux, PostgreSQL, and the authoritative server

- **Status:** Accepted by project decisions DD-043–DD-050
- **Date:** 2026-11-12
- **Scope:** Phase 1 platform and service boundaries

## Context

SimLifeClone needs a deterministic simulation that can run without a renderer, persist independent worlds, and serve browser and future clients through one stable protocol. Phase 1 is single-player gameplay on a server-hosted architecture. Multiplayer sharing and cross-world transfers are later work.

## Decisions

1. Implement the authoritative simulation engine and initial server in Rust and run the server on Linux.
2. Keep the simulation core headless and independent of HTTP, WebSocket, PostgreSQL, browser, renderer, wall-clock, and asynchronous runtime dependencies.
3. Use PostgreSQL for durable world metadata, commands/events, and checkpoints. Keep active simulation state in memory; do not issue database operations for every organism or tick.
4. Expose a versioned JSON API over HTTP and WebSocket. The server validates all state-changing commands and remains the source of truth.
5. Build the browser client first. Android and optional Godot clients must be able to use the same public API.
6. Keep planetary worlds independent, with separate clocks, populations, histories, and random streams. No automatic inter-world migration is part of Phase 1.
7. Preserve future copy-based genome transfer at the boundary, but defer its gameplay workflow to Phase 2.

## Consequences

- Phase 1 uses a modular Rust workspace and a single server executable; microservices are not required.
- PostgreSQL migrations and checkpoint recovery are part of the deployment contract.
- Client observations are read-only and cannot consume simulation randomness or change simulation fidelity.
- Simulation, protocol, persistence, server, and presentation concerns remain separate.
- Authentication, exact server scheduling policy, checkpoint codec/cadence, and production deployment details remain separate implementation decisions. This ADR does not approve biological mechanics.

## References

- [Functional specification](../SimLife_Phase1_Functional_Specification.md), DD-043–DD-050
- [System architecture specification](../SimLife_System_Architecture_and_Implementation_Specification.md), §§1–4 and 18
