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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorldConfig {
    pub name: String,
    pub seed: u64,
    pub tick_duration_seconds: u32,
}
impl WorldConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("name must not be empty".into());
        }
        if self.name.len() > 120 {
            return Err("name must be 120 characters or fewer".into());
        }
        if self.tick_duration_seconds == 0 {
            return Err("tick_duration_seconds must be positive".into());
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorldSnapshot {
    pub id: WorldId,
    pub config: WorldConfig,
    pub state: RunState,
    pub tick: u64,
    pub state_hash: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum SimulationCommand {
    Start,
    Pause,
    Resume,
    Advance { ticks: u64 },
}
