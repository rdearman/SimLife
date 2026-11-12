use rand::{RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;
use simlife_domain::{RunState, SimulationCommand, WorldConfig, WorldId, WorldSnapshot};
use thiserror::Error;

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
}

/// Standalone deterministic world state. This crate has no networking or storage dependencies.
pub struct World {
    id: WorldId,
    config: WorldConfig,
    state: RunState,
    tick: u64,
    rng: ChaCha8Rng,
    rng_marker: u64,
}

impl World {
    pub fn new(id: WorldId, config: WorldConfig) -> Result<Self, EngineError> {
        config.validate().map_err(EngineError::InvalidConfig)?;
        let seed = config.seed;
        Ok(Self {
            id,
            config,
            state: RunState::Created,
            tick: 0,
            rng: ChaCha8Rng::seed_from_u64(seed),
            rng_marker: 0,
        })
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
            SimulationCommand::Advance { ticks }
                if self.state == RunState::Running && (1..=100_000).contains(&ticks) =>
            {
                for _ in 0..ticks {
                    self.tick += 1;
                    self.rng_marker = self.rng.next_u64();
                }
            }
            SimulationCommand::Advance { .. } => return Err(EngineError::InvalidAdvance),
            _ => return Err(EngineError::InvalidTransition(self.state)),
        }
        Ok(())
    }
    pub fn snapshot(&self) -> WorldSnapshot {
        let canonical = serde_json::json!({ "id": self.id, "config": self.config, "state": self.state, "tick": self.tick, "rng_marker": self.rng_marker });
        WorldSnapshot {
            id: self.id,
            config: self.config.clone(),
            state: self.state,
            tick: self.tick,
            state_hash: blake3::hash(canonical.to_string().as_bytes())
                .to_hex()
                .to_string(),
        }
    }
    pub fn restore(snapshot: &WorldSnapshot) -> Result<Self, EngineError> {
        let mut world = Self::new(snapshot.id, snapshot.config.clone())?;
        world.state = snapshot.state;
        // M1 stores the hashable state; deterministic RNG stream restoration is addressed by replaying ticks.
        if snapshot.tick > 0 {
            for _ in 0..snapshot.tick {
                world.rng_marker = world.rng.next_u64();
            }
        }
        world.tick = snapshot.tick;
        if world.snapshot().state_hash != snapshot.state_hash {
            return Err(EngineError::InvalidCheckpoint);
        }
        Ok(world)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn config(seed: u64) -> WorldConfig {
        WorldConfig {
            name: "test".into(),
            seed,
            tick_duration_seconds: 1,
        }
    }
    #[test]
    fn identical_seed_and_commands_hash_identically() {
        let mut a = World::new(WorldId::new(), config(9)).unwrap();
        let id = a.snapshot().id;
        let mut b = World::new(id, config(9)).unwrap();
        a.apply(SimulationCommand::Start).unwrap();
        b.apply(SimulationCommand::Start).unwrap();
        a.apply(SimulationCommand::Advance { ticks: 25 }).unwrap();
        b.apply(SimulationCommand::Advance { ticks: 25 }).unwrap();
        assert_eq!(a.snapshot().state_hash, b.snapshot().state_hash);
    }
    #[test]
    fn worlds_advance_independently() {
        let mut a = World::new(WorldId::new(), config(1)).unwrap();
        let b = World::new(WorldId::new(), config(1)).unwrap();
        a.apply(SimulationCommand::Start).unwrap();
        a.apply(SimulationCommand::Advance { ticks: 2 }).unwrap();
        assert_eq!(a.snapshot().tick, 2);
        assert_eq!(b.snapshot().tick, 0);
    }
    #[test]
    fn control_transitions_are_checked() {
        let mut w = World::new(WorldId::new(), config(1)).unwrap();
        assert!(w.apply(SimulationCommand::Pause).is_err());
        w.apply(SimulationCommand::Start).unwrap();
        w.apply(SimulationCommand::Pause).unwrap();
        w.apply(SimulationCommand::Resume).unwrap();
        w.apply(SimulationCommand::Advance { ticks: 1 }).unwrap();
        assert_eq!(w.snapshot().tick, 1);
    }
    #[test]
    fn zero_seed_is_rejected() {
        assert!(World::new(WorldId::new(), config(0)).is_err());
    }
    #[test]
    fn checkpoint_restores_the_deterministic_stream() {
        let mut original = World::new(WorldId::new(), config(77)).unwrap();
        original.apply(SimulationCommand::Start).unwrap();
        original
            .apply(SimulationCommand::Advance { ticks: 15 })
            .unwrap();
        let recovered = World::restore(&original.snapshot()).unwrap();
        assert_eq!(
            recovered.snapshot().state_hash,
            original.snapshot().state_hash
        );
    }
}
