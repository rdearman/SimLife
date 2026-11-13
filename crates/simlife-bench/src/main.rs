use serde_json::json;
use simlife_domain::{
    Alleles, Genome, SimulationCommand, WorldConfig, WorldId, WorldSize, WorldSnapshot,
};
use simlife_engine::{World, CURRENT_MECHANICS_VERSION, LEGACY_MECHANICS_VERSION};
use std::{collections::HashMap, env, fs, hint::black_box, process::ExitCode, time::Instant};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            eprintln!("usage: simlife-bench <organisms> [ticks] [seed] [small|medium|large] [sterile|reproductive]\n       simlife-bench stability [seed] [founders] [small|medium|large] [fertility:1..100] [1|2|both]");
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
