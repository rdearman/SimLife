# SimLife-inspired single-player reconstruction — Functional specification

**Document:** SL-FS-001  
**Version:** 0.9 — Batches 1–7 plus architecture and planetary-world decisions  
**Source:** *SimLife: The Genetic Playground*, Maxis user manual (1992), supplied as `SimLife.txt` (204-page manual plus machine-specific addendum). Page references below are the printed `[PAGE: n]` markers in that text.  
**Purpose:** Implement an independently written single-player ecological simulation reflecting the documented behaviour of the original game, using a multiplayer-ready server architecture. Future multiplayer and planetary extensions are explicitly distinguished from Phase 1 deliverables.

## 1. Scope, fidelity and evidence rules

**Phase 1:** single-player experimental laboratory, world generation/editing, plant and animal genome design, hybrid individual/population simulation, environmental dynamics, mutation/speciation, scenarios, and inspection/statistics. Save/load and reproducible experiments are required.

**Out of scope for Phase 1 gameplay:** shared multiplayer ecology, interplanetary organism transfer, player-owned evolutionary laboratories, exotic biology, and new mechanics beyond approved Phase 1 decisions. **In scope for Phase 1 architecture:** Rust Linux server, PostgreSQL persistence, versioned JSON HTTP/WebSocket interface, browser-first client, account access and private worlds. The engine must remain independent of UI, networking and database frameworks. See §18.

**Evidence tags:** **[D]** = explicitly documented in the manual; **[I]** = necessary implementation interpretation not specified by the manual; **[U]** = unknown requiring empirical testing against the original executable or an agreed approximation. A requirement tagged [D] does **not** imply that its internal numerical formula is documented.

**Important fidelity limitation:** This is a specification of *observable gameplay and model parameters*, not a claim to recover the original code or algorithms. Where the manual gives qualitative rules, numerical implementation remains [U]. The manual occasionally contains inconsistencies and typographical errors; these are recorded rather than silently reconciled.

## 2. Gameplay modes and simulation lifecycle

| ID | Requirement | Evidence |
|---|---|---|
| SL-001 | Offer **Experimental Mode** for unconstrained world/ecosystem experiments and a **scenario mode** with objectives. | [D] pp. 2–3, 103 |
| SL-002 | Support five difficulty levels, including a **Modified** state when physical laws are changed. Exact presets require extraction/verification. | [D] pp. 3, 89–91; [U] presets |
| SL-003 | Support start, pause, and at least slow/medium/fast simulation speed controls; simulation time advances independently of UI refresh. | [D] pp. 85–86; [I] decoupling |
| SL-004 | Maintain **Tick**, **Day**, **Year**; permit resetting the clock to zero. | [D] p. 91 |
| SL-005 | Allow run-until pause after a user-specified number of days or years. | [D] pp. 91, 189 |
| SL-006 | Provide New, Save, Load, and species/world data management consistent with the original workflows. | [D] pp. 84, 128–131 |
| SL-007 | Provide a user-settable **non-zero random seed** to reproduce world generation when all generation settings are identical. | [D] p. 90 |
| SL-008 | Record a time-stamped history of mutations, extinctions, species danger and species creation. | [D] p. 174 |

**Scenario discrepancy:** The introduction states **six** scenarios (p. 2); the New Game description says “nine scenarios” but subsequently says “six included scenarios” (p. 103). Treat the scenario inventory as **[U]** until the actual scenario descriptions and addendum have been reconciled. Do not invent scenario objectives.

## 3. World and environment

| ID | Requirement | Evidence |
|---|---|---|
| SL-010 | Represent the world as a two-dimensional **tile grid**. Support sizes **32×64**, **64×128**, **128×256**, **256×512** (Tiny–Large). | [D] p. 127 |
| SL-011 | Generate terrain/altitude, temperature zones, moisture zones, rivers/lakes and optional artefacts using configurable parameters. | [D] pp. 117–127, 181 |
| SL-012 | World-generation controls: regional weather variation; average temperature; average moisture; river/lake amount; mountain amount; toxin amount; mutagen amount; Ultra-Food source amount; barrier amount; world name. | [D] pp. 125–127 |
| SL-013 | Support rebuilding **only** temperature zones, moisture zones, rivers/lakes or artefacts, or rebuilding everything; selectable lake size. | [D] p. 181 |
| SL-014 | Support seasonal variation in **daylight length**, **rainfall/humidity**, and **temperature**, each with an amplitude and peak season. | [D] p. 151 |
| SL-015 | Support editing local terrain altitude, temperature, moisture, water, barriers, toxins and other world elements through a close-up map. | [D] pp. 104–116 |
| SL-016 | Include **water**, **Ultra-Food**, **toxins**, **mutagens**, and **barriers** as environmental objects/fields with observable ecological effects. | [D] pp. 111–116, 126 |
| SL-017 | Allow the soil and climate to change over time, controlled by a global soil/climate change setting. | [D] p. 180 |
| SL-018 | Offer a global overview map and a close-up editable map with overlays for environmental and biological data. | [D] pp. 104–122 |

**Not specified [U]:** procedural generation algorithm, tile coordinate convention, temperature/moisture units, hydrology, seasonal waveforms, erosion equations, resource diffusion/regeneration, barrier topology and numerical damage from hazards.

## 4. Biological entities and identity

| ID | Requirement | Evidence |
|---|---|---|
| SL-020 | Model two organism categories: **plants** and **animals**; both have species and living individuals. | [D] pp. 128–150 |
| SL-021 | Each species has a **Prototype Genome**, the genetic baseline from which descendants can diverge. | [D] p. 82 |
| SL-022 | Each individual has its own genotype and mutable state, including at least age, health, size/weight, food and water reserves where applicable. | [D] pp. 140, 182–183 |
| SL-023 | Species names may contain up to **15 characters**, with a species version number. | [D] pp. 80–81 |
| SL-024 | Individuals have species-derived names and monotonically allocated numeric identifiers; do not recycle IDs during normal operation. | [D] p. 81 |
| SL-025 | Permit selecting a global **Selected Species** and window-local **Local Species** without conflating the two. | [D] p. 81 |
| SL-026 | Permit editing a species prototype or an individual genome, with distinct scope and immediate observable effects where documented. | [D] pp. 129, 138–140 |
| SL-027 | Support creation, deletion, placement and relocation of individuals and bulk population insertion/removal. | [D] pp. 111–113, 123–124 |
| SL-028 | Use phenotype/icon representations for display, not as the canonical biological state. | [D] pp. 82, 133–137, 184 |

**Data model [I]:** `World`, `Tile`, `Species`, `Genome`, `Organism`, `EnvironmentalField`, `Event`, `SimulationParameters`, `StatisticsSeries`, `Scenario`. Keep **genotype**, **phenotype** and **current state** separate. Do not assume genome sliders are literal floating-point values with known ranges: these are [U].

## 5. Animal genome — documented controls

The following are **all explicit animal genome groups** described on pp. 141–147. The actual gene encoding, slider precision and numerical costs remain [U].

| Group | Controls / options | Documented function |
|---|---|---|
| Gender | Female, Male, Asexual, Sterile | Reproductive category |
| Movement | Walk, Climb, Swim, Fly (independent toggles) | Movement capabilities; each costs energy; none = stationary |
| Food sources | Nectar, Plants, Animals, Fruit, Filter Food, Seeds (independent toggles) | Diet capabilities; multiple diets incur additional energy cost; Ultra-Food is separate |
| Behaviour | Share Food; Roaming; Turning; Persist | Group feeding, idle movement, directional variation, trail persistence |
| Turning mode | Zig-zag, Looping, Random | Movement pattern |
| Turn angle | Low, Medium-Low, Medium-High, High | Directional change magnitude |
| Attraction/avoidance | Up to eight species references, each Prefer/Avoid/Ignore | Response to other organisms; rightmost reference wins conflicts |
| Life | Life Span; Adult; Die-off = Immortal / Old Age / Medium Age | Maximum longevity, maturity, natural death behaviour |
| Features | Size; Stealth; Weapons; Vision | Physical capabilities with energy trade-offs |
| Gestation | Size; Time; %Female; Mutation; Children = 1/2/4/8 | Reproductive cost, timing, sex ratio, mutation, litter size |
| Food stores | Max; Action; Danger | Capacity and thresholds for foraging urgency |
| Water stores | Max; Action; Danger | Capacity and thresholds for seeking water |
| Health | Max; Action; Danger | Maximum health, rest/recovery thresholds |
| Design feedback | Maximum Size bar; Energy Requirement bar | Whether organism can accommodate traits and afford their energy cost |

**Behavioural requirements:**

- **SL-030 [D]** Animals seek food and water, reproduce, and respond to enemies; behaviour is influenced by the genome (pp. 143–147).
- **SL-031 [D]** At **Action** thresholds, food/water seeking or health recovery becomes desirable; at **Danger** thresholds it dominates behaviour (pp. 146–147).
- **SL-032 [D]** Increasing capacities, movement modes, diets, weapons, vision etc. has survival costs, especially energy and sometimes body weight/speed (pp. 140–147).
- **SL-033 [D]** Animal death can result from starvation, dehydration, predation, age and other hazards (pp. 144–147, 156–157).
- **SL-034 [D]** Animals may share food with conspecifics, according to Share Food (p. 143).
- **SL-035 [D]** The rightmost of eight attraction/avoidance targets takes precedence when stimuli conflict (p. 144).

**Manual anomaly:** The Health **Danger** description refers to a “food storage level” rather than health (p. 147). Treat intended threshold semantics as **[U]** pending executable testing.

## 6. Plant genome — documented controls

| Group | Controls / options | Documented function |
|---|---|---|
| Gender | Male, Female, Both, Asexual | Reproductive category |
| Structure | Tree, Shrub, Grass, Floating | Ecological form |
| Seed type | Sticky, Dropping, Drifting, Fruit | Seed dispersal mechanism |
| Seasons | Sprout = Spring/Summer/Autumn/Winter | Flower season follows sprout; seed season follows flower |
| Evaporation Rate | Slider | Water loss and growth rate trade-off |
| Sprout Moisture | Slider | Moisture needed for germination |
| Sprout Temperature | Slider | Temperature needed for germination |
| Health | Slider | Resilience to toxins and partial consumption |
| Food | Slider | Maximum stored food |
| Water | Slider | Maximum stored water |
| Mutation Rate | Slider | Mutation likelihood in new generations |
| Maximum Size | Slider | Size limit |
| Old Age | Slider | End of reproductive life |
| Nectar | Boolean | Whether plant produces nectar |
| Evergreen | Boolean | Evergreen versus deciduous strategy |

**SL-040 [D]:** Plants grow, consume resources, may be eaten without necessarily dying, reproduce through seeds, and respond to seasonal climate and genotype (pp. 148–150, 165–166). **SL-041 [D]:** Different seed types, structures and evergreen behaviour exploit different ecological niches (pp. 171–173). **[U]:** pollination, fertilisation, seed trajectories, germination probabilities, photosynthesis and decay formulas.

## 7. Ecology, energy and population dynamics

| ID | Requirement | Evidence |
|---|---|---|
| SL-050 | Implement feeding interactions among animals and plants and track predator/prey relationships. | [D] pp. 4–5, 143, 165–168 |
| SL-051 | Support six food types: **Animals, Plants, Filter Food, Fruit, Nectar, Seeds**; Ultra-Food is an additional special source not governed by the six food-value settings. | [D] pp. 143, 176–177 |
| SL-052 | Organisms consume energy for survival, movement, growth, gestation and healing. Costs are adjustable globally. | [D] pp. 140, 175–178 |
| SL-053 | Simulate food and water shortages, injury, predation, toxins, plague, old age, crowding and battle as applicable causes of decline/death. | [D] pp. 96–97, 156–157, 177 |
| SL-054 | Track reproduction, inheritance, mutation and changing genetic distributions within populations. | [D] pp. 5, 82, 145–150, 158–164 |
| SL-055 | Permit species extinction and emergence; record both in history/statistics. | [D] pp. 92–93, 174 |
| SL-056 | Represent ecological niche competition and diversity (food source × movement for animals; plant strategies for plants). | [D] pp. 171–173 |
| SL-057 | Permit plant and animal population limits, with limits applied globally as described. | [D] pp. 179–180 |

**Critical [U] rules:** tick update order; individual movement/pathfinding; perception radius; prey targeting; battle outcome; energy accounting; mating eligibility and genetic crossover; mutation distributions; death probabilities; food-web nutrient recycling; capacity limits on individual tiles. These are essential to implementing a running simulation but cannot be recovered exactly from the manual alone.

## 8. Evolution and speciation

- **SL-060 [D]** Each species has a prototype genome; descendants can diverge genetically (p. 82).
- **SL-061 [D]** **AutoSpeciate** creates a new species when an organism has diverged sufficiently from its prototype that it could no longer successfully mate with that prototype; new species retain the old name/icon and receive a new version number (pp. 81–82, 92).
- **SL-062 [D]** **Reconverge Species** first averages each species' living population genotype to update its prototype, then compares prototypes and merges species that could mate, retaining the earlier species identity (pp. 92–93).
- **SL-063 [D]** Manual speciation operations: replace the genomes of all members of a species with a selected individual's genotype; create a new species from an individual; or transform an individual into another existing species of the same plant/animal category (pp. 185–186).
- **SL-064 [D]** Global mutation rate scales individual species mutation rates; global life span scales species life span potential (pp. 178–179).
- **SL-065 [D]** The gene-pool view distinguishes **discrete** and **continuous** genes and displays population distributions (pp. 158–164).

**[U]:** genetic compatibility metric and threshold; how continuous/discrete genes inherit and mutate; how reconvergence averages discrete genes; when AutoSpeciate is evaluated; whether mating compatibility is symmetric. These should be tested, not assumed.

## 9. Laws of physics and configurable parameters

| Group | Documented values / controls | Source |
|---|---|---|
| Movement costs | Swim, Walk, Climb, Fly; each Tiny/Low/Medium/High/Huge | pp. 175–176 |
| Food energy values | Animals, Plants, Filter Food, Fruit, Nectar, Seeds; each Tiny–Huge | pp. 176–177 |
| Metabolic costs | Gestation, Growth, Healing; each Tiny–Huge | p. 177 |
| Health costs | Toxins, Plague, Old Age, Crowding, Battle; each Tiny–Huge | p. 177 |
| Plant costs | Food and Water; described as Tiny/Low/High/Huge | p. 178 |
| Global life span | Brief, Short, Medium, Long, Methuselah (manual spells “Methuselan”) | p. 178 |
| Ticks per day | **8, 16, 32, 64, 128, 256** | p. 178 |
| Days per year | **16, 32, 64, 128, 256** | p. 179 |
| Plant population cap | Unlimited, or 1/2 through 1/128 of total world tile count | p. 179 |
| Animal population cap | Unlimited, **50, 100, 200, 400, 800, 1600** total animals | pp. 179–180 |
| Mutation rate | Global adjustment scaling species rates | p. 179 |
| Soil/climate change | Global slider | p. 180 |

**SL-070 [D]:** A physics modification must affect the current simulation immediately and mark difficulty Modified (pp. 90–91, 180). **SL-071 [D]:** Support saving current physics settings as defaults separately from applying them to the active world (p. 180).

**Manual anomalies:** p. 177 describes Growth in terms of recovery from injury, plague and toxins, overlapping Healing. p. 180 refers to “maximum animals per species” although p. 179 says the limit applies to **all species combined**. The latter is the clearer statement; retain the discrepancy as an open verification item.

## 10. Disasters and interventions

**SL-080 [D]:** Expose the original disaster actions from the Disasters menu (pp. 96–97). **SL-081 [D]:** Allow deliberate interventions such as population introduction/removal, editing an individual's variables or genome, modifying climate/terrain, and placing toxins/mutagens/barriers (pp. 104–116, 123–124, 182–186). **SL-082 [D]:** Make intervention effects visible in organism health, mortality, population and history outputs where relevant.

**[U]:** exact disaster magnitudes, spread, duration, affected tiles and recovery. Preserve the names and documented effects from pp. 96–97 during detailed implementation; do not invent a disaster algorithm and label it authentic.

## 11. Observation, analysis and evaluation

| ID | View / function | Evidence |
|---|---|---|
| SL-090 | Dashboard: species selection, visibility and simulation controls. | [D] pp. 98–102 |
| SL-091 | Map/Edit: global map, local map, tile overlays, individual inspection and editing. | [D] pp. 104–122 |
| SL-092 | Biology Lab, Genome, Phenotype and Variables: inspect prototype/individual genes, phenotype and mutable state. | [D] pp. 128–150, 182–184 |
| SL-093 | Graphs: selectable time-series statistics, with **50-day** or **50-year** view. | [D] pp. 152–155 |
| SL-094 | Mortality: causes of death and predator attribution. | [D] pp. 156–157 |
| SL-095 | Gene Pool: gene frequencies and distributions for one species or all plants/animals. | [D] pp. 158–164 |
| SL-096 | Food Web: predator/prey links; Population Interaction: related predator/prey population history. | [D] pp. 165–169 |
| SL-097 | Population: relative species abundance; Diversity: occupied ecological niches. | [D] pp. 170–173 |
| SL-098 | History: major ecological and evolutionary events. | [D] p. 174 |
| SL-099 | Evaluation: ecology score, diversity, duration and difficulty contributing to numerical score/rating/status. | [D] p. 187 |
| SL-100 | Export simulation data as **ASCII tab-delimited** records. | [D] p. 92 |
| SL-101 | Optional reduced statistics collection to improve performance. | [D] p. 91 |

**[U]:** exact graph formulas, ecology score formula, 21 rating thresholds and detailed data-log field schema. The manual directs readers to a machine-specific addendum for the data-log format (p. 92).

## 12. Proposed implementation architecture [I — not manual-derived]

The architecture is deliberately compatible with later multiplayer conversion, **without implementing multiplayer in Phase 1**:

```text
Single-player Godot UI
  ├─ world editor / species designer / experiment controls
  ├─ map and organism renderer
  └─ statistics and analysis views
              │ local API
Simulation core (headless, authoritative)
  ├─ world + climate + terrain
  ├─ organism state and behaviour
  ├─ reproduction + genetics + speciation
  ├─ ecology + resources + hazards
  └─ deterministic clock + PRNG
              │
Persistence and experiment harness
  ├─ save/load snapshots
  ├─ scenario definitions
  ├─ event log and statistical export
  └─ unattended batch simulation/tests
```

**Architecture invariants:** simulation never depends on frame rate; all random draws come from controlled seeded PRNG streams; world state is serialisable; commands are explicit actions; core can run without UI; results can be inspected after unattended runs. These are implementation choices, not claims about original SimLife internals.

## 13. Validation and acceptance gates

| Gate | Acceptance test | Basis |
|---|---|---|
| G1 — World | All four grid sizes construct; same seed/settings reproduce same world; layers can be regenerated selectively. | SL-007, 010–013 |
| G2 — Genome | Every documented animal/plant gene is represented; changes survive save/load and affect model outputs. | SL-020–028, 030–041 |
| G3 — Ecology | A mixed plant/herbivore/predator population exhibits feeding, resource depletion, reproduction and mortality; outputs are inspectable. | SL-050–057 |
| G4 — Evolution | Heritable variation, mutation, AutoSpeciate, manual speciation and Reconverge operate consistently with chosen compatibility rules. | SL-060–065 |
| G5 — Physics | Every documented physics setting is editable; day/year durations and population caps enforce exact documented options. | SL-070–071 |
| G6 — Observation | Individual inspection, population/food-web/gene-pool/mortality/history views reflect simulation state. | SL-090–101 |
| G7 — Persistence | Save, reload and continue from the same tick; seeded unattended runs are repeatable with the same implementation version. | SL-006–008; architecture [I] |
| G8 — Original comparison | For each unresolved [U] mechanic, record a test against the original game where executable access exists, or mark an explicit approximation. | Fidelity rule |

**Important:** G3 and G4 prove *plausible behaviour*, not original-algorithm equivalence. Exact equivalence cannot be certified from the manual alone.

## 14. Unresolved questions and reverse-engineering backlog

**Priority A — simulation-critical:**

1. Tick phase ordering (climate, movement, metabolism, feeding, reproduction, mortality, mutation).
2. Genome encoding, ranges and mapping to movement speed, perception, combat and energy requirements.
3. Inheritance, mutation and genetic compatibility/speciation algorithms.
4. Plant energy, growth, water, seed spread, seasonal timing and nutrient cycles.
5. Animal decision priorities, target selection, pathing, encounter/battle and mating rules.
6. Tile occupancy, collision, carrying capacity, food depletion and regeneration.

**Priority B — fidelity and balancing:**

7. Default physics/difficulty presets and the precise Tiny→Huge multipliers (manual says adjacent values double, p. 175, but absolute base values are not supplied).
8. Exact world generation, seasonal climate, disasters and environmental hazards.
9. Scenario inventory and win/loss criteria (manual inconsistencies noted above).
10. Ecology score, diversity score, rating thresholds, data-log schema.
11. Platform-specific differences and addendum instructions relevant to gameplay.

**Resolution protocol:** Each [U] item receives an `OBSERVED`, `APPROXIMATED`, or `DEFERRED` status; a reproducible test case; the chosen rule/formula; and a link to the implementation/test. Do not promote approximations to documented SimLife mechanics.

## 15. Phase boundaries (revised architecture)

**Phase 1 — single-player game, server-hosted:** reconstruct the terrestrial SimLife-inspired simulation and approved Phase 1 mechanics. Deploy a Rust authoritative simulation/server on Linux, PostgreSQL persistence and a browser-based client. One player operates each private world; multiple independent private worlds per account are supported. Scenarios/objectives remain available as optional experiments, not the top-level organising concept. Multiplayer readiness is an architectural constraint, not a requirement to implement shared gameplay now.

**Phase 2 — shared multiplayer planetary worlds:** allow multiple players to select an existing persistent planet/world, design organisms and release populations into its existing ecology. Worlds are independent authoritative simulations. Add shared-world permissions, concurrent intervention handling and player-initiated genome-copy transfers between worlds. No automatic interplanetary migration. Web, Android and optional Godot clients use the same protocol.

**Phase 3 — expanded biology and gameplay:** add private evolutionary laboratories, controlled release workflows and radically different planetary environments/chemistries (e.g. ocean-only or hydrogen-rich worlds) when the underlying model supports them. No commitment to implement non-terrestrial biochemistry in Phase 1.

**Compatibility rule:** The Phase 1 biological model must not depend on a specific client, one connected user, a single world, local files, or a running renderer. Biological rules may evolve in later phases; preserve a stable engine boundary and version saved worlds/protocols rather than promising immutable Phase 1 mechanics.

## 16. Source index

Manual sections most directly relevant to coding: **pp. 80–83** terminology/species identity; **pp. 84–97** commands and disasters; **pp. 103–127** new game, editing, population and world generation; **pp. 128–150** Biology Lab and genomes; **p. 151** climate; **pp. 152–174** telemetry and ecological views; **pp. 175–181** laws of physics and world building; **pp. 182–189** individual variables, speciation, evaluation and run control.

This specification paraphrases the supplied manual and does not reproduce its original prose or artwork. The original game’s branding, source code, artwork and text are not required for the independent implementation.


## 17. Approved Phase 1 design decisions (12 November 2026)

These are **[DESIGN]** decisions made by the project designer, **not** claims about the original SimLife implementation. Where they alter the baseline's assumptions, they take precedence over earlier interpretative text. They do not override the manual-derived descriptions of available controls.

| ID | Approved decision | Requirements affected | Implementation implication |
|---|---|---|---|
| DD-001 | Player is a **scientist**: create organisms and environments, then observe. | SL-001, SL-015, SL-026–027 | World/species editing and observation are primary; no direct avatar or unit-command gameplay. |
| DD-002 | Organisms are **fully autonomous**: genetics and environment determine behaviour. | SL-030–035, SL-040, SL-050–056 | Behaviour is simulation-owned; player changes conditions/genomes, not moment-to-moment movement. Seeded, testable behaviour is required. |
| DD-003 | Use **hybrid simulation fidelity**: individuals where necessary, statistical populations elsewhere. | SL-020–028, SL-050–057 | Define an explicit aggregation/disaggregation policy; preserve species/genetic diversity, conservation of resources and meaningful events when changing fidelity. Earlier wording implying every individual must always be simulated is superseded. |
| DD-004 | Support **natural evolution and optional genetic intervention**. | SL-021, SL-026, SL-054–055, SL-060 onward | Natural reproduction/mutation must function without intervention. Genome editing is an optional scientist action, distinguishable in history/statistics from naturally evolved changes. |
| DD-005 | Provide **objectives and scoring**, plus optional open-ended sandbox mode. | SL-001–002, evaluation/scenario requirements | Scenarios require explicit success/failure/scoring definitions; sandbox has no compulsory victory condition. The manual's exact scenario list and scoring coefficients remain unresolved. |
| DD-006 | Support **variable speed, pause and accelerated simulation**. | SL-003–005 | Simulation clock is independent of rendering; support batch/headless accelerated runs and reproducible fixed-seed tests. Exact time-unit mapping and speed presets remain open. |

### Approved Batch 2 decisions — genetics and reproduction (12 November 2026)

These are **[DESIGN]** decisions, not undocumented claims about the original game's algorithms. They take precedence over earlier implementation assumptions while retaining the manual-derived feature inventory.

| ID | Approved decision | Requirements affected | Observable behaviour / acceptance criterion |
|---|---|---|---|
| DD-007 | Provide **both a simple trait editor and an advanced genetic view**. | SL-021–022, SL-026, SL-065; G2 | Both views edit/read the same canonical genome; edits made in either view appear consistently in the other. The exact gene-to-trait mapping remains to be specified. |
| DD-008 | Offspring inherit **discrete alleles from each parent**. | SL-022, SL-060–065; G4 | Sexual offspring receive inherited allelic contributions from both parents; seeded crosses reproduce the same results. Dominance, linkage, ploidy and allele-to-phenotype rules remain **OPEN**. For asexual reproduction, the genetic copying rule remains **OPEN**. |
| DD-009 | Support **sexual and asexual reproduction**, with maturity and mating compatibility. | SL-030, SL-040, SL-050–056, SL-061; G3–G4 | Reproduction requires eligibility checks; sexual reproduction requires compatible partners and mature organisms. Exact asexual triggers, plant fertilisation and compatibility rules remain **OPEN**. |
| DD-010 | Mutation occurs **at reproduction and through environmental mutagens**. | SL-022, SL-040, SL-060, SL-064; G4 | Reproductive mutations and environmental mutagen exposure are distinct, logged causes. Whether environmental exposure changes somatic phenotype, transmissible alleles, or both remains **OPEN**; mutation rates and effects are not yet defined. |
| DD-011 | Support **automatic speciation plus optional player-defined species**. | SL-023–024, SL-060–063; G4 | The simulation can split lineages under a defined genetic compatibility rule; the scientist can explicitly create a new species identity. Species identity and lineage history survive save/load. The precise threshold/algorithm remains **OPEN**. |
| DD-012 | Genome editing can target **a species prototype or a particular individual**, at the player's choice. | SL-021–022, SL-026, SL-063; G2 | The UI makes edit scope explicit and records the intervention. Editing a prototype does not silently rewrite living individuals; editing one individual does not silently rewrite its species prototype or peers. How prototype edits affect subsequently spawned organisms versus naturally conceived offspring remains **OPEN**. |

#### Batch 2 implementation consequences

- **Genome architecture:** distinguish genotype (alleles), phenotype (expressed traits), mutable physiological state and species prototype. A trait editor must not become an independent competing genome representation.
- **Reproduction:** sexual and asexual paths need separately testable eligibility, inheritance and mutation stages. Both must work without user commands.
- **Species history:** track lineage, speciation events and player-authored identities, including changes caused by later reconvergence.
- **Hybrid population model:** aggregated populations must preserve enough allele-frequency and compatibility information for reproduction, mutation and speciation; otherwise DD-003 and DD-008 conflict.
- **Determinism:** seed-controlled mutation, inheritance and mating decisions must be reproducible in headless tests and after save/load.

#### Additional acceptance tests

| Test | Expected outcome |
|---|---|
| AT-DD07 | Editing a genome through the basic view is reflected in the advanced view and vice versa, including after reload. |
| AT-DD08 | Given fixed parents, alleles and PRNG seed, offspring genotypes are repeatable and respect the approved inheritance rule once specified. |
| AT-DD09 | Immature/incompatible organisms cannot reproduce sexually; eligible compatible adults can; asexual reproduction works independently. |
| AT-DD10 | Mutation event records distinguish reproductive from environmental causes, and replay reproducibly. |
| AT-DD11 | Automatic and manual species creation generate distinct traceable lineage events and persistent identities. |
| AT-DD12 | Prototype-only edits leave existing individual genomes unchanged; individual-only edits leave prototype and other individuals unchanged. |

### Approved Batch 3 decisions — behaviour, movement and ecology (12 November 2026)

These are **[DESIGN]** decisions, not claims that the original SimLife used these internal algorithms. They supersede conflicting implementation interpretations elsewhere in this document.

| ID | Approved decision | Requirements affected | Observable behaviour / acceptance criterion |
|---|---|---|---|
| DD-013 | Organisms select actions using **evolvable behavioural drives**; genes influence the relative strength of competing motivations. | SL-022, SL-050–054; G3–G4 | Under identical environmental conditions, organisms with different inherited drive parameters may choose different actions. Decisions are autonomous and deterministic for fixed initial state and random seed. Exact drive list, scoring and arbitration are **OPEN**. |
| DD-014 | **Multiple organisms may occupy a tile**, with interactions resolved at tile level. | SL-020, SL-027, SL-050, SL-053, SL-057; G1–G3 | A tile can contain several organisms of one or more species; co-location makes eligible interactions possible but does not guarantee them. Occupancy limits and interaction ordering remain **OPEN**. |
| DD-015 | Organisms have **local perception and limited memory** of previously discovered food, water and mates. | SL-022, SL-050–053; G3 | Organisms cannot navigate using undiscovered global resource locations; remembered locations can influence movement. Perception range, memory capacity, decay and pathfinding are **OPEN**. |
| DD-016 | Predation and combat are **multi-step encounters**: detection, pursuit, escape and combat. | SL-050, SL-052–053; G3 | Eligible encounters progress through observable states and can end without combat. Speed, sensory and defensive traits influence outcomes; exact equations, action timing and cancellation conditions are **OPEN**. |
| DD-017 | Food and other resources are **explicit quantities** that are consumed, depleted and regenerated. | SL-030–040, SL-050–053, SL-057; G1–G3 | Consumption debits a resource stock, insufficient stock limits feeding, and regeneration follows a specified environmental rule. Resource types, storage, growth and conservation rules are **OPEN**. |
| DD-018 | Simulate **individual organisms in observed or interacting areas** and **statistical populations elsewhere**. | SL-020–028, SL-050–057; G1–G4; DD-003, DD-008 | Observation/interaction regions use individual simulation; eligible remote regions use aggregate models. Transitions must preserve organism counts, resource quantities and sufficient genetic variation, and must not create or delete organisms through rounding. Boundaries, interaction triggers, transition algorithms and equivalence tolerances are **OPEN**. |

#### Batch 3 implementation consequences

- **Behavioural genome:** define a gene-to-drive mapping, with inherited drive weights and physiological inputs (hunger, thirst, fear, reproductive readiness). Avoid hard-coded species-specific scripts that bypass inherited behaviour.
- **Spatial indexing:** maintain tile occupancy collections rather than a single occupant reference; define consistent ordering for encounters, movement and consumption within each simulation tick.
- **Local knowledge:** distinguish actual environmental state from each organism's perceived/remembered state. Memory must be serialised when an organism is individually simulated.
- **Encounter state machine:** detection → pursuit or avoidance → escape or engagement → combat/resolution; interruption and disengagement must be representable.
- **Resource ledger:** local stocks and regeneration rules must account for transfers/consumption consistently across individual and aggregate simulation.
- **Hybrid fidelity:** observed areas and areas containing active cross-species interactions need individual resolution. Aggregates must retain genetic distributions, life stages, resource demand and interaction potential. Switching fidelity is a model operation, not merely a rendering optimisation. Define how observation itself avoids altering biological outcomes.

#### Additional acceptance tests

| Test | Expected outcome |
|---|---|
| AT-DD13 | Two organisms with different inherited behavioural weights can make different choices from identical stimuli; fixed-seed replay matches. |
| AT-DD14 | Multiple organisms occupy one tile and can interact without overwriting each other's state; save/load preserves occupancy. |
| AT-DD15 | An organism cannot act on a distant, undiscovered resource; it can subsequently use a location it has perceived and remembered. |
| AT-DD16 | Predation can be resolved at detection, pursuit, escape or combat stages; outcomes and event histories are replayable. |
| AT-DD17 | Resource consumption decreases local stock; depleted resources cannot be consumed; regeneration respects defined limits. |
| AT-DD18 | Aggregation/disaggregation preserves exact counts and accounted resource stocks, retains prescribed genetic distributions within tolerance, and produces no artificial birth/death/speciation events. |

### Approved Batch 4 decisions — world generation and environment (12 November 2026)

These are **[DESIGN]** decisions made for the reconstruction; they do not assert that the original SimLife used identical topology, update rules or intervention handling. Where they differ from an interpretation of the manual, the approved design decision governs Phase 1.

| ID | Approved decision | Requirements affected | Observable behaviour / acceptance criterion |
|---|---|---|---|
| DD-019 | The world is **toroidal**: both horizontal and vertical axes wrap. | SL-010–011, SL-015, SL-018, SL-050–053; G1–G3 | Neighbour lookup, movement, perception, pursuit, pathfinding, water adjacency and environmental propagation use modulo coordinates on both axes. No world-edge collision or invisible boundary. World-generation seams must be handled coherently. |
| DD-020 | **Terrain elevation and landform tiles remain fixed** after generation, but changing climate and vegetation can change biome conditions. | SL-011, SL-013–015, SL-017, SL-030–040; G1–G2 | Environmental variables and biome classification can change over time without altering base terrain. Explicit scientist terrain edits (DD-024) are an exception to natural terrain immutability. Whether generated river/lake geometry can change naturally is **OPEN**. |
| DD-021 | Model **seasons and weather**, without a separate day/night simulation. | SL-003–004, SL-014, SL-017, SL-030–040; G1–G3 | Seasonal and weather conditions affect ecological rules; no daily light/dark phase affects organism actions. Retain a simulation day as a clock unit. Manual references to seasonal daylight variation can be represented as a seasonal parameter, but not a day/night cycle. |
| DD-022 | Use **tile-based water availability**, including rivers, lakes and oceans. | SL-011–016, SL-030–040, SL-050–053; G1–G3 | Water bodies and tile water availability are explicit, queryable map state; organisms access water according to local rules. Full fluid dynamics is not required. Water replenishment, movement constraints, salinity and ocean generation are **OPEN**. |
| DD-023 | Disasters may occur **naturally** or be **triggered by the scientist**. | SL-012, SL-015–017, SL-008; G1–G3 | Both trigger paths use the same disaster-effect implementation; each event records type, location, time, cause and seed-relevant parameters. Disaster catalogue, probabilities, magnitudes and recovery rules are **OPEN**. |
| DD-024 | The scientist may **edit the world while paused or running**; live edits are recorded as interventions. | SL-003, SL-006, SL-008, SL-015, SL-018; G1–G3 | A running edit is applied atomically at a defined simulation-tick boundary; intervention history records the actor, target, before/after values and simulation time. Replaying the same seed and ordered interventions reproduces the result. Undo/rollback policy is **OPEN**. |

#### Batch 4 implementation consequences

- **Toroidal topology:** centralise wrapping and toroidal distance calculations; apply them to spatial indexes, map tools, perception and pathfinding. Avoid hard-coded edge checks or ordinary Euclidean distances across seams.
- **Stable physical terrain, changing ecology:** separate immutable baseline landforms from mutable climate, vegetation, soil/resource conditions and derived biome classification. Scientist edits are explicit exceptions to baseline immutability.
- **Environmental time:** maintain tick/day/year and seasonal phase; weather evolves independently of the UI. No separate day/night state machine is required.
- **Water model:** represent river, lake and ocean tiles plus local availability; specify drinking, traversal, replenishment and habitat effects later. Toroidal water-body generation must not assume a conventional map boundary.
- **Disasters:** route natural and player-generated disasters through a shared effects pipeline and event log; distinguish causes in statistics and replay.
- **Live interventions:** enqueue edits and commit at tick boundaries with deterministic ordering; include them in save/load and replay. Pausing does not change the semantics of an edit.

#### Additional acceptance tests

| Test | Expected outcome |
|---|---|
| AT-DD19 | Movement, perception, pathfinding and ecological neighbourhoods connect across both map seams; corner adjacency works correctly; save/load retains topology. |
| AT-DD20 | Climate and vegetation changes can change biome conditions without changing baseline terrain; explicit terrain edits are logged and persist. |
| AT-DD21 | Seasonal/weather variation affects environment and organism outcomes; no separate day/night behaviour is present. |
| AT-DD22 | River, lake and ocean tiles and local water availability are inspectable; organisms respond to accessible/depleted water as defined by subsequent rules. |
| AT-DD23 | Natural and scientist-triggered events of the same type use consistent effects; logs distinguish origin and replay deterministically. |
| AT-DD24 | Live edits take effect at tick boundaries without partial-state reads; identical initial state, seed and intervention log reproduce the same results. |

### Approved Batch 5 decisions — objectives, extinction and scientific tools (12 November 2026)

These are **[DESIGN]** decisions, not assertions about the original SimLife implementation. They supersede conflicting interpretations of earlier requirements.

| ID | Approved decision | Requirements affected | Observable behaviour / acceptance criterion |
|---|---|---|---|
| DD-025 | Provide **both authored scenarios and procedurally generated challenges**. | SL-001–002, SL-006; G6 | A scenario has a reproducible initial state, constraints and objectives; authored and generated scenarios use the same scenario execution framework. Generation parameters and catalogue remain **OPEN**. |
| DD-026 | Evaluate success using **scenario-specific objectives**, while exposing ecological metrics for analysis. | SL-001–002, SL-008; G6 | Scenario completion and scoring are determined by explicit scenario rules; biodiversity, stability and other metrics are inspectable but do not implicitly constitute victory. Metric definitions and scoring weights remain **OPEN**. |
| DD-027 | **Extinction is permanent** unless the scientist explicitly reintroduces the species. | SL-008, SL-060–063; G4 | When the final living member dies, the species is marked extinct and cannot spontaneously respawn. Species identity, genome reference and lineage history remain available for inspection and deliberate reintroduction. Automatic speciation may produce a similar new lineage, but must not silently resurrect the extinct species identity. |
| DD-028 | Scientist intervention is **unlimited in sandbox mode**; scenarios may impose **configurable budgets or restrictions**. | SL-001–002, SL-015, SL-021, SL-026, SL-063; DD-001, DD-004, DD-024; G6 | The same intervention operations work in both modes. Sandbox imposes no gameplay cost; scenario rules can reject or debit interventions and must communicate remaining allowance. Budget units and restricted actions remain **OPEN**. |
| DD-029 | Provide **maps, population charts, individual/genome/behaviour/lineage inspection, experiment replay and comparison tools**. | SL-008, SL-020–028, SL-065; G5, G7 | Users can inspect both current state and historical changes; compare two runs/branches against common metrics and selected snapshots. Inspection must not itself alter simulation outcomes; replay fidelity and retention limits remain **OPEN**. |
| DD-030 | Maintain **deterministic experiment history, checkpoints and branching alternative timelines**. | SL-003–008; DD-006, DD-024; G7 | Given identical initial state, simulation version, random streams and ordered interventions, replay yields the same outcomes. A checkpoint can fork into independent branches without changing its parent; branch ancestry, seed, simulation version and intervention log are retained. Checkpoint interval, storage policy and cross-version compatibility remain **OPEN**. |

#### Batch 5 implementation consequences

- **Unified scenario model:** authored and generated challenges produce the same serialisable scenario definition, with objectives, scoring, permitted interventions and starting conditions.
- **Metrics separate from victory:** record ecological time-series independently from scenario success logic; scoring is declarative and testable.
- **Extinction ledger:** retain extinct species identities, lineage and genomes without keeping phantom living populations. Explicit reintroduction is a logged scientist action, subject to scenario constraints.
- **Intervention policy:** route all scientist actions through a common validation, budgeting and deterministic tick-boundary commit path. Sandbox bypasses gameplay restrictions, not data validation.
- **Observation and replay:** store enough state/event information to inspect individuals and compare runs, while ensuring observation never modifies random streams or simulation rules.
- **Branching persistence:** checkpoint immutable simulation state plus random-generator state and ordered interventions. Child branches have distinct identifiers and independent subsequent histories. Deterministic replay is guaranteed only for a fixed simulation implementation/version and declared configuration.

#### Additional acceptance tests

| Test | Expected outcome |
|---|---|
| AT-DD25 | An authored and a generated scenario both load, execute, save and evaluate through the same objective system; generation is repeatable from seed and settings. |
| AT-DD26 | Completing an objective changes scenario success/score according to its rules; unrelated ecological metrics remain available without accidentally triggering victory. |
| AT-DD27 | The last death marks a species extinct; unattended simulation never recreates that identity; deliberate reintroduction restores a living population and is logged. |
| AT-DD28 | Sandbox interventions succeed without gameplay cost; a scenario can reject an over-budget or forbidden intervention without changing world state. |
| AT-DD29 | Individual, genome, lineage and population views agree with saved simulation state; comparing/replaying does not perturb a running experiment. |
| AT-DD30 | Two runs from the same checkpoint with identical inputs match; differing interventions create independently reproducible child branches without modifying the parent. |

### Approved Batch 6 decisions — simulation implementation boundaries (12 November 2026)

These are **[DESIGN]** decisions approved by the designer, not claims about undocumented original SimLife algorithms. They supersede conflicting implementation interpretations.

| ID | Approved decision | Requirements affected | Observable behaviour / acceptance criterion |
|---|---|---|---|
| DD-031 | Offer **several selectable world-size presets**, rather than one fixed size or arbitrary dimensions. | SL-006–007; DD-019, DD-030 | The world-creation interface lists defined size presets; each produces a correctly wrapping toroidal map, and saves retain the selected preset and actual dimensions. Preset dimensions and performance budgets remain **OPEN**. |
| DD-032 | Schedule **ecological, reproductive and environmental updates at separate intervals**. | SL-003–005; DD-006, DD-021, DD-030 | Each subsystem executes at its declared simulation-time interval, in a deterministic order where updates coincide; speed changes affect wall-clock throughput, not simulated outcomes. Intervals and ordering remain **OPEN**. |
| DD-033 | Use **hybrid plant simulation**: individual plants where relevant and aggregate vegetation elsewhere. | DD-003, DD-017–018, DD-029–030 | Plants can be individually represented for interactions requiring identity; aggregate vegetation preserves biomass, resource demand and sufficient genetic distributions for inheritance/speciation. Conversion rules and fidelity thresholds remain **OPEN**. |
| DD-034 | **Observation of an aggregated region is read-only**: retain aggregate simulation and display statistical information. | DD-018, DD-029–030, DD-033 | Opening or closing a region view does not trigger individualisation, consume simulation RNG values or change subsequent outcomes. Individualisation may still occur for actual ecological interactions under separately defined rules. |
| DD-035 | Require **identical deterministic outcomes across all supported platforms and machines**, given the same engine/protocol version, inputs and seed. | SL-007–008; DD-006, DD-024, DD-030, DD-032–034 | Cross-platform test runs produce identical canonical state hashes at defined checkpoints. Specify canonical numeric arithmetic, PRNG, update ordering, serialisation and parallel execution constraints before implementation. Cross-version equivalence is not implied. |
| DD-036 | Support **configurable history retention, checkpoints and pruning**. | SL-006–008; DD-029–030 | A retention policy controls checkpoint frequency and retained history; pruning must not corrupt active branches or retained replay windows. Historical replay outside the retained interval may be unavailable and must be signalled explicitly. Default policy and storage limits remain **OPEN**. |

#### Batch 6 implementation consequences

- **World-size presets:** define a versioned preset catalogue; apply toroidal wrapping consistently at every supported dimension. Save explicit dimensions so later preset changes do not reinterpret existing worlds.
- **Multi-rate scheduler:** drive all subsystems from a single integer simulation clock with deterministic scheduling and tie-breaking. Pause, fast-forward and headless execution must advance the same event sequence.
- **Hybrid plants:** define conservation invariants for biomass, population, resources and allele distributions when changing representation. Statistical approximation must not depend on whether the region is being viewed.
- **Read-only observation:** UI queries and sampling must use independent read-only data or independent display RNG streams. Observing a region cannot trigger a simulation-detail change.
- **Cross-platform reproducibility:** avoid platform-dependent floating-point reductions, unordered container iteration and nondeterministic thread scheduling in state transitions. Consider fixed-point arithmetic or precisely specified integer operations and a fixed PRNG; use canonical state hashing in CI across target platforms.
- **Retention and branching:** distinguish checkpoint state, authoritative inputs/interventions, derived analytics and optional visual replay. A branch must retain its required ancestor checkpoint and intervening history; prune only data not referenced by retained branches. Exact historical replay is guaranteed only within retained history.

#### Additional acceptance tests

| Test | Expected outcome |
|---|---|
| AT-DD31 | Every world-size preset creates a valid toroidal world; seam adjacency, generation, save and load work at each size. |
| AT-DD32 | Equal seeds and interventions produce identical state hashes under pause/resume, different rendering rates, and normal versus accelerated simulation. Coincident subsystem updates follow a documented stable order. |
| AT-DD33 | Hybrid plant representation conserves defined population/biomass/resource quantities and retains genetic diversity sufficient for subsequent inheritance and speciation tests. |
| AT-DD34 | Two otherwise identical runs, one continuously observed and one never observed, yield identical simulation state hashes at checkpoints. |
| AT-DD35 | Identical seed, configuration, engine version and ordered interventions produce matching canonical state hashes on every supported operating system and CPU architecture. |
| AT-DD36 | Retention/pruning respects configured limits while keeping active branch ancestry and supported replay intact; attempts to replay pruned periods report unavailability. |

### Approved Batch 7 decisions — simulation foundations (12 November 2026)

These are **[DESIGN]** decisions approved by the designer, not documented original SimLife algorithms. They supersede the Batch 7 proposals in v0.7.

| ID | Approved decision | Requirements affected | Observable behaviour / acceptance criterion |
|---|---|---|---|
| DD-037 | Define **three world-size presets**, with actual dimensions established through benchmarking. | DD-019, DD-031, DD-035 | World creation offers three stable size categories. Benchmark results, not arbitrary assumptions, determine tile dimensions and supported population budgets. Each saved world records its actual dimensions and preset identifier. |
| DD-038 | **One simulation tick represents one day**; other systems execute at day-based intervals. | SL-003–005, DD-006, DD-021, DD-032 | Integer day is the authoritative clock; ecology, reproduction and environment schedules are expressed as positive integer day intervals. Simulation speed changes throughput, never simulated time per tick. The year length and coincident-event order remain open. |
| DD-039 | **Trait-specific allele expression**: a gene may use dominant, recessive or additive rules. | DD-007–010, DD-041 | Each gene definition declares its expression model; phenotype calculation uses that rule consistently in individual and aggregated representations. Exact allele encoding and expression mathematics remain open. |
| DD-040 | **Genetic reproductive compatibility** determines mating; species classification emerges from reproductive isolation. | DD-009, DD-011, DD-027 | A mating attempt checks explicit genetic compatibility, not merely a species label. Species/lineage classification reflects reproductive isolation; operational thresholds and classification timing remain open. |
| DD-041 | Aggregate populations retain **exact counts, resources and allele distributions**; individual histories and behaviour may be approximated. | DD-003, DD-018, DD-030, DD-033–035 | Aggregate updates and representation transitions preserve the defined exact quantities. Histories and behaviour estimates are labelled statistical; exact allele distribution must be defined at sufficient resolution to support inheritance and compatibility. |
| DD-042 | **Single-threaded simulation for Phase 1**; optimise later. | DD-030, DD-032, DD-035 | One authoritative simulation thread executes a documented stable update sequence; identical seeds and inputs yield identical state hashes on all supported platforms. Parallel simulation is not a Phase 1 requirement. |

#### Batch 7 implementation consequences

- **Preset benchmarking:** create reproducible workloads and performance targets before assigning dimensions to Small, Medium and Large. The catalogue must be versioned so existing saves retain their dimensions.
- **Day-based scheduling:** use integer day numbers and integer interval divisibility; specify exact ordering when multiple subsystems execute on the same day. Do not equate a day with a rendering frame or wall-clock duration.
- **Trait-specific genetics:** a gene schema must identify allele domain, ploidy, expression rule and phenotype mapping. Do not assume all traits have the same inheritance model.
- **Reproductive isolation:** compatibility checks must work for different lineages and across aggregate representations; species labels are outcomes, not the primary mating gate.
- **Aggregate fidelity:** storing marginal allele frequencies alone may be insufficient for linked genes, genotype combinations or mate compatibility. The mechanics specification must define which joint distributions are conserved exactly and what aggregation is permitted.
- **Deterministic single-threading:** specify stable entity traversal, fixed PRNG streams, arithmetic and serialisation. Rendering and observation must never consume simulation randomness.

#### Additional acceptance tests

| Test | Expected outcome |
|---|---|
| AT-DD37 | Three size categories exist; benchmark artefacts justify dimensions; each category generates and loads toroidal maps correctly. |
| AT-DD38 | One tick increments simulation time by exactly one day; different playback speeds yield identical state hashes at the same day. |
| AT-DD39 | Dominant, recessive and additive gene fixtures express the documented phenotype values and reproduce across save/load. |
| AT-DD40 | Mating succeeds or fails based on genetic compatibility regardless of provisional species labels; isolation produces reproducible lineage classification. |
| AT-DD41 | Aggregation, simulation and reconstruction conserve exact population/resource/allele quantities defined by the chosen representation; approximations are documented. |
| AT-DD42 | Reference simulation is single-threaded; identical input fixtures yield identical state hashes across supported architectures and operating systems. |

**Next deliverable:** `SimLife_Phase1_Mechanics_Specification.md` is the companion detailed-mechanics specification. Proposed algorithms in that document are not approved until explicitly accepted.

### Further unresolved mechanics (not yet design-approved)

- Numerical energy, feeding, hydration, ageing, death, growth, movement and combat formulas.
- Genetic loci, allele representation, dominance, mutation rates and environmental mutagen effects.
- Perception radius, memory capacity/decay, drive arbitration and encounter state transitions.
- Water availability, climate/weather dynamics, season lengths, disaster frequencies and effects.
- Scenario catalogue, objective/scoring formulae, intervention budgets and restrictions.
- Snapshot format, event granularity, history defaults, checkpoint cadence and branch retention.

These are **[U]** where the manual does not disclose the mechanism, or **[DESIGN OPEN]** where an explicit new design policy is required. No numerical values should be represented as original-game facts without evidence.


## 18. Approved platform, architecture and planetary-world decisions (12 November 2026)

**Status:** These are explicit design decisions and architectural constraints **[A]**, not mechanics extracted from the original SimLife manual **[D]**. They supersede earlier references to a mandatory Godot client, local-only execution, SQLite and Phase 1 being an offline desktop executable. IDs DD-043–DD-050 are reserved here for these approved choices.

| ID | Decision | Phase 1 obligation | Later-phase boundary |
|---|---|---|---|
| DD-043 | **Rust** is the implementation language for the authoritative simulation engine and initial server. C++ is excluded. | Build native Rust engine and server; engine runs headlessly. | Reuse engine unchanged at the integration boundary; internal mechanics may be revised and versioned. |
| DD-044 | **Client-independent, web-first design.** Godot is optional, not mandatory. | Browser client provides world management, editors, observation and charts; simulation has no UI dependencies. | Android app and optional Godot client use the same public API; responsive web/PWA is a possible delivery route, not yet mandated. |
| DD-045 | Server runs on **Linux**. | Deploy/test Linux-native server process. | Scale/deploy on Linux as required. |
| DD-046 | **PostgreSQL** is the persistent database; no SQLite requirement. | Store accounts, world metadata, species designs, checkpoints, interventions and history as appropriate. Simulation hot state runs in memory rather than issuing database operations for every organism/tick. | Add shared-world access, audit and scalability policies. |
| DD-047 | Client/server interface uses a **versioned JSON API**, with HTTP and WebSocket transport. | All state-changing client actions are validated server commands; clients consume queries, snapshots and events. Define versioning, ordering and errors. | Shared client protocol for web, Android and Godot; authentication and multi-client synchronisation grow as needed. |
| DD-048 | Phase 1 is **single-player gameplay on a server-hosted architecture**. | Account login and private worlds; simulation continues according to its configured run/pause policy independently of browser sessions. | Add multiple players to shared worlds, not a replacement simulation engine. |
| DD-049 | Support **web, Android and optional Godot** as client targets. | Implement web first; avoid assumptions that require a desktop renderer. | Android UI may be responsive web/PWA or separately packaged/native; Godot may be implemented later. |
| DD-050 | **Planet/world isolation and deliberate transfers.** Planetary worlds are independent persistent simulations; no automatic migration. A player may copy the genome of a selected organism/design from Planet A and introduce it into Planet B, retaining source provenance and establishing a separate evolutionary branch. | Multiple private independent worlds and portable, versioned organism/genome definitions; actual interplanetary transfer gameplay is **deferred**. | Player-initiated transfers, subject to destination permissions and release rules; source organism/population remains intact by default. No implicit world-clock synchronisation. |

### 18.1 Architectural component boundaries [A]

1. **Simulation core (Rust library):** authoritative world state, organisms, genetics, ecology, environment, clock and deterministic transitions. No dependency on HTTP, WebSockets, PostgreSQL, Godot, browser code or wall-clock rendering.
2. **Server application (Rust/Linux):** world lifecycle, scheduled execution, command validation, account/session interfaces, client subscriptions and persistence coordination.
3. **Persistence (PostgreSQL):** durable world/account/species metadata, checkpoints, event/intervention history, lineage provenance and retention metadata. Exact schemas and checkpoint encoding remain **DESIGN OPEN**.
4. **Protocol (versioned JSON):** world creation/selection, organism editing/introduction, observation, pause/speed control, snapshots, events and errors. HTTP and WebSocket roles, schema and synchronisation rules remain **DESIGN OPEN**.
5. **Clients:** browser first; future Android and optional Godot clients may implement different presentation/interaction models without changing the authoritative simulation.

**Technology candidates, not yet approved as requirements:** Axum, Tokio, SQLx, Serde and tracing for Rust; Canvas/WebGL for rendering; PWA/Tauri/Capacitor for Android. Choosing Rust/Linux/PostgreSQL does not automatically approve a specific framework or deployment topology.

### 18.2 Planet/world model [A]

- **Planet definition:** reusable environmental template (e.g. atmosphere, temperature, gravity, terrain/water, resources, energy availability and ecological constraints). The precise schema is **DESIGN OPEN**.
- **World instance:** an independent persistent simulation with its own state, clock, random streams, populations, history and access controls. Multiple instances may share a planet definition while evolving differently.
- **Organism/genome definition:** portable across worlds; environmental viability is evaluated in the destination world, not guaranteed by successful export. Distinguish physiological incompatibility from ecological failure (competition, starvation, predation).
- **Transfer provenance:** source world, source organism or genome/design identifier, source generation/lineage, destination world and introduction event. The source world is not mutated by a default copy transfer.
- **Initial terrestrial model:** Phase 1 implements terrestrial ecology. Future ocean-only and hydrogen-rich planetary types are conceptual examples, **not** promises of Phase 1 biological support. Keep extensibility boundaries, but do not invent alien metabolic rules.
- **Scenarios versus worlds:** scenarios are optional objectives/rule sets attached to a world or experiment; a world/planet is the primary environment selection unit for multiplayer.

### 18.3 Multiplayer-readiness invariants [A]

- No Godot/browser or rendering-loop dependencies in simulation code.
- No assumption of a single world, single connected client or continuous client connectivity.
- No direct client mutation of authoritative simulation memory; changes enter through validated, ordered commands.
- Do not encode player ownership as an intrinsic biological trait; track authorship, rights and permissions separately.
- World clocks and simulation state remain independent; do not introduce automatic cross-world events.
- Version the protocol, genome format, world state and checkpoints to support future evolution of mechanics.
- Deterministic replay must record accepted interventions and execution order; database writes and client observation must not consume simulation randomness.

### 18.4 Architecture acceptance criteria

| Test | Expected outcome |
|---|---|
| AT-DD43 | Rust headless engine runs without network, UI or database and produces deterministic checkpoint hashes. |
| AT-DD44 | Browser client connects solely through the public API; replacing the client does not alter simulation state transitions. |
| AT-DD45 | Server builds and runs on a supported Linux host. |
| AT-DD46 | Restart restores a persisted world from PostgreSQL; ordinary simulation ticks do not require per-organism SQL round trips. |
| AT-DD47 | Versioned JSON commands are validated and rejected safely when malformed, unsupported or unauthorised. |
| AT-DD48 | Two private Phase 1 worlds can progress independently; disconnecting the browser does not implicitly terminate either world. |
| AT-DD49 | API is client-neutral and documented sufficiently for an independent Android or Godot implementation. |
| AT-DD50 | Two worlds have independent clocks, populations and RNG state; copying a genome into a destination creates a traceable new introduction without altering the source; no automatic transfers occur. Transfer workflow acceptance is scheduled for Phase 2. |

### 18.5 Outstanding architecture decisions (not approved)

- Whether initial hosted Phase 1 requires public internet access or is restricted to a private Linux host; deployment security and reverse proxy.
- Exact Rust frameworks, web frontend framework, authentication mechanism and API schema.
- World concurrency, server process model, memory limits, idle-world policy, snapshot frequency and crash recovery guarantees.
- Transfer permissions, ownership, costs, quarantine, allowed source designs, population quantities and whether physical relocation should ever be offered.
- Planet definition schema and how non-terrestrial biochemistry will be represented in Phase 3.
