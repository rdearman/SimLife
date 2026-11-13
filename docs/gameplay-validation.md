# SimLife gameplay validation

This document separates machine-verifiable simulation outcomes from human judgments about whether SimLife is enjoyable. The command below implements five repeatable experiments using only existing mechanics; it does not implement new biological rules or score human experience.

## Run the experiment suite

```sh
cargo build --release -p simlife-bench
target/release/simlife-bench gameplay 20261113 1000
```

The positional arguments are a non-zero base seed and simulated days (100–100,000). Results are JSON. The run uses mechanics version 2 and small worlds. The survival experiment picks the candidate seed with the least initial water-plus-vegetation among twelve generated maps derived from the base seed. All other experiments use the base seed. Repeat with other base seeds to assess sensitivity.

## Implemented experiments

| Experiment | Controlled setup | Measures |
| --- | --- | --- |
| Survival challenge | 32 low-metabolism herbivores versus 32 founders with randomized speed, metabolism, and diet; fertility and lifespan held constant | Day-1,000 population, deaths, births, predation, and surviving lineage branches |
| Diverging populations | 20 speed-0 and 20 speed-4 asexual herbivores share one world | Descendant population and lineage branches attributed to each founder group |
| Predator–prey | 32 herbivores and 8 carnivores introduced at deterministic locations in one world | Final diet populations, deaths, and predation deaths |
| Ecological intervention | Fork a mixed ecosystem checkpoint into control, +16 carnivore, and +16 carnivore followed halfway by +24 herbivore branches | Population and predation differences at the end of each branch |
| Replay and experimentation | Restore the same checkpoint twice and apply the same introduction; restore a third time and alter the introduced genome | Identical-branch hash equality and whether the changed experiment changes state hash and outcomes |

Organism positions and genome recipes are fixed by the command, not selected by a player. The random baseline randomizes three traits, but keeps fertility, lifespan, and sex fixed to keep the comparison more focused. The “driest” habitat is only a heuristic based on initial total water and vegetation; the game has no explicit difficulty or climate control.

## Initial results

One release build was run for 1,000 simulated days with seeds 20261113, 20261114, and 20261115. Each run took 2.25–2.73 seconds on the development machine. These are three deterministic samples, not a broad calibration study.

| Seed | Low-metabolism final population | Random-genome final population | Slow / fast herbivore population | Predator–prey final population | Control / predator / predator+herbivore final population |
| --- | ---: | ---: | ---: | ---: | ---: |
| 20261113 | 5,094 | 0 | 1 / 2,169 | 0 | 2,459 / 2,557 / 2,433 |
| 20261114 | 5,121 | 0 | 0 / 1,542 | 3,917 | 0 / 0 / 2,331 |
| 20261115 | 5,221 | 0 | 0 / 1,468 | 0 | 0 / 0 / 2,283 |

Predation deaths in the predator–prey runs were 4,466, 4,603, and 4,662 respectively. The fixed low-metabolism design outperformed the randomized design in these samples, while speed-4 founders displaced speed-0 founders in every sample. This is an early warning that some current trait choices may be overwhelmingly favorable under these setups; it does not establish a universally optimal genome. Predator–prey populations varied sharply by seed. Introducing herbivores after the other intervention branches collapsed restored a non-zero population in two of the three runs. That intervention is a population reset, not proof of ecological restoration or stability.

All three replay experiments reported matching hashes for identical restored branches and a changed hash for the alternative genome branch. The machine evidence scorecard therefore marks replay as testable. Agency, feedback, and emergence are only partially tested from outcome differences. Trade-offs, challenge, and enjoyment remain unassessed by this automated suite.

## Verification performed

- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed, including the new deterministic five-experiment suite test and the existing engine/server tests.
- The PostgreSQL integration test did not exercise a database: `DATABASE_URL` was unset and the local PostgreSQL socket reported no response. Database migration and recovery verification remains outstanding in this sandbox run.
- The release gameplay command completed for the three seeds shown above.

## Species and lineage terminology

Existing API and storage fields named `species_id`, `species_lineages`, and `species_count` retain those names for compatibility. In the current implementation they identify recorded ancestry branches and speciation events. Pairwise genetic compatibility determines mating eligibility; lineage labels do not define mutually isolated biological species. Treat the measured values as lineage-branch counts, not species diversity, until a stable reproductive-group assignment model exists.

## Human gameplay acceptance protocol

Automation cannot establish whether a person understands the feedback, makes meaningful decisions, or wants to play again. The proposed acceptance study is:

1. Recruit at least five people who have not worked on the simulation. Give each person 60 minutes and no developer assistance.
2. Ask each person to create a world, state a hypothesis about an organism design, compare two designs, inspect a predator–prey outcome, and branch an experiment to test an intervention.
3. Record task completion, time to first experiment, abandoned actions, and a short explanation from each person of why they think the outcomes differed.
4. Have each person score agency, trade-offs, feedback, challenge, emergence, and replayability from 0 to 5, with a sentence of evidence for each score. Ask separately whether they would play again and why.
5. Keep individual scores and comments. Report the median and range for each dimension; do not convert machine metrics into human gameplay scores.

For an initial project gate, use the previously proposed bands provisionally: a median total of at least 21/30, a median of at least 3/5 in every dimension, and at least four of five participants saying they would play again. These thresholds have not been validated and should be revisited after the first study. M1–M6 have no human participant data yet, so gameplay acceptance remains open.

## What this does not answer

- The engine does not let players edit terrain, climate, resources, or remove organisms. The intervention suite cannot test those decisions.
- The “agents” are fixed genome recipes. There is no autonomous adaptive agent that chooses actions from observations.
- Automated outcomes do not measure human comprehension, fun, pacing, accessibility, or one-hour retention.
- Three seeds reveal outcome variance but are too few for reliable strategy ranking or universal balance conclusions.
- Lineage-branch counts are not reproductively isolated species counts.
