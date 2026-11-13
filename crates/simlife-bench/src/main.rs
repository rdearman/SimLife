use serde_json::json;
use simlife_domain::{Alleles, Genome, SimulationCommand, WorldConfig, WorldId, WorldSize};
use simlife_engine::World;
use std::{env, fs, hint::black_box, process::ExitCode, time::Instant};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            eprintln!("usage: simlife-bench <organisms> [ticks] [seed] [small|medium|large] [sterile|reproductive]");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let population = parse_arg(args.next(), 1_000_usize, "organisms")?;
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
