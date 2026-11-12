# SimLifeClone — System architecture and implementation specification

**Document:** SL-SA-001  
**Version:** 0.1 — implementation proposal, 12 November 2026  
**Status:** Architecture baseline for review; technology and interface details marked **[P]** are recommendations, not previously approved design decisions.  
**Authoritative inputs:** `SimLife_Phase1_Functional_Specification.md` v0.9 (DD-001–DD-050); `SimLife_Phase1_Mechanics_Specification.md` v0.1; original `SimLife.txt` manual for documented gameplay.  
**Scope:** Phase 1 single-player browser game on Linux/Rust/PostgreSQL; deliberately preserves boundaries for Phase 2 shared multiplayer worlds and Phase 3 expanded planetary biology and private evolutionary laboratories.

## 1. Normative status and phase separation

- **[A] Approved:** Rust engine/server; Linux; PostgreSQL; versioned JSON HTTP/WebSocket API; browser-first client; independent worlds; portable genome designs; player-initiated, copy-based cross-world introduction in later phases; autonomous organisms; deterministic simulation; client-neutral engine (DD-001–DD-050).
- **[P] Proposed here:** Axum/Tokio/SQLx, frontend technology, concrete table layout, endpoints, event envelopes, checkpoint implementation, authentication, deployment topology, quotas and defaults. They are implementable starting points, not user-approved mechanics.
- **[U] Undetermined:** Biological equations and precise simulation scheduling, world dimensions, authoritative aggregate genetic encoding, planet-specific chemistry and transfer policy. Do not infer these from the original manual.

| Capability | Phase 1 required | Preserve for later | Phase 2/3 implementation |
|---|---|---|---|
| Headless deterministic simulation | Yes | Stable engine API and state versions | Evolve mechanics with versioned migration |
| Account and private world management | Yes | Multiple independent worlds and clocks | Shared world membership/roles |
| Web client | Yes | Same public API for all clients | Responsive/PWA/Android, optional Godot |
| World generation, organism editor, observation | Yes | Planet definitions separate from world instances | Exotic planet types |
| Single-player scenario/experiment tools | Yes, per functional specification | Scenario is optional world ruleset | Shared-world gameplay replaces scenario focus |
| Player transfers between worlds | No | Genome portability and lineage provenance | Explicit, authorised copy-based introductions |
| Player sandbox / private breeding lab | No distinct multiplayer feature | Private worlds and exportable genome designs | Private lab and release workflow |
| Concurrent multiplayer interventions | No | Validated ordered command path | Multi-user rights, conflicts and moderation |

## 2. Architecture and trust boundaries

```text
Browser (Phase 1)      Android/PWA (later)      Godot (optional)
       \                      |                    /
        +----------- HTTPS / WSS JSON API --------+
                            |
                Linux Rust server (Axum)
       Auth/session | world registry | commands | subscriptions
                            |
                  World supervisor / scheduler
                    |       |        |
                 World A World B   World N    (isolated actors)
                    \       |        /
                   Rust simulation-core crate
            deterministic tick; no I/O or async
                            |
                 persistence adapter (SQLx)
                            |
                        PostgreSQL
       accounts / worlds / commands / snapshots / events
```

**Trust:** browser/client data are untrusted. The API authenticates and authorises; the server validates commands against world rules; the engine validates domain invariants again. Neither client, WebSocket connection nor SQL database is the source of truth *during* a tick. The committed simulation state plus durable command/checkpoint history are authoritative for recovery.

**One executable, modular crates [P]:** deploy a single Rust server binary for Phase 1; keep engine and protocol libraries reusable. Avoid premature microservices or a second networking language.

### 2.1 Cargo workspace [P]

```text
simlifeclone/
  Cargo.toml                   # workspace, pinned dependency policy
  Cargo.lock                   # committed
  crates/
    sim-core/                  # deterministic model, pure tick API, no network/SQL
    sim-genetics/              # optional subcrate when warranted; pure functions
    sim-protocol/              # API request/response/event DTOs, version constants
    sim-persistence/           # SQLx repositories, checkpoint codec, migrations
    sim-server/                # Axum HTTP/WS, Tokio world supervisor, CLI
    sim-testkit/               # golden worlds, state hashes, fixtures, replay
  web/                         # browser frontend
  migrations/                  # numbered PostgreSQL SQL migrations
  schemas/                     # JSON Schema/OpenAPI, versioned examples
  docs/                        # ADRs, specifications, API reference
  tests/                       # black-box end-to-end and cross-platform fixtures
  deploy/                      # systemd, reverse proxy, environment examples
```

Recommended dependencies: `axum`, `tokio`, `serde`, `serde_json`, `sqlx` with PostgreSQL, `tracing`, `uuid`, `thiserror`, `tower-http`, `argon2`, `rand_core` only behind an explicitly selected deterministic PRNG implementation, `blake3` for canonical state hashes (subject to canonical encoding). Pin major versions and MSRV. Avoid runtime feature creep in `sim-core`. **No database, HTTP, clock, thread scheduler, wall time or OS RNG inside authoritative simulation steps.**

### 2.2 Core interfaces [P; illustrative Rust, not yet a compilable contract]

```rust
pub struct WorldState { /* versioned world-owned state */ }
pub struct TickInput { pub day: u64, pub commands: Vec<DomainCommand> }
pub struct TickOutput { pub events: Vec<DomainEvent>, pub state_hash: [u8; 32] }

pub fn advance_one_day(state: &mut WorldState, input: TickInput)
    -> Result<TickOutput, SimulationError>;

pub fn inspect_world(state: &WorldState, query: ObservationQuery)
    -> Result<ObservationResult, ObservationError>;
```

A more robust implementation may return a new state or stage changes before commit; **atomic tick semantics** are mandatory regardless. Observation is pure and must not advance PRNG streams or trigger population materialisation (DD-034). Use integer day/tick identifiers, not wall-clock timestamps, for simulation ordering. Simulation version is distinct from API version.

## 3. World, planet and organism domains

### 3.1 Identity model

- `planet_definition`: reusable environment template, versioned, potentially terrestrial initially; defines permissible environment parameter schema, not a live ecology.
- `world`: independent instance referencing one planet-definition version; has owner, mode, lifecycle, clock, seed, RNG streams, state/checkpoint lineage and optional scenario ruleset.
- `world_branch`: experiment branch from a checkpoint with independent clock and RNG continuation; no implicit shared state mutation.
- `organism_design`: immutable, portable, versioned genome/design revision, separate from a living organism. A design may have an author; biological traits must not encode ownership.
- `individual`: living organism identity scoped to a world, with genotype/phenotype and lineage links.
- `species_lineage`: world-local classification derived from compatibility/reproductive isolation; preserve ancestry/provenance across worlds without assuming species identity is globally equivalent.
- `introduction`: validated event creating organism(s) from a design in a target world. For future transfer, record source world, source individual/design revision, generation, destination and requester; do **not** delete or change source organism by default.

**Invariant:** a world can be simulated, paused, restored and branched independently; it never depends on another world's current state. A cross-world transfer copies a **frozen design/genome payload** through an explicit command. It is not a cross-world transaction coupling simulation ticks.

### 3.2 Extensibility for non-terrestrial worlds

Define environment capabilities and resource/metabolism descriptors rather than baking `oxygen`, `land`, `water_drinking`, `plant_food` into universal core interfaces. Phase 1 implements terrestrial trait sets; unknown hydrogen-world biochemistry is **not** specified. Do not create a fake generic biology model to claim future support. Store schema versions and reject unsupported planet/organism combinations explicitly.

### 3.3 World lifecycle [P]

`created → initialising → paused ↔ running → pausing → paused → archived` with `faulted` recovery state. `deleted` is a separate privileged operation. A running world may continue when no browser is connected (DD-048). Each world has independent speed policy, simulation day and RNG streams. Multiple private worlds are Phase 1 capability; no multiplayer shared permissions are required yet.

## 4. Execution, concurrency and determinism

### 4.1 World actor model [P]

- One exclusive simulation owner per active world (Tokio task or dedicated worker; **do not run CPU-heavy ticks on Tokio's I/O worker threads**).
- Phase 1 authoritative tick engine is **single-threaded per world** (DD-042). Different worlds may execute concurrently because they share no mutable simulation state.
- Commands enter a bounded per-world queue. The world owner assigns a monotonic accepted sequence; commands execute at a defined day boundary in canonical order. Acknowledgement means *durably accepted*, not necessarily *applied*.
- World observation reads an immutable, periodically published snapshot or lock-protected read model. Inspection never changes simulation fidelity.
- Server speed controls schedule how quickly days are advanced, not the number or order of operations within a day. Accelerated/headless runs should be possible.
- World budgets: cap active worlds, memory, CPU share, snapshot size, queued commands and event delivery. Preset dimensions are selected through benchmarks (DD-037).

### 4.2 Determinism contract [A with implementation details P]

- Define an explicit integer/fixed-point numeric scheme and overflow/rounding rules before implementing biological equations.
- Specify one deterministic PRNG algorithm, initialisation, substream derivation, consumption rules and published test vectors. Never use OS RNG inside the simulation.
- Stable traversal (e.g. sorted IDs/tile order); no hash-map iteration order as a simulation decision; deterministic tie-breaking.
- Canonical, versioned state serialisation and state hashing; exclude wall-clock metadata, UI caches and asynchronous log ordering.
- Identical seed + engine/mechanics version + initial state + ordered command log must produce identical hashes on supported machines (DD-035).
- CI must compare golden state hashes on Linux x86_64 and a second supported architecture; **cross-platform identity is a testable target, not something guaranteed merely by using Rust**.
- If mechanics change, retain old engine versions or define explicit migrations; do not silently replay historical worlds under changed algorithms.

### 4.3 Tick transaction boundary [P]

1. Drain accepted commands eligible for the next tick in durable sequence order.
2. Validate against current state; apply interventions.
3. Execute the **mechanics-spec-defined** phases; precise ecology/reproduction/environment order remains unapproved in SL-MS-001.
4. Produce canonical events, state hash and optional checkpoint.
5. Commit durable tick progress and events consistently, then publish the new observation version.

**Crash-safety nuance:** persisting every tick synchronously may be expensive. Choose an explicit durability policy: (a) durable every tick, or (b) checkpoints plus durable command/event log and deterministic replay after a crash. If (b), define exactly what constitutes committed progress and which events are reproducible. Never advertise an event as durable before its recovery basis exists.

## 5. PostgreSQL persistence design [P]

PostgreSQL stores **metadata and durable recovery state**, not every organism position on every tick. Active world simulation runs in memory. Initial relational model:

| Table | Key fields | Notes |
|---|---|---|
| `users` | `id uuid pk`, `login_name`, `password_hash`, `created_at`, `disabled_at` | Unique canonicalised login; Argon2id password hashes |
| `sessions` | `id uuid pk`, `user_id fk`, `token_hash`, `expires_at`, `revoked_at` | Opaque tokens in Secure HttpOnly cookies; no plaintext token storage |
| `planet_definitions` | `id uuid pk`, `revision int`, `name`, `schema_version`, `definition jsonb` | Composite uniqueness `(id, revision)`; immutable revisions |
| `worlds` | `id uuid pk`, `owner_user_id fk`, `planet_id`, `planet_revision`, `name`, `status`, `mode`, `seed`, `day`, `branch_id`, `mechanics_version`, `created_at` | World state summary, not organism row-per-tick |
| `world_memberships` | `(world_id, user_id) pk`, `role`, `created_at` | Phase 1 owner only; later roles |
| `world_branches` | `id uuid pk`, `world_id fk`, `parent_branch_id`, `fork_checkpoint_id`, `created_at` | Branch lineage and checkpoint reference tracking |
| `organism_designs` | `id uuid pk`, `owner_user_id fk`, `name`, `created_at` | Stable logical design |
| `organism_design_revisions` | `(design_id, revision) pk`, `genome_schema_version`, `genome jsonb`, `parent_revision`, `created_at` | Immutable, portable design revisions; validate schema |
| `world_commands` | `id uuid pk`, `world_id fk`, `branch_id fk`, `client_request_id`, `accepted_sequence bigint`, `scheduled_day bigint`, `actor_user_id`, `kind`, `payload jsonb`, `status`, `result jsonb` | Unique `(world_id, client_request_id)`; idempotency; audit |
| `world_events` | `(world_id, branch_id, event_sequence)`, `day`, `kind`, `payload jsonb`, `state_hash` | Append-only ordered event stream; retention policy |
| `world_checkpoints` | `id uuid pk`, `world_id`, `branch_id`, `day`, `mechanics_version`, `format_version`, `state_hash`, `blob bytea`, `created_at` | Initial implementation bytea; large snapshots may move to object storage later |
| `world_statistics` | `(world_id, branch_id, day, metric_key)`, `value`, `dimensions jsonb` | Derived time series, can be pruned/recomputed |
| `introductions` | `id uuid pk`, `destination_world_id`, `source_world_id nullable`, `source_design_id`, `source_revision`, `source_individual_ref`, `actor_user_id`, `command_id`, `created_at` | Phase 2 transfer workflow; Phase 1 introductions can omit source world |

**Schema details to enforce:** foreign keys, explicit `ON DELETE` rules (avoid cascading away lineage/audit inadvertently), uniqueness, constrained statuses, optimistic revision for world metadata, per-world monotonic command/event sequences, indexes on `(world_id, branch_id, day)`, and timestamps in UTC. `jsonb` is appropriate for versioned payloads, not as a substitute for relational ownership/permission constraints. Binary checkpoint format must include schema and mechanics version, endian-independent encoding and checksum.

### 5.1 PostgreSQL transaction and recovery [P]

- **Command acceptance transaction:** verify permission and world status; insert command with idempotency key and allocated sequence; commit; return accepted sequence. Do not claim applied until the world actor has executed it.
- **Tick persistence transaction:** atomically persist checkpoint and/or replay boundary, resulting events, command statuses, authoritative day/hash. Avoid holding SQL transactions while executing expensive simulation steps.
- **Restart:** load most recent valid checkpoint; validate hash/format; replay commands/events as defined by durability policy; compare resulting hashes; quarantine world as `faulted` on mismatch rather than silently proceeding.
- **Branching:** immutable checkpoint references and copy-on-write logical histories. Pruning must not delete checkpoints referenced by descendants.
- **Backups:** PostgreSQL backup/PITR appropriate to deployment, plus restore drills. Retention policies are configurable (DD-036); default values remain to be benchmarked/approved.

## 6. JSON API contract v1 [P]

Base path `/api/v1`. HTTPS required for remote access; WebSocket uses WSS. JSON UTF-8. Distinguish transport version (`v1`), simulation mechanics version, genome schema version and checkpoint format version.

### 6.1 HTTP endpoints (proposed)

| Method | Path | Purpose |
|---|---|---|
| `POST` | `/api/v1/auth/login` | Authenticate and create session |
| `POST` | `/api/v1/auth/logout` | Revoke session |
| `GET` | `/api/v1/me` | Account/session information |
| `GET` | `/api/v1/planets` | Available planet definitions and revisions |
| `GET` | `/api/v1/worlds` | List accessible worlds |
| `POST` | `/api/v1/worlds` | Create private world from planet template and seed |
| `GET` | `/api/v1/worlds/{world_id}` | Metadata, status, day, mechanics version |
| `POST` | `/api/v1/worlds/{world_id}/commands` | Submit validated, idempotent command |
| `GET` | `/api/v1/worlds/{world_id}/observations` | Read-only map/region/population data, bounded pagination |
| `GET` | `/api/v1/worlds/{world_id}/events?after=...` | Catch up from sequence |
| `GET` | `/api/v1/worlds/{world_id}/statistics` | Chart/time-series query |
| `POST` | `/api/v1/worlds/{world_id}/branches` | Branch from retained checkpoint |
| `GET` | `/api/v1/designs` | List owned organism designs |
| `POST` | `/api/v1/designs` | Create immutable first revision |
| `POST` | `/api/v1/designs/{id}/revisions` | Save new immutable genome revision |
| `GET` | `/api/v1/designs/{id}/revisions/{revision}` | Retrieve portable genome revision |
| `GET` | `/api/v1/health/live` | Process liveness; no sensitive internals |
| `GET` | `/api/v1/health/ready` | DB and world-supervisor readiness |

Use standard HTTP status semantics: `200` query, `201` creation, `202` accepted command, `400` malformed, `401` unauthenticated, `403` forbidden, `404` absent/inaccessible (policy-dependent), `409` conflict or stale revision, `422` valid JSON but invalid domain command, `429` quota exceeded, `503` unavailable. Never leak another user's world existence through errors.

### 6.2 Command envelope [P]

```json
{
  "protocol_version": 1,
  "request_id": "a9d2b8a0-ec36-4ed5-8f5e-9f56c9791d51",
  "world_id": "91e44a79-6b0c-43a6-a52d-73036a4d8f5a",
  "expected_world_revision": 42,
  "command": "introduce_organisms",
  "payload": {
    "design_id": "6d744bcb-f913-4978-933d-f37ec9d8a72c",
    "design_revision": 3,
    "tile": { "x": 12, "y": 19 },
    "count": 10
  }
}
```

Illustrative response:

```json
{
  "protocol_version": 1,
  "request_id": "a9d2b8a0-ec36-4ed5-8f5e-9f56c9791d51",
  "command_id": "d936c23c-0a68-4977-b3cf-8c98e5b9fb6b",
  "status": "accepted",
  "accepted_sequence": 138,
  "scheduled_day": 201
}
```

`request_id` is unique per world for idempotent retries. `expected_world_revision` is for optimistic control conflicts; it should not be tied to every observation update. A command can later become `applied` or `rejected` with reason; the client must not assume `202` means the biological change has happened. Distinguish validation at acceptance and execution-time preconditions.

### 6.3 WebSocket events [P]

Connect to `/api/v1/worlds/{world_id}/stream?after=<sequence>` using authenticated session; do not place bearer secrets in query strings. Send ordered, bounded events:

```json
{
  "protocol_version": 1,
  "message_type": "world.event",
  "world_id": "91e44a79-6b0c-43a6-a52d-73036a4d8f5a",
  "branch_id": "00000000-0000-0000-0000-000000000001",
  "event_sequence": 4501,
  "day": 201,
  "event": "population_changed",
  "payload": { "lineage_id": "lineage-17", "population": 82 }
}
```

Subscription handshake returns latest retained event sequence and observation revision. Reconnect uses `after`; if history was pruned, server responds with `resync_required` and a fresh observation endpoint. WebSocket is **not** a guarantee of exactly-once delivery; event IDs support deduplication. For large maps send viewport/region deltas and aggregates, not every organism every tick. Throttle/coalesce nonessential presentation updates without dropping authoritative durable events required for replay.

### 6.4 Error envelope [P]

```json
{
  "error": {
    "code": "WORLD_REVISION_CONFLICT",
    "message": "The world control revision has changed.",
    "request_id": "a9d2b8a0-ec36-4ed5-8f5e-9f56c9791d51",
    "details": { "current_revision": 43 }
  }
}
```

Error codes are stable, human text is not. Validate JSON schema, numeric bounds, string lengths, coordinate wrapping policy, design revision compatibility, permissions and quotas. Document payload size and rate limits.

## 7. Web client design [P]

**Recommended:** TypeScript + Vite + a modest component framework (React or Svelte; **selection remains open**). Use HTML/CSS for menus, forms, editors and charts; Canvas 2D for tile map and organism overlays initially. Consider WebGL only after profiling map throughput. Do not build a second simulation in JavaScript.

Suggested screens: login/account; world catalogue and create world; world overview (map, clock, pause/speed, climate); organism/genome editor (basic traits and advanced alleles); population/species/lineage inspector; resource and ecological overlays; charts/timeline/events; scenario objectives; checkpoints/branches and comparisons; settings. Responsive layouts should be considered now, but Phase 1 need not deliver a polished Android app.

Client state model: server owns truth; client holds transient view state (viewport, selection, open panels, cached observation). Reconcile after reconnect using event cursor and snapshot revision. Never infer command success from optimistic animation alone. Web UI and later Godot/Android clients use the same public API.

## 8. Authentication, authorisation and security [P]

- Phase 1 has accounts and private worlds; registration policy (invite-only vs self-service) remains open. Support a bootstrap administrator through deployment tooling, not a hard-coded password.
- Argon2id password hashing, server-side opaque session tokens hashed at rest, Secure/HttpOnly/SameSite cookies, CSRF defence for cookie-authenticated mutating requests, TLS via reverse proxy, WSS same-origin enforcement and origin checks.
- Separate roles: global administrator; world owner (Phase 1). Add editor/contributor/observer in Phase 2 without modifying biological entities.
- Authorise every world/design read and command, not just page navigation. Rate-limit login and costly simulation actions; validate all request sizes.
- Never expose database credentials, session secrets or raw checkpoint blobs through ordinary observation endpoints. Log request/command IDs but redact secrets and genome data where appropriate.
- Validate server commands again against simulation constraints. Phase 2 requires explicit rules for species release, ownership, transfer eligibility, moderation and griefing; **not approved yet**.

## 9. Deployment and operations [P]

**Target:** one Linux host, one Rust service, existing PostgreSQL instance, HTTPS reverse proxy (Nginx or Caddy), static frontend served by proxy or Axum. Deployment via `systemd`; containers are optional, not required. Do not assume root access to the PostgreSQL service or co-location with any existing application.

```text
Internet / LAN
    |
TLS reverse proxy :443
    |-- /                 -> static web bundle
    |-- /api/v1/*         -> Rust Axum 127.0.0.1:8080
    |-- /api/v1/.../stream -> WebSocket upgrade
                              |
                         PostgreSQL over restricted connection
```

- Run as dedicated unprivileged service account, separate DB role/schema/database, migrations applied by a controlled deployment identity.
- Config via environment or root-readable configuration: `DATABASE_URL`, bind address, public base URL, session key, limits, log level and checkpoint settings. No committed credentials.
- Provide readiness/liveness, structured logs, metrics (world day, ticks/sec, tick duration, active worlds, memory, queue depth, checkpoint latency, DB failures, WS subscribers), graceful shutdown and recovery tests.
- On shutdown stop accepting commands, finish or roll back current tick atomically, persist recovery boundary and close connections. Define timeouts for large worlds.
- Existing PostgreSQL availability is assumed, but capacity, backup configuration, TLS and access policy must be verified on the actual host.

## 10. Testing and validation plan

| Test layer | Minimum gate |
|---|---|
| Unit | Pure genotype/phenotype functions, toroidal coordinates, resource accounting, command validation, schema conversions |
| Determinism | Golden world seed + command stream → exact state hashes across supported architectures, release/debug builds |
| Property | No negative resources/populations; bounded coordinates; conservation rules; reproducible PRNG streams |
| Integration | PostgreSQL migrations on clean DB, constraints, command idempotency, checkpoint round-trip, crash/restart replay |
| API contract | JSON Schema/OpenAPI validation, errors, permission denial, reconnect/dedup/resync |
| World isolation | Advance World A while B is paused; no state or RNG interference; different world speeds |
| Security | Unauthorised world access denied; CSRF/origin protection; rate limits; no token disclosure |
| Browser E2E | Create world → introduce design → run/pause → inspect → save/reload → branch |
| Performance | Benchmarks for 3 world-size presets; tick throughput, resident memory, persistence latency, map streaming |
| Recovery | Forced termination at tick boundaries and DB failure injection; restore to last declared durable state |
| Phase 2 contract | Synthetic second client can subscribe without engine changes; provenance payload can represent copy transfer |

**Acceptance criterion:** every requirement and decision in SL-FS-001 maps to tests or a named deferred milestone; algorithms still labelled [U]/[P] are not falsely marked as verified original behaviour.

## 11. Delivery milestones and dependency gates

1. **Foundation:** Cargo workspace, CI, formatting/linting, PostgreSQL migrations, health endpoints, account bootstrap, OpenAPI skeleton. Gate: Linux deploy and authenticated API.
2. **Deterministic core:** versioned world state, fixed PRNG/canonical hashing, toroidal grid, scheduler scaffolding, headless CLI. Gate: cross-platform golden fixtures and repeatable save/load.
3. **First ecology vertical slice:** initial terrestrial plants, herbivores, resource depletion/regeneration, autonomous behaviour; numerical rules from separately approved mechanics specification. Gate: reproducible multi-year headless runs with invariants.
4. **Persistence and world management:** multiple private worlds, command journal, checkpoints, branch/restart, speed controls. Gate: crash/recovery and isolation tests.
5. **Browser gameplay:** world map, species/genome editor, introduction, inspection, overlays, charts and history. Gate: browser end-to-end playable experiment.
6. **Feature completeness:** remaining Phase 1 documented SimLife-style mechanics and optional scenario tools. Gate: SL-FS-001 acceptance traceability and benchmarked size presets.
7. **Phase 2 (deferred):** shared-world roles, multiple concurrent players, planet selection, copy-based organism transfers, audit and abuse controls.
8. **Phase 3 (deferred):** private evolutionary labs, expanded planet chemistry/biochemistry, Android packaging and optional Godot client as prioritised.

**Avoid:** beginning with the full genetic system, all biomes, exotic biology or a polished renderer before the headless simulation can demonstrate meaningful, stable ecological dynamics.

## 12. Open architecture decisions and recommended defaults

| ID | Open decision | Proposed default | Decision owner / timing |
|---|---|---|---|
| AR-01 | Browser framework | TypeScript + Vite + Svelte (or React if preferred) | Before UI implementation |
| AR-02 | Initial server exposure | Private/authenticated HTTPS Linux host; invite-only accounts | Before deployment |
| AR-03 | Authentication | Server-side opaque sessions with HttpOnly cookies | Before account work |
| AR-04 | Checkpoint codec | Versioned binary, canonical encoding; zstd compression if beneficial | Before persistence |
| AR-05 | Recovery durability | Durable command journal + periodic checkpoints + deterministic replay | Before world execution |
| AR-06 | World concurrency | One simulation owner per world; concurrency across worlds | Before scheduler |
| AR-07 | World memory/speed quotas | Benchmark-derived, per-account limits | Before public hosting |
| AR-08 | Preset dimensions | Benchmark three presets; no fixed dimensions yet | After core performance tests |
| AR-09 | Exact genetic aggregate representation | Explicit joint genotype/compatibility fidelity; do not rely solely on marginal frequencies | Before aggregation mechanics |
| AR-10 | Mechanics scheduling/formulas | Decide in SL-MS-001, not this document | Before ecology implementation |
| AR-11 | Planet schema | Versioned terrestrial template first; capability-based extension points | Before world generation |
| AR-12 | Multiplayer transfer restrictions | Copy genome by explicit command; quotas/permissions to be designed | Phase 2 |
| AR-13 | Android delivery | Responsive web/PWA first; native packaging only if justified | Phase 2/3 |
| AR-14 | Godot client | Optional, no Phase 1 dependency | Later |
| AR-15 | Event retention and checkpoint cadence | Configurable; initial values benchmarked | Before production |

## 13. Traceability and document precedence

- **SL-FS-001:** approved gameplay, world model and architectural decisions DD-001–DD-050. This document must not silently override those decisions.
- **SL-MS-001:** proposed detailed simulation rules, numerical models and tick scheduling. Unapproved algorithms remain proposals.
- **SL-SA-001 (this document):** proposed implementation architecture, interfaces, storage, protocol and delivery plan. Framework choices are recommendations pending acceptance.
- **`SimLife.txt`:** source for original game behaviour only; it does not prescribe Rust, SQL schemas, modern web protocols or multiplayer mechanics.

If a proposal here conflicts with an approved DD decision, the approved decision prevails. If implementation uncovers a genuine conflict (for example, exact aggregate allele distributions versus memory budgets), record an architecture decision record (ADR) and seek an explicit change rather than silently weakening the requirement.

## 14. First engineering deliverables

Create `docs/adr/0001-rust-linux-postgres.md` reflecting DD-043–DD-050; initialise the workspace; publish `schemas/openapi-v1.yaml`; implement migration `0001_initial.sql`; build a `sim-core` executable test fixture with a fixed seed, integer day counter, toroidal coordinates, command sequencing and canonical hash; then implement a minimal browser page that creates/starts/pauses/inspects a world via the API. Biological mechanics should be introduced only against the separate mechanics specification and its acceptance tests.