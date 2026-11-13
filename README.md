# SimLife

SimLife is a server-authoritative deterministic ecology sandbox. This repository includes the M1 project foundation and the M2–M6 world engine, organism, ecology, playable client, and hardening work. The Rust simulation engine is independent of PostgreSQL, HTTP, and browser code. Each world advances independently; the server keeps active state in memory and PostgreSQL stores definitions, commands, and periodic recovery checkpoints.

## Architecture

- `simlife-domain`: shared world, organism, configuration, snapshot, and command types.
- `simlife-engine`: deterministic integer-only world generation and simulation. No database, server, or client dependencies.
- `simlife-persistence`: SQLx PostgreSQL store and migrations.
- `simlife-server`: Axum HTTP/WebSocket API and per-world asynchronous clock tasks.
- `simlife-cli`: server executable.
- `web`: TypeScript/React control panel and terrain viewer.

## Implemented simulation rules and defaults

The mechanics specification labels detailed formulas as proposals. These M2–M6 defaults are implementation choices intended to be deterministic and easy to inspect, not claims about original SimLife behavior:

- One tick is one simulated day. World sizes are small (32×64), medium (64×128), and large (128×256); maps wrap at both edges.
- Terrain is generated from a seeded SplitMix64 integer stream, smoothed elevations, and fixed thresholds for ocean, lake, river, plains, forest, desert, and mountain. Tile water and vegetation regenerate with fixed integer rules; minerals are a static terrain resource.
- Organisms have diploid integer alleles for speed, metabolism, and fertility, plus diet, sex, and lifespan. Movement chooses a seeded random neighboring direction per movement point. Metabolism and water use are paid daily; herbivores consume vegetation; starvation, dehydration, and lifespan cause death.
- Founder organisms with matching genomes apart from sex share a species lineage, so separately introduced compatible females and males can reproduce; offspring retain their parental lineage.
- Carnivores attack co-located non-carnivores. Organisms meeting the configured fertility threshold reproduce asexually, or sexually when a same-species opposite-sex mate shares their tile. Each inherited allele is selected from a parent and may mutate by one integer step with a fixed 1% chance. Mutations and births use the same saved random stream.
- The single-threaded engine orders organisms by stable ID and uses integer arithmetic. The canonical state hash includes terrain, resources, organisms, counters, and random state, but excludes the world UUID and display name; identical seeds therefore generate the same initial world and produce equal states under equal commands.
- The server starts an independent clock task for each running world. State stays in memory between checkpoints. Checkpoints are written at creation, after accepted commands, and every 100 simulated days, not on every tick. Startup restores the newest checkpoint for each world.

The M5 client lists/creates worlds, selects map size and tick interval, displays terrain and organisms, reports status/day/population/births/deaths, receives WebSocket changes, starts/pauses/resumes worlds, advances a day, and introduces a starter organism on a selected tile. Map color and red organism markers are presentation only.

## Requirements

- Rust stable toolchain and Cargo
- PostgreSQL 14 or later
- Node.js 20 and npm

Cargo and npm lockfiles pin dependency versions.

## Local development

Create a PostgreSQL database and role for SimLife. Copy the example environment file and configure the connection string and bind address. The server reads `DATABASE_URL` and `SIMLIFE_BIND` from the process environment; it does not automatically read `.env`.

```sh
createdb simlife
cp .env.example .env
export DATABASE_URL='postgres://YOUR_USER:YOUR_PASSWORD@localhost:5432/simlife'
export SIMLIFE_BIND='127.0.0.1:3000'
```

Do not commit `.env` or credentials. To install the migration CLI and apply migrations explicitly:

```sh
cargo install sqlx-cli --no-default-features --features rustls,postgres
sqlx migrate run --source crates/simlife-persistence/migrations
```

The server also applies migrations automatically at startup.

## Build, test, and run

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace
cargo test --workspace
cargo run -p simlife-cli
```

Run the browser client in another shell:

```sh
cd web
npm ci
npm run build
npm run dev
```

Open the Vite URL (normally `http://127.0.0.1:5173`). Set `VITE_API_URL` in the client environment if the API uses a different origin. Browser CORS is permissive for local development; put the server behind an appropriate deployment boundary before exposing it publicly.

## API

All routes are versioned under `/api/v1`:

- `GET /health`
- `GET /worlds`
- `POST /worlds` with `{ "name": "Earth", "seed": 42, "size": "small", "tick_duration_ms": 1000 }`
- `GET /worlds/{id}`
- `GET /worlds/{id}/replay` (rebuild from seed and command history through the live tick; returns a hash match flag)
- `POST /worlds/{id}/commands` with `{"command":"start"}`, `pause`, `resume`, `{"command":"advance","ticks":1}`, `{"command":"set_speed","tick_duration_ms":500}`, or `{"command":"introduce_organism","x":2,"y":4,"genome":{...}}`
- `GET /worlds/{id}/stream` (initial full snapshot followed by compact world updates)

World configuration requires a non-empty name, non-zero 64-bit seed, and tick interval between 10 and 60,000 ms. `advance` requires a running world and accepts 1 to 100,000 days. The API server is authoritative; the client cannot change a world except through validated commands.

## Current limits

This implementation provides a first playable individual-organism ecology, not every Phase 1 proposal. The hybrid aggregate population model, richer environmental/weather systems, species lineage/speciation, sophisticated multi-step predation, genome editing, multi-player accounts, and production authentication are not implemented. Population is capped at 100,000 organisms per world. Replay currently starts from the original seed and walks accepted command history, so replay cost grows with world age; it is intended as a verification/recovery foundation while checkpoints provide fast restart recovery. The reproducibility guarantee targets supported Rust platforms with the current versioned integer rules; changing simulation algorithms requires a state-hash/version migration. PostgreSQL migration/recovery and network WebSocket integration tests require a reachable disposable database via `DATABASE_URL`; without one, those tests exit early and must be reported as skipped, not as database verification.
