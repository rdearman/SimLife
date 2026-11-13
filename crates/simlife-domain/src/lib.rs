use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WorldId(pub Uuid);

impl WorldId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}
impl Default for WorldId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorldSize {
    #[default]
    Small,
    Medium,
    Large,
}

impl WorldSize {
    pub fn dimensions(self) -> (u16, u16) {
        match self {
            Self::Small => (32, 64),
            Self::Medium => (64, 128),
            Self::Large => (128, 256),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorldConfig {
    pub name: String,
    pub seed: u64,
    #[serde(default)]
    pub size: WorldSize,
    #[serde(default = "default_tick_ms")]
    pub tick_duration_ms: u32,
}
fn default_tick_ms() -> u32 {
    1000
}
impl WorldConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("name must not be empty".into());
        }
        if self.name.len() > 120 {
            return Err("name must be 120 characters or fewer".into());
        }
        if !(10..=60_000).contains(&self.tick_duration_ms) {
            return Err("tick_duration_ms must be between 10 and 60000".into());
        }
        if self.seed == 0 {
            return Err("seed must be non-zero".into());
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunState {
    Created,
    Running,
    Paused,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerrainType {
    Ocean,
    Lake,
    River,
    Plains,
    Forest,
    Desert,
    Mountain,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Resources {
    pub water: u32,
    pub vegetation: u32,
    pub minerals: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Tile {
    pub elevation: i16,
    pub moisture: u8,
    pub terrain: TerrainType,
    pub resources: Resources,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Diet {
    Herbivore,
    Carnivore,
    Omnivore,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Sex {
    Female,
    Male,
    Asexual,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Alleles {
    pub first: u8,
    pub second: u8,
}
impl Alleles {
    pub fn expressed(self) -> u8 {
        (self.first as u16 + self.second as u16).div_ceil(2) as u8
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Genome {
    pub speed: Alleles,
    pub metabolism: Alleles,
    pub fertility: Alleles,
    pub lifespan_days: u16,
    pub diet: Diet,
    pub sex: Sex,
}
impl Default for Genome {
    fn default() -> Self {
        Self {
            speed: Alleles {
                first: 1,
                second: 1,
            },
            metabolism: Alleles {
                first: 2,
                second: 2,
            },
            fertility: Alleles {
                first: 5,
                second: 5,
            },
            lifespan_days: 365,
            diet: Diet::Herbivore,
            sex: Sex::Asexual,
        }
    }
}
impl Genome {
    pub fn validate(&self) -> Result<(), String> {
        if [self.speed.first, self.speed.second].iter().any(|v| *v > 4) {
            return Err("speed alleles must be 0..=4".into());
        }
        if [self.metabolism.first, self.metabolism.second]
            .iter()
            .any(|v| *v > 20)
        {
            return Err("metabolism alleles must be 0..=20".into());
        }
        if [self.fertility.first, self.fertility.second]
            .iter()
            .any(|v| *v > 100)
        {
            return Err("fertility alleles must be 0..=100".into());
        }
        if self.lifespan_days == 0 {
            return Err("lifespan_days must be positive".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Organism {
    pub id: u64,
    pub species_id: u64,
    pub x: u16,
    pub y: u16,
    pub age_days: u16,
    pub energy: u32,
    pub water_reserve: u32,
    pub genome: Genome,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Default)]
pub struct WorldStatistics {
    pub births: u64,
    pub deaths: u64,
    pub predation_deaths: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Default)]
pub struct PopulationSample {
    pub day: u64,
    pub population: u32,
    pub herbivores: u32,
    pub carnivores: u32,
    pub omnivores: u32,
    pub average_speed: u8,
    pub average_metabolism: u8,
    pub average_fertility: u8,
    pub births: u64,
    pub deaths: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorldSummary {
    pub id: WorldId,
    pub config: WorldConfig,
    pub state: RunState,
    pub tick: u64,
    pub width: u16,
    pub height: u16,
    pub population: u32,
    pub statistics: WorldStatistics,
    pub state_hash: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorldSnapshot {
    pub id: WorldId,
    pub config: WorldConfig,
    pub state: RunState,
    pub tick: u64,
    pub width: u16,
    pub height: u16,
    pub tiles: Vec<Tile>,
    pub organisms: Vec<Organism>,
    pub statistics: WorldStatistics,
    #[serde(default)]
    pub population_history: Vec<PopulationSample>,
    pub rng_state: u64,
    pub next_organism_id: u64,
    pub state_hash: String,
}

/// Compact live update; the initial WebSocket message carries the immutable map once.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorldUpdate {
    pub id: WorldId,
    pub config: WorldConfig,
    pub state: RunState,
    pub tick: u64,
    pub organisms: Vec<Organism>,
    pub statistics: WorldStatistics,
    pub population_sample: Option<PopulationSample>,
    pub state_hash: String,
}
impl From<&WorldSnapshot> for WorldUpdate {
    fn from(snapshot: &WorldSnapshot) -> Self {
        Self {
            id: snapshot.id,
            config: snapshot.config.clone(),
            state: snapshot.state,
            tick: snapshot.tick,
            organisms: snapshot.organisms.clone(),
            statistics: snapshot.statistics.clone(),
            population_sample: snapshot
                .population_history
                .last()
                .filter(|sample| sample.day == snapshot.tick)
                .cloned(),
            state_hash: snapshot.state_hash.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum SimulationCommand {
    Start,
    Pause,
    Resume,
    Advance { ticks: u64 },
    SetSpeed { tick_duration_ms: u32 },
    IntroduceOrganism { x: u16, y: u16, genome: Genome },
}
