# SimLife-inspired Phase 1 — Detailed simulation mechanics specification

**Document:** SL-MS-001  
**Version:** 0.1 — working draft; algorithms not yet approved  
**Parent:** SL-FS-001 v0.9 (DD-001–DD-050; DD-043–DD-050 concern architecture/worlds, not approved mechanics)  
**Source policy:** Original manual claims must be traced to `SimLife.txt` and the parent functional specification. The mechanics below are **proposals [P]**, not claims about original SimLife implementation. Approved design constraints are **[A]**. Unknown original algorithms remain **[U]**.

**Architecture boundary:** The authoritative simulation is a headless Rust library, hosted by a Linux Rust server with PostgreSQL persistence and a versioned JSON HTTP/WebSocket API. Web is the first client; Android and Godot remain future/optional. Planetary world transfer and exotic biology are not Phase 1 mechanics. See parent §18.

## 1. Approved invariants [A]

- One integer simulation tick equals one simulated day (DD-038).
- Ecology, reproduction and environment use separately configured day-based intervals (DD-032, DD-038).
- The world is toroidal on both axes (DD-019); world-size presets are benchmark-derived (DD-037).
- Organisms are autonomous; behavioural drives are genetically influenced (DD-002, DD-013).
- Multiple organisms can occupy one tile; interactions occur at tile level (DD-014).
- Local perception and limited memory guide foraging and mating (DD-015).
- Predation involves detection, pursuit, escape and combat (DD-016).
- Plants and animals use hybrid individual/aggregate representations; observation does not change simulation fidelity (DD-018, DD-033, DD-034).
- Aggregate population counts, resources and defined allele distributions are exact; individual histories/behaviour may be approximate (DD-041).
- Cross-platform deterministic outcomes are mandatory, using a single-threaded reference engine (DD-035, DD-042).
- Checkpoints, reproducible branches, interventions and configurable history retention are required (DD-024, DD-030, DD-036).

## 2. Authoritative state model — proposal [P]

`WorldState`: engine/schema version, seed, integer day, toroidal width/height, tile states, environment state, species/lineage registry, individual organisms, aggregate cohorts, scenario state, pending interventions, deterministic random-stream state, branch/checkpoint metadata.

`TileState`: fixed elevation/terrain, mutable climate/biome classification, surface-water classification and availability, resources, individual-entity identifiers and aggregate cohorts.

`Individual`: stable identifier, location, age/life stage, energy, hydration, health, genotype, expressed phenotype, behavioural drive state, limited resource memory, encounter state, ancestry and lineage references.

`AggregateCohort`: location/region, life-stage/sex/reproductive classes as needed, exact count, exact conserved resource totals and exact allele/genotype distribution representation, plus statistical estimates for behaviour and individual histories.

**Open:** exact numeric units, gene catalogue, cohort partitioning and the minimal sufficient joint-genotype representation. Do not implement aggregation from allele marginals alone before proving compatibility/inheritance fidelity.

## 3. Daily tick contract — candidate, not approved [P]

Every invocation of `advance_one_day(state, ordered_commands)` must be deterministic and atomic. Proposed phases:

1. Validate and apply scheduled scientist interventions at the day boundary, in stable command-ID order.
2. Apply environmental processes when the environmental interval is due: weather, seasons, water/resource regeneration, disasters and biome classification.
3. Apply ecological processes when the ecological interval is due: perception, drive selection, movement, feeding, hydration, encounters, energy/health costs and mortality.
4. Apply reproductive processes when the reproductive interval is due: maturity/compatibility checks, mating or asexual reproduction, allele inheritance, mutation, births and lineage/speciation updates.
5. Resolve extinction, scenario objectives, analytics and authoritative events.
6. Increment the integer day, emit a canonical state hash and checkpoint if configured.

**Unapproved choices:** whether environmental updates precede ecology, whether ageing occurs daily or at ecological intervals, whether death is processed before mating, and whether newborns act on their birth day. The order above is a review candidate, not a settled rule.

## 4. Determinism contract — candidate [P]

Use a specified integer PRNG with published test vectors and isolated streams (world generation, weather, ecology, reproduction, mutation, scenario generation). Use stable entity and tile traversal, deterministic tie-breaking and canonical serialisation. Prefer fixed-point or integer arithmetic for authoritative state; avoid unspecified floating-point operations. The UI/inspection path must not advance any simulation PRNG stream. Define a canonical state hash over authoritative state, excluding UI caches and non-authoritative analytics. Cross-platform golden fixtures must be checked in CI.

## 5. Mechanics work packages and decisions required

| Package | Required specification | Outstanding designer/engineering choices |
|---|---|---|
| M-01 Clock and scheduler | Year length; ecological/reproductive/environmental intervals; phase ordering; ageing and birth-day rules | Choose calendar and daily update order |
| M-02 Genetics | Gene/locus schema, allele values, expression rules, gamete formation, asexual copying, mutation | Choose ploidy, linkage, mutation mechanism and rates |
| M-03 Compatibility/speciation | Mating eligibility, isolation metric, lineage splitting/merging, extinction identities | Choose compatibility metric and threshold policy |
| M-04 Energy and life cycle | Feeding conversion, metabolism, hydration, growth, maturity, senescence, mortality | Choose resource units and energy balance |
| M-05 Behaviour | Drives, sensory range, memory decay, goal selection, movement, deterministic tie-breaking | Choose arbitration model and memory limits |
| M-06 Predation | Detection, pursuit, escape, combat, injury, consumption | Choose encounter duration and resolution model |
| M-07 Plants and ecology | Plant growth/reproduction, resource uptake, competition, regeneration, biomass conservation | Choose plant lifecycle and resource stocks |
| M-08 Environment | Seasons, weather, biome transitions, tile water, disaster triggers/effects | Choose climate units and transition rules |
| M-09 Aggregation | Cohort partitioning, exact genetic distributions, interactions, promotion/demotion, conservation | Choose cohort fidelity and transition triggers |
| M-10 Experiment framework | Interventions, scenarios, score rules, checkpoints, branching, retention | Choose replay guarantees and storage defaults |
| M-11 Performance | Benchmark workloads, three world-size presets, population ceilings | Choose target hardware and tick-throughput budgets |

## 6. First mechanics review — M-01 clock and scheduler

These are the first decisions to obtain before numerical algorithms are written:

**M01-Q1. Year length:** (a) 360 days / 12×30-day months; (b) 365 days with conventional months; (c) configurable days per year. **Proposed default:** 360 days, for straightforward seasonal scheduling.

**M01-Q2. Ecological interval:** (a) every day; (b) every 2–5 days; (c) configurable integer interval. **Proposed default:** every day.

**M01-Q3. Reproduction scheduling:** (a) daily eligibility checks with gestation timers; (b) global reproduction interval; (c) season-gated reproduction with daily eligibility checks. **Proposed default:** daily eligibility checks, with species-specific maturity/gestation rules.

**M01-Q4. Environmental updates:** (a) daily weather and resources, seasonal climate changes; (b) weekly weather and resources; (c) one configurable environment interval for everything. **Proposed default:** daily weather/resources, seasonal baseline shifts.

**M01-Q5. Coincident-event ordering:** (a) environment → ecology → reproduction; (b) ecology → reproduction → environment; (c) explicitly configurable order. **Proposed default:** environment → ecology → reproduction, subject to birth/death rules.

**M01-Q6. Birth and death timing:** (a) newborns begin acting next day; death resolves before reproduction; (b) newborns act immediately; death after reproduction; (c) species-configurable. **Proposed default:** newborns act next day; organisms dying during ecology cannot reproduce later that day.

## 7. Engineering acceptance gates (draft)

- **DET-01:** identical seed, configuration, commands and version produce byte-identical canonical hashes across supported platforms.
- **DET-02:** UI inspection, playback speed, pause/resume and headless execution never change hashes at the same simulation day.
- **TIME-01:** exactly one day advances per authoritative tick; scheduled subsystems execute on their specified days and in the approved order.
- **GEN-01:** known parent allele fixtures produce reproducible offspring genotypes and phenotypes for each approved expression rule.
- **POP-01:** every aggregate transition preserves the exact quantities required by DD-041.
- **HIST-01:** checkpoint replay reproduces hashes; independent branches diverge only after distinct commands.

## 8. Approval state

**Approved:** DD-001–DD-042 in the parent functional specification.  
**Pending:** M-01 through M-11 algorithm definitions and the six M01 questions above.  
**Do not treat proposed defaults as designer-approved decisions.**