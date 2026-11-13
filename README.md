# SimLife

SimLife is a server-authoritative deterministic ecology sandbox. This repository includes the M1 project foundation and the M2–M6 world engine, organism, ecology, playable client, and hardening work. The Rust simulation engine is independent of PostgreSQL, HTTP, and browser code. Each world advances independently; the server keeps active state in memory and PostgreSQL stores definitions, commands, and periodic recovery checkpoints.

## Architecture

- `simlife-domain`: shared world, organism, configuration, snapshot, and command types.
- `simlife-engine`: deterministic integer-only world generation and simulation. No database, server, or client dependencies.
- `simlife-persistence`: SQLx PostgreSQL store and migrations.
- `simlife-server`: Axum HTTP/WebSocket API and per-world asynchronous clock tasks.
- `simlife-cli`: server executable.
- `simlife-bench`: reproducible seeded engine benchmark for throughput, memory, and population change.
- `web`: TypeScript/React control panel and terrain viewer.

## Implemented simulation rules and defaults

The mechanics specification labels detailed formulas as proposals. These M2–M6 defaults are implementation choices intended to be deterministic and easy to inspect, not claims about original SimLife behavior:

- One tick is one simulated day. World sizes are small (32×64), medium (64×128), and large (128×256); maps wrap at both edges.
- Terrain is generated from a seeded SplitMix64 integer stream, smoothed elevations, and fixed thresholds for ocean, lake, river, plains, forest, desert, and mountain. Tile water and vegetation regenerate with fixed integer rules; minerals are a static terrain resource.
- Organisms have diploid integer alleles for speed, metabolism, and fertility, plus diet, sex, and lifespan. Movement chooses a seeded random neighboring direction per movement point. Metabolism and water use are paid daily; herbivores consume vegetation; starvation, dehydration, and lifespan cause death.
- Founder organisms with matching genomes apart from sex share a species lineage, so separately introduced compatible females and males can reproduce; offspring retain their parental lineage.
- Carnivores attack co-located non-carnivores. Organisms meeting the configured fertility threshold reproduce asexually, or sexually when a same-species opposite-sex mate shares their tile. Each inherited allele is selected from a parent and may mutate by one integer step with a fixed 1% chance. Mutations and births use the same saved random stream.
- The single-threaded engine orders organisms by stable ID and uses integer arithmetic. The canonical state hash includes terrain, resources, organisms, ancestry, lineage records, counters, and random state, but excludes the world UUID and display name; identical seeds therefore generate equal states under equal commands.
- The server starts an independent clock task for each running world. State stays in memory between checkpoints. Checkpoints are written at creation, after accepted commands, and every 100 simulated days, not on every tick. Startup restores the newest checkpoint for each world; if an older pre-lineage checkpoint fails the new hash format, it replays command history and stores a current-format checkpoint.

The M5 client lists/creates worlds, selects map size and tick interval, displays terrain and organisms, reports status/day/population/births/deaths, receives WebSocket changes, starts/pauses/resumes worlds, advances a day, and introduces a configured founder organism on a selected tile. It plots rolling total and diet-specific population history. Map color and red organism markers are presentation only.

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

## Benchmarking and population observation

Build the release benchmark and run the same seeded, 100-day workload at several population sizes. Run each size as its own process so peak resident memory is attributable to that case:

```sh
cargo build -p simlife-bench --release
./target/release/simlife-bench 1000 100 20261113 large
./target/release/simlife-bench 10000 100 20261113 large
./target/release/simlife-bench 50000 100 20261113 large
./target/release/simlife-bench 100000 100 20261113 large
```

Enable reproduction by adding `reproductive` after the map size. This uses asexual founders with fertility alleles set to 100 and the deterministic reproduction and mutation rules used by the engine:

```sh
./target/release/simlife-bench 1000 500 20261113 medium reproductive
./target/release/simlife-bench 10000 500 20261113 large reproductive
```

The result includes the selected mode and birth/death totals. Evaluate repeated seeds and population trajectories before drawing conclusions about sustainable carrying capacity. One local reproductive/sterile comparison is recorded in [`docs/benchmarks/2026-11-13-reproductive-observability.md`](docs/benchmarks/2026-11-13-reproductive-observability.md).

The benchmark prints JSON containing setup time, ticks per second, population change, births/deaths, peak process RSS, and the final state hash. Throughput includes each tick plus construction of the compact live update and its deterministic state hash. It excludes database checkpoints, HTTP, and browser work. Linux `VmHWM` is used for peak memory; other platforms report `null`. These measurements are a single local baseline, not a capacity guarantee. See [`docs/benchmarks/2026-11-13-baseline.md`](docs/benchmarks/2026-11-13-baseline.md).

The evolutionary stability run advances mechanics versions 1 and 2 to days 1,000, 5,000, and 10,000, checks checkpoint restoration at each point, and reports population, lineage extinctions, allele heterozygosity, speciation rates, throughput, memory, and hashes. Fertility is configurable to examine different growth pressures:

```sh
cargo run --release -p simlife-bench -- stability 20261113 1 small 20 both
```

Arguments are seed, founder count, map size, expressed fertility (1–100), and mechanics selection (`1`, `2`, or `both`). For a recorded run and its limitations, see [`docs/benchmarks/2026-11-13-evolutionary-stability.md`](docs/benchmarks/2026-11-13-evolutionary-stability.md).

### Gameplay validation experiments

Run five deterministic game-facing experiments for a seed, defaulting to 1,000 days. They compare a low-metabolism design to random trait recipes in a resource-poor generated habitat, compare fast and slow herbivore populations, run a predator–prey introduction, fork an established ecosystem for control/disruption/restoration attempts, and compare identical and changed experiment branches:

```sh
cargo run --release -p simlife-bench -- gameplay 20261113 1000
```

The command prints JSON metrics and a machine-evidence scorecard. It does not assign subjective gameplay scores: human agency, challenge, trade-offs and enjoyment require unassisted playtesting. Results and the proposed acceptance protocol are in [`docs/gameplay-validation.md`](docs/gameplay-validation.md). The currently serialized `species_*` fields refer to genealogical lineage branches. Pairwise genetic compatibility gates mating; a lineage label does not define a reproductively isolated biological species.

The simulation records population, diet counts, average expressed speed/metabolism/fertility, and cumulative births/deaths every 10 days. The rolling history holds up to 20,000 samples (200,000 simulated days); checkpoints preserve it. The browser plots total population and population by diet. World listing returns compact summaries; selecting a world fetches its full map and history.

## Evolutionary observability

Every organism has a stable world-local ID and permanent lineage record with parent IDs, birth day, birth genome, lineage ID (currently serialized as `species_id`), and death day when applicable. Introduced organisms are founders without parents. Individual inspection reports both parents and offspring, and ancestry follows the first parent branch, nearest ancestor first; the API caps requested depth at 100. Click an organism on the map to inspect its current variables, genome, parents, offspring and ancestry. Inspect with `GET /api/v1/worlds/{world_id}/organisms/{organism_id}?ancestors=10`.

World snapshots store per-lineage allele copy counts for speed (0–4), metabolism (0–20), and fertility (0–100), sampled every 10 days. Each vector index is an allele value and each count is the number of copies, so the sum is twice the sampled population. Every lineage present in a sample is recorded; the history retains at most 100,000 lineage-sample rows. The browser limits its chart selector to the first 32 lineages for presentation, but this does not cap lineages or stored lineage history. `species_lineages` retain prototype genomes, founder identity and extinction day, while `speciation_events` record parent lineage, new lineage, founder, day and measured genetic distance. These are branching ancestry records, not assigned biological species groups; biological species membership is not currently modeled.

### Compatibility and automatic speciation (mechanics version 2)

New worlds use mechanics version 2. Compatibility is a deterministic integer score on a 0–10,000 basis-point scale. It averages five normalized dimensions: diploid speed, metabolism and fertility alleles (allele pairs sorted to ignore chromosome phase), lifespan, and diet. Sex is not a species trait. A score at or below 500 bp (5%) is compatible; a higher score prevents mating. Asexual reproduction remains available. Species labels do not prevent mating when genotypes are compatible, allowing compatible neighboring lineages to interbreed. After inheritance and mutation, a child more than 500 bp from its reproducing parent's species prototype establishes a new lineage and a recorded speciation event. The child genotype becomes that lineage's prototype. This threshold and metric are explicit provisional defaults because the mechanics proposal did not approve a formula; they are versioned and can be calibrated later.

World checkpoints store `mechanics_version`. Worlds created before version 2 continue using mechanics version 1, which preserves the previous exact-genome founder grouping and same-lineage mating rules and does not automatically speciate. Checkpoint recovery and history replay use the world's version so a software upgrade does not silently change its future outcomes.

## API

All routes are versioned under `/api/v1`:

- `GET /health`
- `GET /worlds`
- `POST /worlds` with `{ "name": "Earth", "seed": 42, "size": "small", "tick_duration_ms": 1000 }`
- `GET /worlds/{id}`
- `GET /worlds/{id}/organisms/{organism_id}?ancestors=10`
- `GET /worlds/{id}/replay` (rebuild from seed and command history through the live tick; returns a hash match flag)
- `POST /worlds/{id}/commands` with `{"command":"start"}`, `pause`, `resume`, `{"command":"advance","ticks":1}`, `{"command":"set_speed","tick_duration_ms":500}`, or `{"command":"introduce_organism","x":2,"y":4,"genome":{...}}`
- `GET /worlds/{id}/stream` (initial full snapshot followed by compact world updates)

World configuration requires a non-empty name, non-zero 64-bit seed, and tick interval between 10 and 60,000 ms. `advance` requires a running world and accepts 1 to 100,000 days. The API server is authoritative; the client cannot change a world except through validated commands.

## Current limits

This implementation provides an early playable individual-organism ecology, not every Phase 1 proposal. The hybrid aggregate population model, richer environmental/weather systems, sophisticated multi-step predation, genome editing, multi-player accounts, and production authentication are not implemented. Population is capped at 100,000 organisms per world. Permanent ancestry records increase memory use with every birth and currently have no pruning policy. Replay starts from the original seed and walks accepted command history, so replay cost grows with world age; checkpoints provide fast restart recovery. The reproducibility guarantee targets supported Rust platforms with the current versioned integer rules; changing simulation algorithms requires a mechanics-version/state-hash migration. PostgreSQL migration/recovery and network WebSocket integration tests require a reachable database via `DATABASE_URL`; without one, those tests exit early and must be reported as skipped, not as database verification.
