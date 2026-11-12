# SimLifeClone

**SimLifeClone** is an independently implemented ecosystem and evolutionary simulation inspired by the gameplay concepts described in *SimLife: The Genetic Playground* (Maxis, 1992). The aim is to create a playable single-player scientific sandbox first, extend it to persistent multiplayer planetary ecosystems, and then expand the simulation and gameplay.

> **Project status:** Specification and architecture design. No working game or server implementation is included yet. The biological algorithms in the mechanics specification are proposals, not approved implementations.

## Vision

Players design organisms, introduce them into simulated ecosystems and observe their survival, reproduction, mutation, speciation and ecological effects. Organisms are autonomous; the player's role is that of a scientist or evolutionary designer, not a direct creature controller.

The longer-term design introduces **independent persistent planetary worlds**. Each planet has its own climate, resources, populations, simulation clock and evolutionary history. Players can choose a planet into which to release a design. Organisms do not move between planets automatically: a player can deliberately copy a selected organism's genome from one world to another, creating a new evolutionary branch without removing the source organism.

## Development phases

| Phase | Goal | Scope |
| --- | --- | --- |
| **1 — Single-player** | Build and validate the simulation and playable browser client | Private worlds, world/organism editors, ecological and genetic simulation, observation, experiments, statistics, checkpoints and scenarios where applicable |
| **2 — Multiplayer** | Introduce persistent shared planetary ecologies | Shared worlds, permissions, concurrent player introductions and deliberate interplanetary genome transfers |
| **3 — Expansion** | Extend the simulation and game | Diverse planetary chemistry and biology, private evolutionary laboratories, additional ecological and gameplay systems |

Phase 1 is deliberately **single-player gameplay on a server-based architecture**. Multiplayer systems and exotic biology are not Phase 1 delivery requirements, but the architecture must not rule them out.

## Approved technical direction

| Area | Decision |
| --- | --- |
| Simulation and server | **Rust**, with a headless simulation core independent of UI, database and networking |
| Hosting | **Linux** |
| Persistence | **PostgreSQL**; active simulation state held in memory and checkpointed durably |
| Client/server protocol | Versioned **JSON over HTTP and WebSocket** |
| First client | **Web browser** |
| Possible subsequent clients | Android app / PWA and optional Godot client, sharing the same API |
| Simulation | Independent world clocks; deterministic, single-threaded reference simulation for Phase 1 |

**Proposed, not yet approved implementation details** include Axum, Tokio, SQLx, Serde, TypeScript/Vite, Canvas 2D and a modular Cargo workspace. See the architecture specification for the full design, alternatives and boundaries.

## Documentation

| Document | Purpose |
| --- | --- |
| [Functional specification](docs/SimLife_Phase1_Functional_Specification.md) | Requirements traced to the original manual, plus approved design decisions (DD-001–DD-050) and Phase 1 boundaries |
| [Simulation mechanics specification](docs/SimLife_Phase1_Mechanics_Specification.md) | Approved invariants, proposed tick/scheduler model and work packages for genetics, ecology, environment and experiments; numerical rules remain to be agreed |
| [System architecture and implementation specification](docs/SimLife_System_Architecture_and_Implementation_Specification.md) | Rust/Linux/PostgreSQL architecture, proposed Cargo structure, API, persistence, client design, security, testing and milestones |

The documents distinguish **documented original behaviour**, **approved game-design decisions**, **implementation proposals** and **unresolved mechanics**. They are living specifications, not evidence that all decisions have been implemented.

## Key design principles

- One authoritative simulation engine, with no client-specific simulation rules.
- Simulation state changes only through validated commands; clients may disconnect without automatically stopping a running world.
- Deterministic evolution and experiment replay, including checkpoints and branching histories.
- Population-level and individual-level simulation, with preservation of genetic distributions and resources where aggregation is used.
- Independent planetary worlds with no automatic inter-world biological migration.
- World-independent, versioned genome definitions to allow future deliberate exports and introductions.
- Terrestrial biology initially; no unnecessary Earth-specific assumptions in long-term engine interfaces.

## Getting started

This repository currently contains specifications only. There is no installable application, database schema or build command yet. The planned initial milestone is a Rust Cargo workspace, a headless deterministic simulation skeleton, PostgreSQL migrations, a versioned JSON API and a minimal browser interface. Implementation should follow the approved requirements and record any deviations as explicit design decisions.

## Source material and intellectual property

The functional specification is informed by the original *SimLife: The Genetic Playground* user manual (Maxis, 1992). **The original manual is not included in this repository** because it is copyrighted source material and this project does not establish redistribution permission. This project is an independent reimplementation of simulation ideas, not a copy of Maxis source code, graphics, audio, interface assets or branding. The name *SimLife* is used here only to identify the historical inspiration; the project name is **SimLifeClone**.

## Licence

No project software or documentation licence has yet been selected. Do not assume an open-source licence merely because the repository is public.