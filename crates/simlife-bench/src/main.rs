use serde_json::json;
use simlife_domain::{
    Alleles, Diet, Genome, RunState, Sex, SimulationCommand, WorldConfig, WorldId, WorldSize,
    WorldSnapshot,
};
use simlife_engine::{World, CURRENT_MECHANICS_VERSION, LEGACY_MECHANICS_VERSION};
use std::{collections::HashMap, env, fs, hint::black_box, process::ExitCode, time::Instant};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            eprintln!("usage: simlife-bench <organisms> [ticks] [seed] [small|medium|large] [sterile|reproductive]\n       simlife-bench stability [seed] [founders] [small|medium|large] [fertility:1..100] [1|2|both]\n       simlife-bench gameplay [seed] [days]");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let first = args.next();
    if first.as_deref() == Some("stability") {
        return run_stability(args);
    }
    if first.as_deref() == Some("gameplay") {
        return run_gameplay(args);
    }
    let population = parse_arg(first, 1_000_usize, "organisms")?;
    let ticks = parse_arg(args.next(), 100_u64, "ticks")?;
    let seed = parse_arg(args.next(), 20_261_113_u64, "seed")?;
    let size = match args.next().as_deref().unwrap_or("small") {
        "small" => WorldSize::Small,
        "medium" => WorldSize::Medium,
        "large" => WorldSize::Large,
        _ => return Err("map size must be small, medium, or large".into()),
    };
    let mode = args.next().unwrap_or_else(|| "sterile".into());
    if !matches!(mode.as_str(), "sterile" | "reproductive") {
        return Err("benchmark mode must be sterile or reproductive".into());
    }
    if args.next().is_some()
        || !(1..=100_000).contains(&population)
        || !(1..=100_000).contains(&ticks)
    {
        return Err("organisms and ticks must both be between 1 and 100000".into());
    }

    let memory_before = memory_kib("VmRSS");
    let setup_started = Instant::now();
    let config = WorldConfig {
        name: "benchmark".into(),
        seed,
        size,
        tick_duration_ms: 1000,
    };
    let mut world = World::new(WorldId::new(), config).map_err(|error| error.to_string())?;
    let genome = Genome {
        speed: Alleles {
            first: 1,
            second: 1,
        },
        metabolism: Alleles {
            first: 2,
            second: 2,
        },
        fertility: Alleles {
            first: if mode == "reproductive" { 100 } else { 0 },
            second: if mode == "reproductive" { 100 } else { 0 },
        },
        ..Genome::default()
    };
    world
        .seed_population(population, genome)
        .map_err(|error| error.to_string())?;
    let setup_seconds = setup_started.elapsed().as_secs_f64();
    let memory_after_setup = memory_kib("VmRSS");

    world
        .apply(SimulationCommand::Start)
        .map_err(|error| error.to_string())?;
    let started = Instant::now();
    for _ in 0..ticks {
        world
            .apply(SimulationCommand::Advance { ticks: 1 })
            .map_err(|error| error.to_string())?;
        black_box(world.update());
    }
    let elapsed_seconds = started.elapsed().as_secs_f64();
    let final_state = world.summary();
    let population_change_percent =
        (final_state.population as f64 - population as f64) / population as f64 * 100.0;

    println!(
        "{}",
        json!({
            "seed": seed,
            "world_size": format!("{:?}", size).to_lowercase(),
            "dimensions": [final_state.width, final_state.height],
            "initial_population": population,
            "mode": mode,
            "final_population": final_state.population,
            "population_change_percent": population_change_percent,
            "ticks": ticks,
            "elapsed_seconds": elapsed_seconds,
            "ticks_per_second": ticks as f64 / elapsed_seconds.max(f64::MIN_POSITIVE),
            "initial_organism_days_per_second": population as f64 * ticks as f64 / elapsed_seconds.max(f64::MIN_POSITIVE),
            "setup_seconds": setup_seconds,
            "births": final_state.statistics.births,
            "deaths": final_state.statistics.deaths,
            "peak_memory_kib": memory_kib("VmHWM"),
            "memory_before_kib": memory_before,
            "memory_after_setup_kib": memory_after_setup,
            "memory_after_run_kib": memory_kib("VmRSS"),
            "state_hash": final_state.state_hash,
        })
    );
    Ok(())
}

fn run_stability(mut args: impl Iterator<Item = String>) -> Result<(), String> {
    let seed = parse_arg(args.next(), 20_261_113_u64, "seed")?;
    let founders = parse_arg(args.next(), 1_000_usize, "founders")?;
    let size = match args.next().as_deref().unwrap_or("medium") {
        "small" => WorldSize::Small,
        "medium" => WorldSize::Medium,
        "large" => WorldSize::Large,
        _ => return Err("map size must be small, medium, or large".into()),
    };
    let fertility = parse_arg(args.next(), 20_u8, "fertility")?;
    let version_selection = args.next().unwrap_or_else(|| "both".into());
    if args.next().is_some()
        || !(1..=100_000).contains(&founders)
        || !(1..=100).contains(&fertility)
        || !matches!(version_selection.as_str(), "1" | "2" | "both")
    {
        return Err(
            "founders must be 1..=100000 and fertility 1..=100; no extra arguments are accepted"
                .into(),
        );
    }

    let targets = [1_000_u64, 5_000, 10_000];
    let versions: Vec<u16> = match version_selection.as_str() {
        "1" => vec![LEGACY_MECHANICS_VERSION],
        "2" => vec![CURRENT_MECHANICS_VERSION],
        _ => vec![LEGACY_MECHANICS_VERSION, CURRENT_MECHANICS_VERSION],
    };
    let runs = versions
        .into_iter()
        .map(|mechanics_version| {
            run_stability_cohort(seed, founders, size, fertility, mechanics_version, &targets)
        })
        .collect::<Result<Vec<_>, _>>()?;
    println!(
        "{}",
        json!({
            "benchmark": "evolutionary_stability",
            "seed": seed,
            "initial_population": founders,
            "fertility": fertility,
            "version_selection": version_selection,
            "world_size": format!("{size:?}").to_lowercase(),
            "target_days": targets,
            "mechanics_versions": runs,
            "process_peak_memory_kib": memory_kib("VmHWM"),
        })
    );
    Ok(())
}

fn run_stability_cohort(
    seed: u64,
    founders: usize,
    size: WorldSize,
    fertility: u8,
    mechanics_version: u16,
    targets: &[u64],
) -> Result<serde_json::Value, String> {
    let config = WorldConfig {
        name: format!("stability-v{mechanics_version}"),
        seed,
        size,
        tick_duration_ms: 1000,
    };
    let mut world = World::new_with_mechanics(WorldId::new(), config, mechanics_version)
        .map_err(|error| error.to_string())?;
    let genome = Genome {
        speed: Alleles {
            first: 1,
            second: 1,
        },
        metabolism: Alleles {
            first: 2,
            second: 2,
        },
        fertility: Alleles {
            first: fertility,
            second: fertility,
        },
        ..Genome::default()
    };
    world
        .seed_population(founders, genome)
        .map_err(|error| error.to_string())?;
    world
        .apply(SimulationCommand::Start)
        .map_err(|error| error.to_string())?;

    let mut previous_target = 0;
    let mut reports = Vec::new();
    let started = Instant::now();
    for target in targets {
        let mut remaining = target - previous_target;
        while remaining > 0 {
            let ticks = remaining.min(100);
            world
                .apply(SimulationCommand::Advance { ticks })
                .map_err(|error| error.to_string())?;
            remaining -= ticks;
        }
        previous_target = *target;
        let snapshot = world.snapshot();
        let restored = World::restore(&snapshot)
            .map_err(|error| format!("checkpoint restore failed: {error}"))?;
        if restored.snapshot().state_hash != snapshot.state_hash {
            return Err(format!(
                "checkpoint hash mismatch at version {mechanics_version}, day {target}"
            ));
        }
        reports.push(stability_metrics(
            &snapshot,
            started.elapsed().as_secs_f64(),
        ));
        eprintln!(
            "stability mechanics v{mechanics_version}: reached day {target}, population {}, {} speciation events",
            snapshot.organisms.len(),
            snapshot.speciation_events.len()
        );
    }
    Ok(json!({
        "mechanics_version": mechanics_version,
        "state_hash": world.summary().state_hash,
        "checkpoints": reports,
    }))
}

/// Runs a compact, seeded set of game-facing experiments. Results are evidence about
/// outcomes and agency signals; they are not a substitute for human playtesting.
fn run_gameplay(mut args: impl Iterator<Item = String>) -> Result<(), String> {
    let seed = parse_arg(args.next(), 20_261_113_u64, "seed")?;
    let days = parse_arg(args.next(), 1_000_u64, "days")?;
    if args.next().is_some() || !(100..=100_000).contains(&days) || seed == 0 {
        return Err(
            "seed must be non-zero, days must be 100..=100000, and no extra arguments are accepted"
                .into(),
        );
    }
    let started = Instant::now();
    let experiments = run_gameplay_suite(seed, days)?;
    println!(
        "{}",
        json!({
            "benchmark": "gameplay_validation",
            "seed": seed,
            "days": days,
            "mechanics_version": CURRENT_MECHANICS_VERSION,
            "elapsed_seconds": started.elapsed().as_secs_f64(),
            "experiments": experiments,
            "scorecard": {
                "agency": {"status": "partially_testable", "evidence": "matched genome choices and interventions are compared"},
                "tradeoffs": {"status": "not_established", "evidence": "one seed and a limited trait set cannot identify universally inferior strategies"},
                "feedback": {"status": "partially_testable", "evidence": "population, diet, births, deaths, predation and hashes are reported"},
                "challenge": {"status": "not_established", "evidence": "the engine has no objectives, difficulty setting or win/loss state"},
                "emergence": {"status": "partially_testable", "evidence": "population, diet and lineage outcomes vary under interventions"},
                "replayability": {"status": "testable", "evidence": "identical snapshot branches are hash-compared against a changed experiment"},
                "human_enjoyment": "requires playtesting"
            },
            "known_limits": [
                "The engine has no editable climate or resource controls; survival uses the driest/lowest-vegetation seed found in a 12-seed search.",
                "Agent strategies are fixed genome recipes; there is no decision-making or adaptive player agent.",
                "Interventions can introduce organisms, but cannot remove organisms or alter terrain/resources.",
                "Lineage records are ancestry branches and are not validated as reproductively isolated species."
            ],
            "process_peak_memory_kib": memory_kib("VmHWM")
        })
    );
    Ok(())
}

fn run_gameplay_suite(seed: u64, days: u64) -> Result<Vec<serde_json::Value>, String> {
    Ok(vec![
        survival_experiment(seed, days)?,
        evolution_experiment(seed, days)?,
        predator_prey_experiment(seed, days)?,
        intervention_experiment(seed, days)?,
        replay_experiment(seed, days)?,
    ])
}

fn survival_experiment(seed: u64, days: u64) -> Result<serde_json::Value, String> {
    let candidate_count = 12_u64;
    let mut candidates = Vec::new();
    for offset in 0..candidate_count {
        let candidate_seed = seed.wrapping_mul(1_000).wrapping_add(offset).max(1);
        let world = new_game_world(candidate_seed, "survival-environment")?;
        let snapshot = world.snapshot();
        let vegetation = snapshot
            .tiles
            .iter()
            .map(|tile| u64::from(tile.resources.vegetation))
            .sum::<u64>();
        let water = snapshot
            .tiles
            .iter()
            .map(|tile| u64::from(tile.resources.water))
            .sum::<u64>();
        let deserts = snapshot
            .tiles
            .iter()
            .filter(|tile| tile.terrain == simlife_domain::TerrainType::Desert)
            .count();
        candidates.push((
            vegetation.saturating_add(water),
            candidate_seed,
            vegetation,
            water,
            deserts,
        ));
    }
    candidates.sort_by_key(|candidate| (candidate.0, candidate.1));
    let (_, challenge_seed, vegetation, water, deserts) = candidates[0];
    let profiles = [
        (
            "specialist_low_metabolism",
            Genome {
                speed: alleles(1),
                metabolism: alleles(0),
                fertility: alleles(20),
                lifespan_days: 1500,
                diet: Diet::Herbivore,
                sex: Sex::Asexual,
            },
        ),
        ("random_genome_baseline", random_genome(seed ^ 0x5345_4544)),
    ];
    let mut outcomes = Vec::new();
    for (name, profile) in profiles {
        let mut world = new_game_world(challenge_seed, name)?;
        let genomes = if name == "random_genome_baseline" {
            (0..32)
                .map(|i| {
                    let mut genome = random_genome(seed.wrapping_add(i));
                    genome.fertility = alleles(20);
                    genome.lifespan_days = 1500;
                    genome
                })
                .collect::<Vec<_>>()
        } else {
            vec![profile.clone(); 32]
        };
        populate(&mut world, &genomes, 0)?;
        advance(&mut world, days)?;
        let recipe = if name == "random_genome_baseline" {
            json!({"varying_traits":["speed","metabolism","diet"],"fixed_fertility":20,"fixed_lifespan_days":1500,"sex":"asexual"})
        } else {
            json!(profile)
        };
        outcomes.push(json!({"strategy":name,"initial_population":32,"final":world_metrics(&world),"genome_recipe":recipe}));
    }
    Ok(
        json!({"id":"survival_challenge","environment":"minimum initial water+vegetation among 12 candidate seeds","environment_seed":challenge_seed,"initial_vegetation":vegetation,"initial_water":water,"desert_tiles":deserts,"outcomes":outcomes}),
    )
}

fn evolution_experiment(seed: u64, days: u64) -> Result<serde_json::Value, String> {
    let mut world = new_game_world(seed, "diverging-populations")?;
    let slow = Genome {
        speed: alleles(0),
        metabolism: alleles(3),
        fertility: alleles(20),
        lifespan_days: 700,
        diet: Diet::Herbivore,
        sex: Sex::Asexual,
    };
    let fast = Genome {
        speed: alleles(4),
        metabolism: alleles(3),
        fertility: alleles(20),
        lifespan_days: 700,
        diet: Diet::Herbivore,
        sex: Sex::Asexual,
    };
    let mut founders = vec![slow; 20];
    founders.extend(vec![fast; 20]);
    populate(&mut world, &founders, 3)?;
    advance(&mut world, days)?;
    let snapshot = world.snapshot();
    let lineages_by_id: HashMap<_, _> = snapshot
        .species_lineages
        .iter()
        .map(|lineage| (lineage.species_id, lineage))
        .collect();
    let groups = [("slow", 1_u64, 20_u64), ("fast", 21, 40)].into_iter().map(|(name, first, last)| {
        let lineages: Vec<_> = snapshot.species_lineages.iter().filter(|lineage| {
            lineage_root_founder(lineage.species_id, &lineages_by_id).is_some_and(|founder| (first..=last).contains(&founder))
        }).collect();
        let group_lineage_ids: std::collections::HashSet<_> = lineages.iter().map(|lineage| lineage.species_id).collect();
        let living = snapshot.organisms.iter().filter(|organism| group_lineage_ids.contains(&organism.species_id)).count();
        json!({"strategy":name,"population":living,"lineage_branches":lineages.len(),"speciation_events":snapshot.speciation_events.iter().filter(|event| group_lineage_ids.contains(&event.new_species_id)).count()})
    }).collect::<Vec<_>>();
    Ok(
        json!({"id":"diverging_populations","setup":"20 low-speed and 20 high-speed asexual herbivores in one seeded world","groups":groups,"final":world_metrics(&world)}),
    )
}

fn lineage_root_founder(
    species_id: u64,
    lineages: &HashMap<u64, &simlife_domain::SpeciesLineage>,
) -> Option<u64> {
    let mut current = species_id;
    loop {
        let lineage = lineages.get(&current)?;
        if let Some(parent_id) = lineage.parent_species_id {
            current = parent_id;
        } else {
            return Some(lineage.founder_organism_id);
        }
    }
}

fn predator_prey_experiment(seed: u64, days: u64) -> Result<serde_json::Value, String> {
    let mut world = new_game_world(seed, "predator-prey")?;
    let prey = Genome {
        speed: alleles(2),
        metabolism: alleles(2),
        fertility: alleles(15),
        lifespan_days: 700,
        diet: Diet::Herbivore,
        sex: Sex::Asexual,
    };
    let predator = Genome {
        speed: alleles(3),
        metabolism: alleles(2),
        fertility: alleles(8),
        lifespan_days: 600,
        diet: Diet::Carnivore,
        sex: Sex::Asexual,
    };
    let mut founders = vec![prey; 32];
    founders.extend(vec![predator; 8]);
    populate(&mut world, &founders, 0)?;
    advance(&mut world, days)?;
    Ok(
        json!({"id":"predator_prey","setup":"32 herbivores and 8 carnivores released at shared habitat coordinates","final":world_metrics(&world),"interpretation":"Predation deaths and final diet counts indicate whether this setup creates an observable food-chain response."}),
    )
}

fn intervention_experiment(seed: u64, days: u64) -> Result<serde_json::Value, String> {
    let mut initial = new_game_world(seed, "ecosystem-branch")?;
    let prey = Genome {
        speed: alleles(2),
        metabolism: alleles(2),
        fertility: alleles(15),
        lifespan_days: 700,
        diet: Diet::Herbivore,
        sex: Sex::Asexual,
    };
    let predator = Genome {
        speed: alleles(3),
        metabolism: alleles(2),
        fertility: alleles(8),
        lifespan_days: 600,
        diet: Diet::Carnivore,
        sex: Sex::Asexual,
    };
    let mut founders = vec![prey.clone(); 24];
    founders.extend(vec![predator.clone(); 6]);
    populate(&mut initial, &founders, 0)?;
    let branch_day = (days / 3).max(1);
    advance(&mut initial, branch_day)?;
    let checkpoint = initial.snapshot();
    let mut control = World::restore(&checkpoint).map_err(|error| error.to_string())?;
    let mut disruption = World::restore(&checkpoint).map_err(|error| error.to_string())?;
    let mut restoration = World::restore(&checkpoint).map_err(|error| error.to_string())?;
    let extra_predator = Genome {
        fertility: alleles(0),
        ..predator
    };
    introduce_group(&mut disruption, &vec![extra_predator.clone(); 16], 13)?;
    introduce_group(&mut restoration, &vec![extra_predator; 16], 13)?;
    advance(&mut control, days - branch_day)?;
    let intervention_days = (days - branch_day) / 2;
    advance(&mut disruption, days - branch_day)?;
    advance(&mut restoration, intervention_days)?;
    introduce_group(&mut restoration, &vec![prey; 24], 29)?;
    advance(&mut restoration, days - branch_day - intervention_days)?;
    Ok(
        json!({"id":"ecological_intervention","branch_day":branch_day,"setup":"control vs +16 carnivores; restoration branch adds 24 herbivores halfway through","outcomes":{"control":world_metrics(&control),"predator_introduction":world_metrics(&disruption),"predator_then_herbivore_introduction":world_metrics(&restoration)}}),
    )
}

fn replay_experiment(seed: u64, days: u64) -> Result<serde_json::Value, String> {
    let mut initial = new_game_world(seed, "replay-branch")?;
    let founders = vec![
        Genome {
            fertility: alleles(12),
            ..Genome::default()
        };
        20
    ];
    populate(&mut initial, &founders, 2)?;
    advance(&mut initial, days / 4)?;
    let branch = initial.snapshot();
    let mut first = World::restore(&branch).map_err(|error| error.to_string())?;
    let mut replay = World::restore(&branch).map_err(|error| error.to_string())?;
    let extra = Genome {
        diet: Diet::Omnivore,
        fertility: alleles(25),
        ..Genome::default()
    };
    let position = 7;
    for world in [&mut first, &mut replay] {
        world
            .apply(SimulationCommand::IntroduceOrganism {
                x: position,
                y: position,
                genome: extra.clone(),
            })
            .map_err(|error| error.to_string())?;
    }
    advance(&mut first, days - days / 4)?;
    advance(&mut replay, days - days / 4)?;
    let identical_hashes_match = first.summary().state_hash == replay.summary().state_hash;
    let control_hash = first.summary().state_hash;
    let mut alternative = World::restore(&branch).map_err(|error| error.to_string())?;
    let different = Genome {
        diet: Diet::Herbivore,
        fertility: alleles(40),
        ..Genome::default()
    };
    alternative
        .apply(SimulationCommand::IntroduceOrganism {
            x: position,
            y: position,
            genome: different,
        })
        .map_err(|error| error.to_string())?;
    advance(&mut alternative, days - days / 4)?;
    Ok(
        json!({"id":"replay_and_hypothesis","identical_branch_hashes_match":identical_hashes_match,"alternative_genome_changes_hash":control_hash!=alternative.summary().state_hash,"repeated_intervention":world_metrics(&first),"alternative_intervention":world_metrics(&alternative)}),
    )
}

fn new_game_world(seed: u64, name: &str) -> Result<World, String> {
    World::new_with_mechanics(
        WorldId::new(),
        WorldConfig {
            name: name.into(),
            seed: seed.max(1),
            size: WorldSize::Small,
            tick_duration_ms: 1000,
        },
        CURRENT_MECHANICS_VERSION,
    )
    .map_err(|error| error.to_string())
}

fn populate(world: &mut World, genomes: &[Genome], offset: u16) -> Result<(), String> {
    for (index, genome) in genomes.iter().enumerate() {
        let x = (offset + ((index as u16 * 7) % 32)) % 32;
        let y = (index as u16 * 13 + offset) % 64;
        world
            .apply(SimulationCommand::IntroduceOrganism {
                x,
                y,
                genome: genome.clone(),
            })
            .map_err(|error| error.to_string())?;
    }
    world
        .apply(SimulationCommand::Start)
        .map_err(|error| error.to_string())
}

fn introduce_group(world: &mut World, genomes: &[Genome], offset: u16) -> Result<(), String> {
    for (index, genome) in genomes.iter().enumerate() {
        let x = (offset + ((index as u16 * 5) % 32)) % 32;
        let y = (index as u16 * 11 + offset) % 64;
        world
            .apply(SimulationCommand::IntroduceOrganism {
                x,
                y,
                genome: genome.clone(),
            })
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn advance(world: &mut World, days: u64) -> Result<(), String> {
    if world.summary().state == RunState::Created {
        world
            .apply(SimulationCommand::Start)
            .map_err(|error| error.to_string())?;
    }
    let mut remaining = days;
    while remaining > 0 {
        let ticks = remaining.min(100);
        world
            .apply(SimulationCommand::Advance { ticks })
            .map_err(|error| error.to_string())?;
        remaining -= ticks;
    }
    Ok(())
}

fn world_metrics(world: &World) -> serde_json::Value {
    let snapshot = world.snapshot();
    let mut diets = HashMap::new();
    for organism in &snapshot.organisms {
        *diets
            .entry(format!("{:?}", organism.genome.diet).to_lowercase())
            .or_insert(0_u64) += 1;
    }
    let extinct_lineages = snapshot
        .species_lineages
        .iter()
        .filter(|lineage| lineage.extinct_day.is_some())
        .count();
    json!({"day":snapshot.tick,"population":snapshot.organisms.len(),"diet_counts":diets,"births":snapshot.statistics.births,"deaths":snapshot.statistics.deaths,"predation_deaths":snapshot.statistics.predation_deaths,"lineage_branches":snapshot.species_lineages.len(),"extant_lineage_branches":snapshot.species_lineages.len()-extinct_lineages,"state_hash":snapshot.state_hash})
}

fn alleles(value: u8) -> Alleles {
    Alleles {
        first: value,
        second: value,
    }
}

fn random_genome(mut state: u64) -> Genome {
    let mut draw = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let diet = match draw() % 3 {
        0 => Diet::Herbivore,
        1 => Diet::Carnivore,
        _ => Diet::Omnivore,
    };
    Genome {
        speed: alleles((draw() % 5) as u8),
        metabolism: alleles((draw() % 21) as u8),
        fertility: alleles((draw() % 31) as u8),
        lifespan_days: (100 + draw() % 901) as u16,
        diet,
        sex: Sex::Asexual,
    }
}

fn stability_metrics(snapshot: &WorldSnapshot, elapsed_seconds: f64) -> serde_json::Value {
    let total_lineages = snapshot.species_lineages.len();
    let extinct_lineages = snapshot
        .species_lineages
        .iter()
        .filter(|lineage| lineage.extinct_day.is_some())
        .count();
    let population = snapshot.organisms.len();
    json!({
        "day": snapshot.tick,
        "population": population,
        "total_species_lineages": total_lineages,
        "extant_species": total_lineages - extinct_lineages,
        "extinct_species": extinct_lineages,
        "lineage_extinction_fraction": fraction(extinct_lineages, total_lineages),
        "speciation_events": snapshot.speciation_events.len(),
        "speciation_events_per_1000_days": snapshot.speciation_events.len() as f64 * 1000.0 / snapshot.tick.max(1) as f64,
        "speciation_events_per_1000_births": snapshot.speciation_events.len() as f64 * 1000.0 / snapshot.statistics.births.max(1) as f64,
        "mean_allele_heterozygosity": allele_heterozygosity(snapshot),
        "births": snapshot.statistics.births,
        "deaths": snapshot.statistics.deaths,
        "ticks_per_second": snapshot.tick as f64 / elapsed_seconds.max(f64::MIN_POSITIVE),
        "state_hash": snapshot.state_hash,
        "checkpoint_restore_hash_matches": true,
    })
}

fn fraction(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

/// Mean expected heterozygosity for the three diploid numeric trait loci (0..=1).
fn allele_heterozygosity(snapshot: &WorldSnapshot) -> f64 {
    if snapshot.organisms.is_empty() {
        return 0.0;
    }
    let mut loci: [HashMap<u8, u64>; 3] = std::array::from_fn(|_| HashMap::new());
    for organism in &snapshot.organisms {
        for (counts, alleles) in loci.iter_mut().zip([
            organism.genome.speed,
            organism.genome.metabolism,
            organism.genome.fertility,
        ]) {
            *counts.entry(alleles.first).or_default() += 1;
            *counts.entry(alleles.second).or_default() += 1;
        }
    }
    loci.iter()
        .map(|counts| {
            let copies = (snapshot.organisms.len() * 2) as f64;
            1.0 - counts
                .values()
                .map(|count| (*count as f64 / copies).powi(2))
                .sum::<f64>()
        })
        .sum::<f64>()
        / loci.len() as f64
}

fn parse_arg<T: std::str::FromStr>(
    value: Option<String>,
    default: T,
    name: &str,
) -> Result<T, String> {
    value
        .map(|value| value.parse().map_err(|_| format!("invalid {name}")))
        .unwrap_or(Ok(default))
}

fn memory_kib(field: &str) -> Option<u64> {
    let status = fs::read_to_string("/proc/self/status").ok()?;
    status.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        if key != field {
            return None;
        }
        value.split_whitespace().next()?.parse().ok()
    })
}

#[cfg(test)]
mod gameplay_tests {
    use super::run_gameplay_suite;

    #[test]
    fn gameplay_experiments_are_repeatable_and_cover_the_five_cases() {
        let first = run_gameplay_suite(20_261_113, 100).expect("first experiment suite");
        let replay = run_gameplay_suite(20_261_113, 100).expect("repeated experiment suite");

        assert_eq!(first, replay);
        assert_eq!(first.len(), 5);
        assert!(first[4]["identical_branch_hashes_match"].as_bool() == Some(true));
        assert!(first[4]["alternative_genome_changes_hash"].as_bool() == Some(true));
    }
}
