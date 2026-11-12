# SimLife

SimLife is a server-authoritative ecological simulation. Milestone M1 establishes a deterministic clock, independent worlds, checkpoint persistence, an HTTP/WebSocket API, and a small browser control panel. Biology and ecology mechanics are intentionally not part of M1.

## Project direction

SimLifeClone is a server-hosted, single-player scientific sandbox. The Rust engine is authoritative and independent of networking, persistence, and presentation. PostgreSQL stores durable world metadata and checkpoints; active simulation state stays in memory. The public interface uses versioned JSON over HTTP and WebSocket. The architecture preserves independent planetary worlds and later copy-based genome transfer without implementing multiplayer gameplay here.

The Phase 1 specifications are in [`docs/`](docs/). Their approved decisions govern the project. The detailed mechanics algorithms remain proposals; M1 deliberately implements no organism, genetics, reproduction, or ecological rules. M1 defaults are UUID world identifiers, a non-zero ChaCha8 seed, one integer tick per simulated day, an initial one-second day interval, JSON checkpoint payloads, and command event records. Checkpoints are written at world creation and after accepted control commands, never per internal tick. These defaults are implementation choices, not claims about original SimLife mechanics.

## Requirements

- Rust stable toolchain and Cargo
- PostgreSQL 14 or later
- Node.js 20 and npm

The Rust dependency graph is pinned by the committed `Cargo.lock`; the browser dependency graph is pinned by `web/package-lock.json`.

## Local setup

Create a PostgreSQL role and database for local development, then copy `.env.example` to `.env` and set `DATABASE_URL`. The server reads environment variables directly; export them in your shell (or use your preferred dotenv loader). Never commit `.env`.

```sh
createdb simlife
cp .env.example .env
export DATABASE_URL='postgres://YOUR_USER:YOUR_PASSWORD@localhost:5432/simlife'
export SIMLIFE_BIND='127.0.0.1:3000'
```

Migrations are applied automatically by the server at startup. To apply them explicitly:

```sh
cargo install sqlx-cli --no-default-features --features rustls,postgres
sqlx migrate run --source crates/simlife-persistence/migrations
```

## Build, test, run

```sh
cargo fmt --all -- --check
cargo build --workspace
cargo test --workspace
cargo run -p simlife-cli
```

The server listens on `SIMLIFE_BIND` (default `127.0.0.1:3000`), requires `DATABASE_URL`, applies migrations, and restores the latest checkpoint of each world at startup.

In another shell:

```sh
cd web
npm install
npm run dev
```

Open the Vite URL (normally `http://127.0.0.1:5173`). Set `VITE_API_URL` if the API is not at `http://127.0.0.1:3000`.

## API

All endpoints are versioned under `/api/v1`:

- `GET /health`
- `GET /worlds`
- `POST /worlds` with `{ "name": "Earth", "seed": 42, "tick_duration_seconds": 1 }`
- `GET /worlds/{id}`
- `POST /worlds/{id}/commands` with `{ "command": "start" }`, `pause`, `resume`, or `{ "command": "advance", "ticks": 1 }`
- `GET /worlds/{id}/stream` (WebSocket snapshot/update stream)

Start transitions a new world to running; pause and resume require their matching current states. Advance is accepted only while running and is limited to 100,000 ticks per request. Each tick represents one simulated day. In M1, progression is explicit through the advance command; an autonomous world supervisor and playback-speed scheduler are later work. The server owns and mutates all simulation state.

## M1 limits

M1 does not yet include accounts, authentication, an autonomous wall-clock runner, graphical ecology, or detailed biological mechanics. The browser client is intended for local development. World snapshots currently include only the deterministic clock and seeded random stream marker. PostgreSQL migration/recovery and end-to-end WebSocket integration tests require `DATABASE_URL`; without it, those tests report a skip. No claim of live database verification is made unless they are run against a PostgreSQL service.
