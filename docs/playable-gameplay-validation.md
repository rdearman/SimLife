# Playable gameplay validation slice

The browser now offers guided objectives over the current simulation mechanics while retaining the free sandbox. These are test scenarios, not a claim that the game has passed human acceptance.

## Scenarios and objectives

| Scenario | Starter setup | Deadline and success condition |
| --- | --- | --- |
| Survival | Eight low-metabolism herbivore founders | Day 250 with at least 8 living organisms |
| Predator–prey balance | Sixteen herbivores and five carnivores sharing a starting location | Day 300 with at least 5 herbivores, 1 carnivore, and one recorded predation death |
| Evolutionary adaptation | Ten fertile herbivores | Day 400 with at least 8 living organisms, 20 births, and 2 distinct live genomes |
| Ecosystem recovery | Twelve herbivores and eight carnivores sharing a starting location | The player introduces at least one organism; at day 400 there are at least 12 herbivores and 1 carnivore |

Each objective records a win or miss when its deadline arrives. A missed objective leaves the world running as a sandbox. The seed control gives a different generated map; the terrain, climate, and resource rules are not player-editable yet.

## Genome feedback and comparison

The editor explains the implemented effects:

- Speed controls movement steps and therefore changes access to food, water, and encounters.
- Metabolism controls daily energy use; low values conserve energy.
- Fertility controls reproduction probability after age 30; reproduction costs parent energy.
- Lifespan is the maximum age.
- Diet changes food access and predation relationships.

From a guided run, a player can create a matched world with the same seed, founder recipes, and introduction positions, changing metabolism, speed, fertility, or herbivore diet. The comparison panel shows both worlds' day, population, births, deaths, and predation deaths. The two clocks are independent; players need to advance both worlds to comparable days.

The selected world also reports the population equation (founders + births − deaths), live land vegetation and water, depleted tiles, cumulative primary death causes, genetic diversity, and lineage branch events. Natural death cause categories use a stable precedence: lifespan, then dehydration, then starvation. The categories are mutually exclusive under that rule. Old checkpoints without this telemetry show the day when complete cause reporting begins. The state hash excludes these derived diagnostics, preserving prior hash behavior.

## Playtest instrumentation

Scenario launches and player actions are stored in the browser's `localStorage`: scenario and attempt number, action timestamps and simulated days, introductions, objective result, elapsed completion time, and page-close abandonment. Use **Export local playtest log** to download JSON for analysis. No playtest data is sent to the server. A repeated launch increments the attempt number for that scenario.

## Acceptance status

| Criterion | Status |
| --- | --- |
| New player can start a scenario, start its clock, and introduce an organism through the labeled map/editor flow | Implemented in the browser; unassisted use not yet observed |
| Four scenarios have explicit, measurable outcomes | Implemented; thresholds are provisional and require calibration |
| Player can change a genome trait and compare matched worlds | Implemented; time alignment is manual |
| Deaths, population change, resources, and lineage events have in-game explanations | Implemented from engine telemetry; no individual death-cause history or causal attribution beyond engine counters |
| Three distinct strategies succeed in different environments | Not established |
| No single genome dominates all scenarios | Not established |
| Unfamiliar players complete a 20–30 minute session and want to retry | Not tested; requires human playtesting |

The previous benchmark found strong low-metabolism survival and fast-herbivore dominance in a small set of seeds. This UI adds ways to form a trait hypothesis and compare outcomes, but it does not yet demonstrate a balanced strategy space. The next validation should recruit unfamiliar players, record task completion and repeat attempts, and run a larger trait-by-seed matrix against the scenario objectives before tuning genome values.

## Verification

The client production build passed with `npm run build`. Rust formatting, Clippy, workspace tests, and workspace build passed after adding resource and death-diagnosis telemetry. The new telemetry was not checked against a live PostgreSQL recovery run in this sandbox because PostgreSQL was unavailable; old serialized statistics fields have serde defaults, and derived diagnostic fields are excluded from the canonical state hash.
