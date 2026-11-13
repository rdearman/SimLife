use blake3::Hasher;
use serde::Serialize;
use simlife_domain::{
    AlleleFrequencySample, Alleles, Diet, Genome, Organism, OrganismInspection, OrganismLineage,
    PopulationSample, Resources, RunState, Sex, SimulationCommand, SpeciesLineage, TerrainType,
    Tile, WorldConfig, WorldId, WorldSnapshot, WorldStatistics, WorldSummary, WorldUpdate,
};
use std::collections::{HashMap, HashSet};
use thiserror::Error;

const MAX_ORGANISMS: usize = 100_000;
const POPULATION_SAMPLE_EVERY_DAYS: u64 = 10;
const MAX_POPULATION_SAMPLES: usize = 20_000;
const MAX_ALLELE_HISTORY_ROWS: usize = 100_000;
const MAX_ALLELE_SPECIES_PER_SAMPLE: usize = 32;
const INITIAL_ENERGY: u32 = 100;
const INITIAL_WATER: u32 = 30;

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("invalid world configuration: {0}")]
    InvalidConfig(String),
    #[error("command is not valid while world is {0:?}")]
    InvalidTransition(RunState),
    #[error("advance count must be between 1 and 100000")]
    InvalidAdvance,
    #[error("checkpoint state hash does not match its contents")]
    InvalidCheckpoint,
    #[error("world coordinates are outside the map")]
    OutOfBounds,
    #[error("world population limit reached")]
    PopulationLimit,
}

/// Versioned integer PRNG: SplitMix64. The state is checkpointed with the world.
#[derive(Clone, Copy, Debug)]
struct DeterministicRng {
    state: u64,
}
impl DeterministicRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }
    fn state(&self) -> u64 {
        self.state
    }
    fn next(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    fn below(&mut self, upper: u64) -> u64 {
        if upper <= 1 {
            return 0;
        }
        let threshold = upper.wrapping_neg() % upper;
        loop {
            let n = self.next();
            if n >= threshold {
                return n % upper;
            }
        }
    }
}

/// A standalone, deterministic world. All authoritative transitions are integer-only,
/// single-threaded, and independent of the server runtime and persistence layer.
pub struct World {
    id: WorldId,
    config: WorldConfig,
    state: RunState,
    tick: u64,
    width: u16,
    height: u16,
    tiles: Vec<Tile>,
    organisms: Vec<Organism>,
    statistics: WorldStatistics,
    population_history: Vec<PopulationSample>,
    allele_frequency_history: Vec<AlleleFrequencySample>,
    organism_lineage: Vec<OrganismLineage>,
    species_lineages: Vec<SpeciesLineage>,
    rng: DeterministicRng,
    next_organism_id: u64,
}

impl World {
    pub fn new(id: WorldId, config: WorldConfig) -> Result<Self, EngineError> {
        config.validate().map_err(EngineError::InvalidConfig)?;
        let (width, height) = config.size.dimensions();
        let mut rng = DeterministicRng::new(config.seed);
        let tiles = generate_terrain(width, height, &mut rng);
        let mut world = Self {
            id,
            config,
            state: RunState::Created,
            tick: 0,
            width,
            height,
            tiles,
            organisms: Vec::new(),
            statistics: WorldStatistics::default(),
            population_history: Vec::new(),
            allele_frequency_history: Vec::new(),
            organism_lineage: Vec::new(),
            species_lineages: Vec::new(),
            rng,
            next_organism_id: 1,
        };
        world.sample_population();
        world.sample_allele_frequencies();
        Ok(world)
    }

    pub fn apply(&mut self, command: SimulationCommand) -> Result<(), EngineError> {
        match command {
            SimulationCommand::Start if self.state == RunState::Created => {
                self.state = RunState::Running
            }
            SimulationCommand::Resume if self.state == RunState::Paused => {
                self.state = RunState::Running
            }
            SimulationCommand::Pause if self.state == RunState::Running => {
                self.state = RunState::Paused
            }
            SimulationCommand::SetSpeed { tick_duration_ms }
                if (10..=60_000).contains(&tick_duration_ms) =>
            {
                self.config.tick_duration_ms = tick_duration_ms
            }
            SimulationCommand::SetSpeed { .. } => {
                return Err(EngineError::InvalidConfig(
                    "tick_duration_ms must be 10..=60000".into(),
                ))
            }
            SimulationCommand::IntroduceOrganism { x, y, genome } => {
                self.introduce(x, y, genome)?
            }
            SimulationCommand::Advance { ticks }
                if self.state == RunState::Running && (1..=100_000).contains(&ticks) =>
            {
                if self.tick.checked_add(ticks).is_none() {
                    return Err(EngineError::InvalidAdvance);
                }
                for _ in 0..ticks {
                    self.advance_one_day();
                }
            }
            SimulationCommand::Advance { .. } => return Err(EngineError::InvalidAdvance),
            _ => return Err(EngineError::InvalidTransition(self.state)),
        }
        Ok(())
    }

    fn introduce(&mut self, x: u16, y: u16, genome: Genome) -> Result<(), EngineError> {
        if x >= self.width || y >= self.height {
            return Err(EngineError::OutOfBounds);
        }
        if self.organisms.len() >= MAX_ORGANISMS {
            return Err(EngineError::PopulationLimit);
        }
        genome.validate().map_err(EngineError::InvalidConfig)?;
        let id = self.next_organism_id;
        self.next_organism_id += 1;
        // Sex is not a species trait: matching founders can form a breeding population.
        let mut signature = genome.clone();
        signature.sex = Sex::Asexual;
        let species_id = self
            .organisms
            .iter()
            .find(|organism| {
                let mut candidate = organism.genome.clone();
                candidate.sex = Sex::Asexual;
                candidate == signature
            })
            .map_or(id, |organism| organism.species_id);
        self.organisms.push(Organism {
            id,
            species_id,
            parent_ids: Vec::new(),
            x,
            y,
            age_days: 0,
            energy: INITIAL_ENERGY,
            water_reserve: INITIAL_WATER,
            genome,
        });
        let founder_genome = self.organisms.last().expect("just inserted").genome.clone();
        self.register_founder(id, species_id, founder_genome);
        if self.tick.is_multiple_of(POPULATION_SAMPLE_EVERY_DAYS) {
            self.sample_population();
            self.sample_allele_frequencies();
        }
        Ok(())
    }

    /// Add deterministic, uniformly placed founders for controlled experiments and benchmarks.
    pub fn seed_population(&mut self, count: usize, genome: Genome) -> Result<(), EngineError> {
        genome.validate().map_err(EngineError::InvalidConfig)?;
        if count > MAX_ORGANISMS.saturating_sub(self.organisms.len()) {
            return Err(EngineError::PopulationLimit);
        }
        if self.state != RunState::Created || self.tick != 0 {
            return Err(EngineError::InvalidTransition(self.state));
        }
        let species_id = self.next_organism_id;
        for _ in 0..count {
            let id = self.next_organism_id;
            self.next_organism_id += 1;
            self.organisms.push(Organism {
                id,
                species_id,
                parent_ids: Vec::new(),
                x: self.rng.below(u64::from(self.width)) as u16,
                y: self.rng.below(u64::from(self.height)) as u16,
                age_days: 0,
                energy: INITIAL_ENERGY,
                water_reserve: INITIAL_WATER,
                genome: genome.clone(),
            });
            self.register_founder(id, species_id, genome.clone());
        }
        self.sample_population();
        self.sample_allele_frequencies();
        Ok(())
    }

    fn advance_one_day(&mut self) {
        self.tick += 1;
        self.regenerate_resources();
        self.move_and_metabolize();
        self.feed_herbivores();
        self.resolve_predation();
        self.resolve_deaths();
        self.reproduce();
        if self.tick.is_multiple_of(POPULATION_SAMPLE_EVERY_DAYS) {
            self.sample_population();
            self.sample_allele_frequencies();
        }
    }

    fn register_founder(&mut self, id: u64, species_id: u64, genome: Genome) {
        if !self
            .species_lineages
            .iter()
            .any(|lineage| lineage.species_id == species_id)
        {
            self.species_lineages.push(SpeciesLineage {
                species_id,
                parent_species_id: None,
                founder_organism_id: id,
                founded_day: self.tick,
                extinct_day: None,
            });
        }
        self.organism_lineage.push(OrganismLineage {
            organism_id: id,
            parent_ids: Vec::new(),
            species_id,
            born_day: self.tick,
            died_day: None,
            genome_at_birth: genome,
        });
    }

    fn sample_population(&mut self) {
        let mut sample = PopulationSample {
            day: self.tick,
            births: self.statistics.births,
            deaths: self.statistics.deaths,
            ..PopulationSample::default()
        };
        let mut speed = 0_u64;
        let mut metabolism = 0_u64;
        let mut fertility = 0_u64;
        for organism in &self.organisms {
            sample.population += 1;
            match organism.genome.diet {
                Diet::Herbivore => sample.herbivores += 1,
                Diet::Carnivore => sample.carnivores += 1,
                Diet::Omnivore => sample.omnivores += 1,
            }
            speed += u64::from(organism.genome.speed.expressed());
            metabolism += u64::from(organism.genome.metabolism.expressed());
            fertility += u64::from(organism.genome.fertility.expressed());
        }
        if sample.population > 0 {
            let count = u64::from(sample.population);
            sample.average_speed = (speed / count) as u8;
            sample.average_metabolism = (metabolism / count) as u8;
            sample.average_fertility = (fertility / count) as u8;
        }
        if self
            .population_history
            .last()
            .is_some_and(|last| last.day == sample.day)
        {
            self.population_history.pop();
        }
        self.population_history.push(sample);
        if self.population_history.len() > MAX_POPULATION_SAMPLES {
            self.population_history.drain(..MAX_POPULATION_SAMPLES / 10);
        }
    }

    fn sample_allele_frequencies(&mut self) {
        let mut by_species: HashMap<u64, AlleleFrequencySample> = HashMap::new();
        for organism in &self.organisms {
            if !by_species.contains_key(&organism.species_id)
                && by_species.len() >= MAX_ALLELE_SPECIES_PER_SAMPLE
            {
                continue;
            }
            let sample =
                by_species
                    .entry(organism.species_id)
                    .or_insert_with(|| AlleleFrequencySample {
                        day: self.tick,
                        species_id: organism.species_id,
                        population: 0,
                        speed: vec![0; 5],
                        metabolism: vec![0; 21],
                        fertility: vec![0; 101],
                    });
            sample.population += 1;
            sample.speed[usize::from(organism.genome.speed.first)] += 1;
            sample.speed[usize::from(organism.genome.speed.second)] += 1;
            sample.metabolism[usize::from(organism.genome.metabolism.first)] += 1;
            sample.metabolism[usize::from(organism.genome.metabolism.second)] += 1;
            sample.fertility[usize::from(organism.genome.fertility.first)] += 1;
            sample.fertility[usize::from(organism.genome.fertility.second)] += 1;
        }
        self.allele_frequency_history
            .retain(|sample| sample.day != self.tick);
        let mut samples: Vec<_> = by_species.into_values().collect();
        samples.sort_by_key(|sample| sample.species_id);
        self.allele_frequency_history.extend(samples);
        if self.allele_frequency_history.len() > MAX_ALLELE_HISTORY_ROWS {
            let oldest_day = self
                .allele_frequency_history
                .first()
                .map(|sample| sample.day)
                .unwrap_or(0);
            self.allele_frequency_history
                .retain(|sample| sample.day != oldest_day);
        }
    }

    fn regenerate_resources(&mut self) {
        for tile in &mut self.tiles {
            let growth = 1 + u32::from(tile.moisture / 32);
            tile.resources.vegetation = (tile.resources.vegetation + growth).min(1000);
            if self.tick.is_multiple_of(7) && tile.terrain != TerrainType::Desert {
                tile.resources.water = tile
                    .resources
                    .water
                    .saturating_add(u32::from(tile.moisture / 8))
                    .min(1000);
            }
            if matches!(
                tile.terrain,
                TerrainType::Ocean | TerrainType::Lake | TerrainType::River
            ) {
                tile.resources.water = 1000;
            }
        }
    }

    fn move_and_metabolize(&mut self) {
        self.organisms.sort_by_key(|o| o.id);
        for organism in &mut self.organisms {
            organism.age_days = organism.age_days.saturating_add(1);
            organism.energy = organism
                .energy
                .saturating_sub(u32::from(organism.genome.metabolism.expressed().max(1)));
            organism.water_reserve = organism.water_reserve.saturating_sub(1);
            let speed = organism.genome.speed.expressed().min(4);
            for _ in 0..speed {
                let direction = self.rng.below(8);
                let (dx, dy) = match direction {
                    0 => (-1_i32, -1_i32),
                    1 => (0, -1),
                    2 => (1, -1),
                    3 => (-1, 0),
                    4 => (1, 0),
                    5 => (-1, 1),
                    6 => (0, 1),
                    _ => (1, 1),
                };
                organism.x = (i32::from(organism.x) + dx).rem_euclid(i32::from(self.width)) as u16;
                organism.y = (i32::from(organism.y) + dy).rem_euclid(i32::from(self.height)) as u16;
            }
            let index = tile_index(self.width, organism.x, organism.y);
            if self.tiles[index].resources.water > 0 && organism.water_reserve < INITIAL_WATER {
                self.tiles[index].resources.water -= 1;
                organism.water_reserve = (organism.water_reserve + 4).min(INITIAL_WATER);
            }
        }
    }

    fn feed_herbivores(&mut self) {
        for organism in &mut self.organisms {
            if matches!(organism.genome.diet, Diet::Herbivore | Diet::Omnivore)
                && organism.energy < INITIAL_ENERGY
            {
                let index = tile_index(self.width, organism.x, organism.y);
                let eaten = self.tiles[index].resources.vegetation.min(40);
                self.tiles[index].resources.vegetation -= eaten;
                organism.energy = (organism.energy + eaten).min(INITIAL_ENERGY);
            }
        }
    }

    fn resolve_predation(&mut self) {
        let mut prey_by_tile: HashMap<(u16, u16), Vec<u64>> = HashMap::new();
        let mut predator_indices = Vec::new();
        for (index, organism) in self.organisms.iter().enumerate() {
            match organism.genome.diet {
                Diet::Herbivore | Diet::Omnivore => prey_by_tile
                    .entry((organism.x, organism.y))
                    .or_default()
                    .push(organism.id),
                Diet::Carnivore if organism.energy < INITIAL_ENERGY => predator_indices.push(index),
                Diet::Carnivore => {}
            }
        }
        let mut killed = HashSet::new();
        let mut eaten_by_tile: HashMap<(u16, u16), usize> = HashMap::new();
        for predator_index in predator_indices {
            let predator = &self.organisms[predator_index];
            let key = (predator.x, predator.y);
            let used = eaten_by_tile.entry(key).or_default();
            let prey = prey_by_tile
                .get(&key)
                .and_then(|ids| ids.iter().skip(*used).find(|id| **id != predator.id))
                .copied();
            if let Some(prey_id) = prey {
                *used += 1;
                killed.insert(prey_id);
                self.organisms[predator_index].energy =
                    (self.organisms[predator_index].energy + 50).min(INITIAL_ENERGY);
            }
        }
        if !killed.is_empty() {
            for organism_id in &killed {
                self.record_death(*organism_id);
            }
            self.organisms.retain(|o| !killed.contains(&o.id));
            let deaths = killed.len() as u64;
            self.statistics.deaths += deaths;
            self.statistics.predation_deaths += deaths;
        }
    }

    fn resolve_deaths(&mut self) {
        let dead_ids: Vec<_> = self
            .organisms
            .iter()
            .filter(|o| {
                o.energy == 0 || o.water_reserve == 0 || o.age_days >= o.genome.lifespan_days
            })
            .map(|o| o.id)
            .collect();
        for organism_id in dead_ids {
            self.record_death(organism_id);
        }
        let old_len = self.organisms.len();
        self.organisms
            .retain(|o| o.energy > 0 && o.water_reserve > 0 && o.age_days < o.genome.lifespan_days);
        self.statistics.deaths += (old_len - self.organisms.len()) as u64;
        let living_species: HashSet<_> = self.organisms.iter().map(|o| o.species_id).collect();
        for lineage in &mut self.species_lineages {
            if lineage.extinct_day.is_none() && !living_species.contains(&lineage.species_id) {
                lineage.extinct_day = Some(self.tick);
            }
        }
    }

    fn record_death(&mut self, organism_id: u64) {
        if let Some(record) = organism_id
            .checked_sub(1)
            .and_then(|index| self.organism_lineage.get_mut(index as usize))
        {
            record.died_day = Some(self.tick);
        }
    }

    fn reproduce(&mut self) {
        if self.organisms.len() >= MAX_ORGANISMS {
            return;
        }
        let males_by_location: HashMap<(u64, u16, u16), Vec<Organism>> = self
            .organisms
            .iter()
            .filter(|o| o.genome.sex == Sex::Male && o.age_days >= 30 && o.energy >= 75)
            .fold(HashMap::new(), |mut by_location, organism| {
                by_location
                    .entry((organism.species_id, organism.x, organism.y))
                    .or_default()
                    .push(organism.clone());
                by_location
            });
        let mut newborns = Vec::new();
        let mut paired = HashSet::new();
        for i in 0..self.organisms.len() {
            if self.organisms.len() + newborns.len() >= MAX_ORGANISMS {
                break;
            }
            let parent = &self.organisms[i];
            if parent.age_days < 30
                || parent.energy < 75
                || parent.genome.fertility.expressed() == 0
            {
                continue;
            }
            if self.rng.below(100) >= u64::from(parent.genome.fertility.expressed()) {
                continue;
            }
            let other = match parent.genome.sex {
                Sex::Asexual => None,
                Sex::Female => males_by_location
                    .get(&(parent.species_id, parent.x, parent.y))
                    .and_then(|males| males.iter().find(|male| !paired.contains(&male.id)))
                    .cloned(),
                Sex::Male => continue,
            };
            if parent.genome.sex != Sex::Asexual && other.is_none() {
                continue;
            }
            if let Some(ref mate) = other {
                paired.insert(mate.id);
            }
            let mut genome = if let Some(ref mate) = other {
                inherit(&parent.genome, &mate.genome, &mut self.rng)
            } else {
                parent.genome.clone()
            };
            mutate(&mut genome, &mut self.rng);
            let id = self.next_organism_id;
            self.next_organism_id += 1;
            let mut child = parent.clone();
            child.id = id;
            child.genome = genome;
            child.parent_ids = if let Some(ref mate) = other {
                vec![parent.id, mate.id]
            } else {
                vec![parent.id]
            };
            child.age_days = 0;
            child.energy = 50;
            child.water_reserve = INITIAL_WATER;
            self.organisms[i].energy -= 25;
            self.organism_lineage.push(OrganismLineage {
                organism_id: id,
                parent_ids: child.parent_ids.clone(),
                species_id: child.species_id,
                born_day: self.tick,
                died_day: None,
                genome_at_birth: child.genome.clone(),
            });
            newborns.push(child);
        }
        self.statistics.births += newborns.len() as u64;
        self.organisms.extend(newborns);
    }

    pub fn snapshot(&self) -> WorldSnapshot {
        let state_hash = self.state_hash();
        WorldSnapshot {
            id: self.id,
            config: self.config.clone(),
            state: self.state,
            tick: self.tick,
            width: self.width,
            height: self.height,
            tiles: self.tiles.clone(),
            organisms: self.organisms.clone(),
            statistics: self.statistics.clone(),
            population_history: self.population_history.clone(),
            allele_frequency_history: self.allele_frequency_history.clone(),
            organism_lineage: self.organism_lineage.clone(),
            species_lineages: self.species_lineages.clone(),
            rng_state: self.rng.state(),
            next_organism_id: self.next_organism_id,
            state_hash,
        }
    }

    pub fn update(&self) -> WorldUpdate {
        WorldUpdate {
            id: self.id,
            config: self.config.clone(),
            state: self.state,
            tick: self.tick,
            organisms: self.organisms.clone(),
            statistics: self.statistics.clone(),
            population_sample: self
                .population_history
                .last()
                .filter(|sample| sample.day == self.tick)
                .cloned(),
            allele_frequency_samples: self
                .allele_frequency_history
                .iter()
                .filter(|sample| sample.day == self.tick)
                .cloned()
                .collect(),
            species_lineages: self.species_lineages.clone(),
            state_hash: self.state_hash(),
        }
    }

    pub fn summary(&self) -> WorldSummary {
        WorldSummary {
            id: self.id,
            config: self.config.clone(),
            state: self.state,
            tick: self.tick,
            width: self.width,
            height: self.height,
            population: self.organisms.len() as u32,
            statistics: self.statistics.clone(),
            state_hash: self.state_hash(),
        }
    }

    fn state_hash(&self) -> String {
        #[derive(Serialize)]
        struct Canonical<'a> {
            tick: u64,
            width: u16,
            height: u16,
            tiles: &'a [Tile],
            organisms: &'a [Organism],
            organism_lineage: &'a [OrganismLineage],
            species_lineages: &'a [SpeciesLineage],
            statistics: &'a WorldStatistics,
            rng_state: u64,
            next_organism_id: u64,
        }
        let state = Canonical {
            tick: self.tick,
            width: self.width,
            height: self.height,
            tiles: &self.tiles,
            organisms: &self.organisms,
            organism_lineage: &self.organism_lineage,
            species_lineages: &self.species_lineages,
            statistics: &self.statistics,
            rng_state: self.rng.state(),
            next_organism_id: self.next_organism_id,
        };
        let bytes = serde_json::to_vec(&state).expect("canonical state serializes");
        let mut hasher = Hasher::new();
        // Population history is derived telemetry, not authoritative simulation state.
        hasher.update(b"simlife-world-state-v3\0");
        hasher.update(&bytes);
        hasher.finalize().to_hex().to_string()
    }

    pub fn restore(snapshot: &WorldSnapshot) -> Result<Self, EngineError> {
        snapshot
            .config
            .validate()
            .map_err(EngineError::InvalidConfig)?;
        let (expected_width, expected_height) = snapshot.config.size.dimensions();
        if snapshot.width != expected_width
            || snapshot.height != expected_height
            || snapshot.tiles.len() != usize::from(snapshot.width) * usize::from(snapshot.height)
        {
            return Err(EngineError::InvalidCheckpoint);
        }
        if snapshot.organisms.iter().any(|o| {
            o.x >= snapshot.width || o.y >= snapshot.height || o.genome.validate().is_err()
        }) {
            return Err(EngineError::InvalidCheckpoint);
        }
        if snapshot.organism_lineage.len() as u64 + 1 != snapshot.next_organism_id
            || snapshot
                .organism_lineage
                .iter()
                .enumerate()
                .any(|(index, record)| {
                    record.organism_id != index as u64 + 1
                        || record
                            .parent_ids
                            .iter()
                            .any(|parent| *parent >= record.organism_id)
                        || (record.parent_ids.len() > 2)
                })
            || snapshot.organism_lineage.iter().any(|record| {
                record.organism_id == 0
                    || record.organism_id >= snapshot.next_organism_id
                    || record.born_day > snapshot.tick
                    || record
                        .died_day
                        .is_some_and(|day| day < record.born_day || day > snapshot.tick)
                    || record
                        .parent_ids
                        .iter()
                        .any(|parent| *parent >= record.organism_id)
                    || record.genome_at_birth.validate().is_err()
            })
            || snapshot.organisms.iter().any(|organism| {
                !snapshot
                    .organism_lineage
                    .iter()
                    .any(|record| record.organism_id == organism.id && record.died_day.is_none())
            })
            || snapshot.species_lineages.iter().any(|lineage| {
                lineage.founded_day > snapshot.tick
                    || lineage
                        .extinct_day
                        .is_some_and(|day| day < lineage.founded_day || day > snapshot.tick)
            })
        {
            return Err(EngineError::InvalidCheckpoint);
        }
        if snapshot.allele_frequency_history.len() > MAX_ALLELE_HISTORY_ROWS
            || snapshot.allele_frequency_history.iter().any(|sample| {
                sample.day > snapshot.tick
                    || sample.population == 0
                    || sample.speed.len() != 5
                    || sample.metabolism.len() != 21
                    || sample.fertility.len() != 101
                    || sample.speed.iter().map(|n| u64::from(*n)).sum::<u64>()
                        != u64::from(sample.population) * 2
                    || sample.metabolism.iter().map(|n| u64::from(*n)).sum::<u64>()
                        != u64::from(sample.population) * 2
                    || sample.fertility.iter().map(|n| u64::from(*n)).sum::<u64>()
                        != u64::from(sample.population) * 2
            })
        {
            return Err(EngineError::InvalidCheckpoint);
        }
        if snapshot.population_history.len() > MAX_POPULATION_SAMPLES
            || snapshot.population_history.iter().any(|sample| {
                sample.day > snapshot.tick
                    || !sample.day.is_multiple_of(POPULATION_SAMPLE_EVERY_DAYS)
                    || sample.population as usize > MAX_ORGANISMS
                    || u64::from(sample.herbivores)
                        + u64::from(sample.carnivores)
                        + u64::from(sample.omnivores)
                        != u64::from(sample.population)
                    || sample.average_speed > 4
                    || sample.average_metabolism > 20
                    || sample.average_fertility > 100
            })
            || snapshot
                .population_history
                .windows(2)
                .any(|pair| pair[0].day >= pair[1].day)
        {
            return Err(EngineError::InvalidCheckpoint);
        }
        let world = Self {
            id: snapshot.id,
            config: snapshot.config.clone(),
            state: snapshot.state,
            tick: snapshot.tick,
            width: snapshot.width,
            height: snapshot.height,
            tiles: snapshot.tiles.clone(),
            organisms: snapshot.organisms.clone(),
            statistics: snapshot.statistics.clone(),
            population_history: snapshot.population_history.clone(),
            allele_frequency_history: snapshot.allele_frequency_history.clone(),
            organism_lineage: snapshot.organism_lineage.clone(),
            species_lineages: snapshot.species_lineages.clone(),
            rng: DeterministicRng {
                state: snapshot.rng_state,
            },
            next_organism_id: snapshot.next_organism_id,
        };
        if world.state_hash() != snapshot.state_hash {
            return Err(EngineError::InvalidCheckpoint);
        }
        Ok(world)
    }

    pub fn day(&self) -> u64 {
        self.tick
    }
    pub fn tick_duration_ms(&self) -> u32 {
        self.config.tick_duration_ms
    }
    pub fn is_running(&self) -> bool {
        self.state == RunState::Running
    }

    pub fn inspect_organism(&self, id: u64, max_ancestors: usize) -> Option<OrganismInspection> {
        let organism = self
            .organisms
            .iter()
            .find(|organism| organism.id == id)?
            .clone();
        let lineage = self
            .organism_lineage
            .iter()
            .find(|record| record.organism_id == id)?
            .clone();
        let mut ancestry = Vec::new();
        let mut parent = lineage.parent_ids.first().copied();
        while let Some(parent_id) = parent.filter(|_| ancestry.len() < max_ancestors) {
            let Some(record) = self
                .organism_lineage
                .iter()
                .find(|r| r.organism_id == parent_id)
            else {
                break;
            };
            ancestry.push(record.clone());
            parent = record.parent_ids.first().copied();
        }
        Some(OrganismInspection {
            organism,
            lineage,
            offspring_ids: self
                .organism_lineage
                .iter()
                .filter(|record| record.parent_ids.contains(&id))
                .map(|record| record.organism_id)
                .collect(),
            ancestry,
        })
    }
}

fn tile_index(width: u16, x: u16, y: u16) -> usize {
    usize::from(y) * usize::from(width) + usize::from(x)
}
fn generate_terrain(width: u16, height: u16, rng: &mut DeterministicRng) -> Vec<Tile> {
    let len = usize::from(width) * usize::from(height);
    let mut elevations: Vec<i16> = (0..len).map(|_| (rng.below(201) as i16) - 100).collect();
    let moisture: Vec<u8> = (0..len).map(|_| rng.below(101) as u8).collect();
    for _ in 0..2 {
        let mut smoothed = elevations.clone();
        for y in 0..height {
            for x in 0..width {
                let idx = tile_index(width, x, y);
                let left = tile_index(width, (x + width - 1) % width, y);
                let right = tile_index(width, (x + 1) % width, y);
                let up = tile_index(width, x, (y + height - 1) % height);
                let down = tile_index(width, x, (y + 1) % height);
                smoothed[idx] = ((i32::from(elevations[idx]) * 2
                    + i32::from(elevations[left])
                    + i32::from(elevations[right])
                    + i32::from(elevations[up])
                    + i32::from(elevations[down]))
                    / 6) as i16;
            }
        }
        elevations = smoothed;
    }
    elevations
        .into_iter()
        .zip(moisture)
        .map(|(elevation, moisture)| {
            let terrain = if elevation < -60 {
                TerrainType::Ocean
            } else if elevation < -44 && rng.below(5) == 0 {
                TerrainType::Lake
            } else if elevation < -30 && rng.below(8) == 0 {
                TerrainType::River
            } else if elevation > 70 {
                TerrainType::Mountain
            } else if moisture < 22 {
                TerrainType::Desert
            } else if moisture > 62 {
                TerrainType::Forest
            } else {
                TerrainType::Plains
            };
            let (water, vegetation) = match terrain {
                TerrainType::Ocean | TerrainType::Lake | TerrainType::River => (1000, 0),
                TerrainType::Forest => (u32::from(moisture) * 3, 700),
                TerrainType::Plains => (u32::from(moisture) * 2, 450),
                TerrainType::Desert => (u32::from(moisture) / 2, 80),
                TerrainType::Mountain => (u32::from(moisture), 30),
            };
            Tile {
                elevation,
                moisture,
                terrain,
                resources: Resources {
                    water,
                    vegetation,
                    minerals: 100 + rng.below(901) as u32,
                },
            }
        })
        .collect()
}

fn inherit(a: &Genome, b: &Genome, rng: &mut DeterministicRng) -> Genome {
    fn pair(a: Alleles, b: Alleles, rng: &mut DeterministicRng) -> Alleles {
        let av = if rng.below(2) == 0 { a.first } else { a.second };
        let bv = if rng.below(2) == 0 { b.first } else { b.second };
        Alleles {
            first: av,
            second: bv,
        }
    }
    Genome {
        speed: pair(a.speed, b.speed, rng),
        metabolism: pair(a.metabolism, b.metabolism, rng),
        fertility: pair(a.fertility, b.fertility, rng),
        lifespan_days: if rng.below(2) == 0 {
            a.lifespan_days
        } else {
            b.lifespan_days
        },
        diet: if rng.below(2) == 0 { a.diet } else { b.diet },
        sex: if rng.below(2) == 0 {
            Sex::Female
        } else {
            Sex::Male
        },
    }
}
fn mutate(genome: &mut Genome, rng: &mut DeterministicRng) {
    fn mutate_pair(pair: &mut Alleles, max: u8, rng: &mut DeterministicRng) {
        for allele in [&mut pair.first, &mut pair.second] {
            if rng.below(100) == 0 {
                if *allele == 0 {
                    *allele = 1;
                } else if *allele >= max || rng.below(2) == 0 {
                    *allele -= 1;
                } else {
                    *allele += 1;
                }
            }
        }
    }
    mutate_pair(&mut genome.speed, 4, rng);
    mutate_pair(&mut genome.metabolism, 20, rng);
    mutate_pair(&mut genome.fertility, 100, rng);
}

#[cfg(test)]
mod tests {
    use super::*;
    fn config(seed: u64) -> WorldConfig {
        WorldConfig {
            name: "test".into(),
            seed,
            size: simlife_domain::WorldSize::Small,
            tick_duration_ms: 1000,
        }
    }
    #[test]
    fn identical_seeds_match_even_with_distinct_world_ids() {
        let a = World::new(WorldId::new(), config(9)).unwrap();
        let mut b = World::new(WorldId::new(), config(9)).unwrap();
        let mut a = a;
        a.apply(SimulationCommand::Start).unwrap();
        b.apply(SimulationCommand::Start).unwrap();
        a.apply(SimulationCommand::Advance { ticks: 100 }).unwrap();
        b.apply(SimulationCommand::Advance { ticks: 100 }).unwrap();
        assert_eq!(a.snapshot().state_hash, b.snapshot().state_hash);
    }
    #[test]
    fn terrain_is_nonempty_and_deterministic() {
        let a = World::new(WorldId::new(), config(9)).unwrap();
        let b = World::new(WorldId::new(), config(9)).unwrap();
        let sa = a.snapshot();
        let sb = b.snapshot();
        assert_eq!(sa.tiles, sb.tiles);
        assert_eq!(sa.tiles.len(), 32 * 64);
        assert!(sa.tiles.iter().any(|t| t.terrain == TerrainType::Ocean));
        assert!(sa.tiles.iter().any(|t| t.resources.vegetation > 0));
    }
    #[test]
    fn world_clocks_are_independent_and_transitions_checked() {
        let mut a = World::new(WorldId::new(), config(1)).unwrap();
        let b = World::new(WorldId::new(), config(1)).unwrap();
        assert!(a.apply(SimulationCommand::Pause).is_err());
        a.apply(SimulationCommand::Start).unwrap();
        a.apply(SimulationCommand::Advance { ticks: 2 }).unwrap();
        assert_eq!(a.day(), 2);
        assert_eq!(b.day(), 0);
    }
    #[test]
    fn checkpoints_restore_random_stream_and_state() {
        let mut original = World::new(WorldId::new(), config(77)).unwrap();
        original
            .apply(SimulationCommand::IntroduceOrganism {
                x: 2,
                y: 3,
                genome: Genome::default(),
            })
            .unwrap();
        original.apply(SimulationCommand::Start).unwrap();
        original
            .apply(SimulationCommand::Advance { ticks: 10 })
            .unwrap();
        let mut restored = World::restore(&original.snapshot()).unwrap();
        original
            .apply(SimulationCommand::Advance { ticks: 50 })
            .unwrap();
        restored
            .apply(SimulationCommand::Advance { ticks: 50 })
            .unwrap();
        assert_eq!(
            original.snapshot().state_hash,
            restored.snapshot().state_hash
        );
    }
    #[test]
    fn coordinates_wrap_on_both_axes() {
        let mut world = World::new(WorldId::new(), config(41)).unwrap();
        let genome = Genome {
            speed: Alleles {
                first: 4,
                second: 4,
            },
            ..Genome::default()
        };
        world
            .apply(SimulationCommand::IntroduceOrganism { x: 0, y: 0, genome })
            .unwrap();
        world.apply(SimulationCommand::Start).unwrap();
        world
            .apply(SimulationCommand::Advance { ticks: 1 })
            .unwrap();
        let organism = &world.snapshot().organisms[0];
        assert!(organism.x < 32 && organism.y < 64);
    }

    #[test]
    fn organisms_age_and_die_at_genetic_lifespan() {
        let mut world = World::new(WorldId::new(), config(12)).unwrap();
        let genome = Genome {
            lifespan_days: 1,
            ..Genome::default()
        };
        world
            .apply(SimulationCommand::IntroduceOrganism { x: 0, y: 0, genome })
            .unwrap();
        world.apply(SimulationCommand::Start).unwrap();
        world
            .apply(SimulationCommand::Advance { ticks: 1 })
            .unwrap();
        assert!(world.organisms.is_empty());
        assert_eq!(world.statistics.deaths, 1);
        assert_eq!(world.organism_lineage[0].died_day, Some(1));
    }

    #[test]
    fn predators_consume_colocated_prey_and_account_for_death() {
        let mut world = World::new(WorldId::new(), config(39)).unwrap();
        for tile in &mut world.tiles {
            tile.resources.vegetation = 1000;
        }
        let stationary = Alleles {
            first: 0,
            second: 0,
        };
        let predator = Genome {
            speed: stationary,
            diet: Diet::Carnivore,
            ..Genome::default()
        };
        let prey = Genome {
            speed: stationary,
            diet: Diet::Herbivore,
            ..Genome::default()
        };
        world
            .apply(SimulationCommand::IntroduceOrganism {
                x: 3,
                y: 4,
                genome: predator,
            })
            .unwrap();
        world
            .apply(SimulationCommand::IntroduceOrganism {
                x: 3,
                y: 4,
                genome: prey,
            })
            .unwrap();
        world.apply(SimulationCommand::Start).unwrap();
        world
            .apply(SimulationCommand::Advance { ticks: 1 })
            .unwrap();
        assert_eq!(world.organisms.len(), 1);
        assert_eq!(world.statistics.predation_deaths, 1);
    }

    #[test]
    fn eligible_asexual_population_reproduces_without_commands() {
        let mut world = World::new(WorldId::new(), config(456)).unwrap();
        for tile in &mut world.tiles {
            tile.resources.vegetation = 1000;
            tile.resources.water = 1000;
        }
        let genome = Genome {
            speed: Alleles {
                first: 0,
                second: 0,
            },
            metabolism: Alleles {
                first: 1,
                second: 1,
            },
            fertility: Alleles {
                first: 100,
                second: 100,
            },
            ..Genome::default()
        };
        world
            .apply(SimulationCommand::IntroduceOrganism { x: 5, y: 5, genome })
            .unwrap();
        world.apply(SimulationCommand::Start).unwrap();
        world
            .apply(SimulationCommand::Advance { ticks: 30 })
            .unwrap();
        assert!(world.statistics.births >= 1);
        assert!(world.organisms.len() > 1);
        let child = world
            .organisms
            .iter()
            .find(|organism| organism.id > 1)
            .unwrap();
        assert_eq!(child.parent_ids.len(), 1);
        let inspection = world.inspect_organism(child.id, 10).unwrap();
        assert_eq!(inspection.ancestry.len(), 1);
        assert_eq!(inspection.ancestry[0].organism_id, child.parent_ids[0]);
        let parent = world.inspect_organism(child.parent_ids[0], 10).unwrap();
        assert!(parent.offspring_ids.contains(&child.id));
        let recovered = World::restore(&world.snapshot()).unwrap();
        assert_eq!(
            recovered.snapshot().organism_lineage,
            world.snapshot().organism_lineage
        );
    }

    #[test]
    fn compatible_opposite_sex_founders_reproduce() {
        let mut world = World::new(WorldId::new(), config(789)).unwrap();
        for tile in &mut world.tiles {
            tile.resources.vegetation = 1000;
            tile.resources.water = 1000;
        }
        let base = Genome {
            speed: Alleles {
                first: 0,
                second: 0,
            },
            metabolism: Alleles {
                first: 1,
                second: 1,
            },
            fertility: Alleles {
                first: 100,
                second: 100,
            },
            ..Genome::default()
        };
        let female = Genome {
            sex: Sex::Female,
            ..base.clone()
        };
        let male = Genome {
            sex: Sex::Male,
            ..base
        };
        world
            .apply(SimulationCommand::IntroduceOrganism {
                x: 4,
                y: 5,
                genome: female,
            })
            .unwrap();
        world
            .apply(SimulationCommand::IntroduceOrganism {
                x: 4,
                y: 5,
                genome: male,
            })
            .unwrap();
        world.apply(SimulationCommand::Start).unwrap();
        world
            .apply(SimulationCommand::Advance { ticks: 30 })
            .unwrap();
        assert!(world.statistics.births > 0);
    }

    #[test]
    fn sampled_population_history_is_deterministic_and_tracks_traits() {
        let mut first = World::new(WorldId::new(), config(9_901)).unwrap();
        let mut second = World::new(WorldId::new(), config(9_901)).unwrap();
        let genome = Genome {
            speed: Alleles {
                first: 3,
                second: 3,
            },
            fertility: Alleles {
                first: 0,
                second: 0,
            },
            ..Genome::default()
        };
        first.seed_population(100, genome.clone()).unwrap();
        second.seed_population(100, genome).unwrap();
        assert_eq!(first.snapshot().state_hash, second.snapshot().state_hash);
        first.apply(SimulationCommand::Start).unwrap();
        second.apply(SimulationCommand::Start).unwrap();
        first
            .apply(SimulationCommand::Advance { ticks: 10 })
            .unwrap();
        second
            .apply(SimulationCommand::Advance { ticks: 10 })
            .unwrap();
        let snapshot = first.snapshot();
        assert_eq!(snapshot.state_hash, second.snapshot().state_hash);
        assert_eq!(snapshot.population_history.len(), 2);
        let sample = snapshot.population_history.last().unwrap();
        assert_eq!(sample.day, 10);
        assert_eq!(sample.population as usize, snapshot.organisms.len());
        assert_eq!(sample.average_speed, 3);
        let allele_sample = snapshot
            .allele_frequency_history
            .iter()
            .find(|sample| sample.day == 10)
            .unwrap();
        assert_eq!(
            allele_sample.population as usize,
            snapshot
                .organisms
                .iter()
                .filter(|o| o.species_id == allele_sample.species_id)
                .count()
        );
        assert_eq!(allele_sample.speed[3], allele_sample.population * 2);
        assert_eq!(snapshot.species_lineages.len(), 1);
        assert_eq!(snapshot.organism_lineage.len(), 100);
    }

    #[test]
    fn checkpoint_preserves_ancestry_and_allele_frequencies() {
        let mut world = World::new(WorldId::new(), config(9801)).unwrap();
        world.seed_population(8, Genome::default()).unwrap();
        world.apply(SimulationCommand::Start).unwrap();
        world
            .apply(SimulationCommand::Advance { ticks: 10 })
            .unwrap();
        let snapshot = world.snapshot();
        let restored = World::restore(&snapshot).unwrap();
        assert_eq!(restored.snapshot().state_hash, snapshot.state_hash);
        assert_eq!(
            restored.snapshot().organism_lineage,
            snapshot.organism_lineage
        );
        assert_eq!(
            restored.snapshot().allele_frequency_history,
            snapshot.allele_frequency_history
        );
    }

    #[test]
    fn reproduction_and_lineage_are_deterministic_for_identical_seeds() {
        let mut first = World::new(WorldId::new(), config(9802)).unwrap();
        let mut second = World::new(WorldId::new(), config(9802)).unwrap();
        let genome = Genome {
            speed: Alleles {
                first: 0,
                second: 0,
            },
            fertility: Alleles {
                first: 100,
                second: 100,
            },
            ..Genome::default()
        };
        for world in [&mut first, &mut second] {
            world.seed_population(24, genome.clone()).unwrap();
            world.apply(SimulationCommand::Start).unwrap();
            world
                .apply(SimulationCommand::Advance { ticks: 60 })
                .unwrap();
        }
        assert!(first.statistics.births > 0);
        assert_eq!(first.snapshot().state_hash, second.snapshot().state_hash);
        assert_eq!(
            first.snapshot().organism_lineage,
            second.snapshot().organism_lineage
        );
    }
}
