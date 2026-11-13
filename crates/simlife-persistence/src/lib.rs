use simlife_domain::{SimulationCommand, WorldConfig, WorldId, WorldSnapshot};
use sqlx::{postgres::PgPoolOptions, PgPool};

pub struct Store {
    pool: PgPool,
}

impl Store {
    pub async fn connect(url: &str) -> Result<Self, sqlx::Error> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(url)
            .await?;
        sqlx::migrate!()
            .run(&pool)
            .await
            .map_err(|e| sqlx::Error::Migrate(Box::new(e)))?;
        Ok(Self { pool })
    }
    pub async fn create_world(&self, snapshot: &WorldSnapshot) -> Result<(), sqlx::Error> {
        let (width, height) = snapshot.config.size.dimensions();
        sqlx::query("INSERT INTO worlds (id,name,seed,tick_duration_seconds,run_state,tick,state_hash,size_category,width,height,tick_duration_ms) VALUES ($1,$2,$3::numeric,$4,$5,$6,$7,$8,$9,$10,$11)")
            .bind(snapshot.id.0).bind(&snapshot.config.name).bind(snapshot.config.seed.to_string())
            .bind(snapshot.config.tick_duration_ms.div_ceil(1000) as i32).bind(format!("{:?}", snapshot.state).to_lowercase())
            .bind(snapshot.tick as i64).bind(&snapshot.state_hash)
            .bind(format!("{:?}", snapshot.config.size).to_lowercase()).bind(i32::from(width)).bind(i32::from(height))
            .bind(snapshot.config.tick_duration_ms as i32).execute(&self.pool).await?;
        sqlx::query("INSERT INTO simulation_config (world_id,config) VALUES ($1,$2)")
            .bind(snapshot.id.0)
            .bind(serde_json::to_value(&snapshot.config).expect("config serialization"))
            .execute(&self.pool)
            .await?;
        Ok(())
    }
    pub async fn record_command(
        &self,
        snapshot: &WorldSnapshot,
        command: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("INSERT INTO command_events (world_id,command,world_tick) VALUES ($1,$2,$3)")
            .bind(snapshot.id.0)
            .bind(command)
            .bind(snapshot.tick as i64)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
    pub async fn checkpoint(&self, snapshot: &WorldSnapshot) -> Result<(), sqlx::Error> {
        let json = serde_json::to_value(snapshot).expect("snapshot serialization");
        let mut tx = self.pool.begin().await?;
        sqlx::query("INSERT INTO world_checkpoints (world_id,tick,state_hash,snapshot) VALUES ($1,$2,$3,$4)")
            .bind(snapshot.id.0).bind(snapshot.tick as i64).bind(&snapshot.state_hash).bind(json).execute(&mut *tx).await?;
        sqlx::query(
            "UPDATE worlds SET run_state=$2,tick=$3,state_hash=$4,tick_duration_ms=$5 WHERE id=$1",
        )
        .bind(snapshot.id.0)
        .bind(format!("{:?}", snapshot.state).to_lowercase())
        .bind(snapshot.tick as i64)
        .bind(&snapshot.state_hash)
        .bind(snapshot.config.tick_duration_ms as i32)
        .execute(&mut *tx)
        .await?;
        sqlx::query("UPDATE simulation_config SET config=$2,updated_at=now() WHERE world_id=$1")
            .bind(snapshot.id.0)
            .bind(serde_json::to_value(&snapshot.config).expect("config serialization"))
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }
    pub async fn recover(
        &self,
        id: simlife_domain::WorldId,
    ) -> Result<Option<WorldSnapshot>, sqlx::Error> {
        let row: Option<(serde_json::Value,)> = sqlx::query_as("SELECT snapshot FROM world_checkpoints WHERE world_id=$1 ORDER BY tick DESC, created_at DESC, id DESC LIMIT 1")
            .bind(id.0).fetch_optional(&self.pool).await?;
        row.map(|(v,)| serde_json::from_value(v).map_err(|e| sqlx::Error::Decode(Box::new(e))))
            .transpose()
    }
    pub async fn recover_all(&self) -> Result<Vec<WorldSnapshot>, sqlx::Error> {
        let rows: Vec<(serde_json::Value,)> = sqlx::query_as("SELECT DISTINCT ON (world_id) snapshot FROM world_checkpoints ORDER BY world_id, tick DESC, created_at DESC, id DESC")
            .fetch_all(&self.pool).await?;
        rows.into_iter()
            .map(|(v,)| serde_json::from_value(v).map_err(|e| sqlx::Error::Decode(Box::new(e))))
            .collect()
    }

    /// Rebuild a world deterministically from its seed and accepted command history.
    /// Autonomous ticks are inferred from the tick recorded on each subsequent command.
    pub async fn replay(
        &self,
        id: WorldId,
        target_tick: u64,
    ) -> Result<WorldSnapshot, sqlx::Error> {
        let row: Option<(serde_json::Value,)> =
            sqlx::query_as("SELECT config FROM simulation_config WHERE world_id=$1")
                .bind(id.0)
                .fetch_optional(&self.pool)
                .await?;
        let Some((config_json,)) = row else {
            return Err(sqlx::Error::RowNotFound);
        };
        let config: WorldConfig = serde_json::from_value(config_json)
            .map_err(|error| sqlx::Error::Decode(Box::new(error)))?;
        let mut world = simlife_engine::World::new(id, config)
            .map_err(|error| sqlx::Error::Decode(Box::new(error)))?;
        let events: Vec<(String, i64)> = sqlx::query_as(
            "SELECT command, world_tick FROM command_events WHERE world_id=$1 ORDER BY id",
        )
        .bind(id.0)
        .fetch_all(&self.pool)
        .await?;
        for (serialized, recorded_tick) in events {
            let command: SimulationCommand = serde_json::from_str(&serialized)
                .map_err(|error| sqlx::Error::Decode(Box::new(error)))?;
            let recorded_tick = u64::try_from(recorded_tick)
                .map_err(|error| sqlx::Error::Decode(Box::new(error)))?;
            if recorded_tick > target_tick {
                break;
            }
            match command {
                SimulationCommand::Advance { ticks } => {
                    if world.day().saturating_add(ticks) != recorded_tick {
                        return Err(sqlx::Error::Decode(Box::new(std::io::Error::other(
                            "advance event tick does not match its recorded tick",
                        ))));
                    }
                    world
                        .apply(SimulationCommand::Advance { ticks })
                        .map_err(|error| sqlx::Error::Decode(Box::new(error)))?;
                }
                command => {
                    if recorded_tick < world.day() {
                        return Err(sqlx::Error::Decode(Box::new(std::io::Error::other(
                            "command event tick precedes replay state",
                        ))));
                    }
                    advance_to(&mut world, recorded_tick)?;
                    world
                        .apply(command)
                        .map_err(|error| sqlx::Error::Decode(Box::new(error)))?;
                }
            }
        }
        advance_to(&mut world, target_tick)?;
        Ok(world.snapshot())
    }
}

fn advance_to(world: &mut simlife_engine::World, target_tick: u64) -> Result<(), sqlx::Error> {
    while world.day() < target_tick {
        let ticks = (target_tick - world.day()).min(100_000);
        world
            .apply(SimulationCommand::Advance { ticks })
            .map_err(|error| sqlx::Error::Decode(Box::new(error)))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use simlife_domain::{WorldConfig, WorldId};
    use simlife_engine::World;

    // Set DATABASE_URL to a disposable PostgreSQL database to execute the migration and
    // verify checkpoint recovery as part of the workspace test suite.
    #[tokio::test]
    async fn migrations_and_checkpoint_recovery() {
        let Ok(url) = std::env::var("DATABASE_URL") else {
            eprintln!("skipping PostgreSQL integration test: DATABASE_URL is unset");
            return;
        };
        let store = Store::connect(&url).await.expect("PostgreSQL migrations");
        let mut world = World::new(
            WorldId::new(),
            WorldConfig {
                name: "persistence-test".into(),
                seed: 123,
                size: simlife_domain::WorldSize::Small,
                tick_duration_ms: 1000,
            },
        )
        .unwrap();
        let snapshot = world.snapshot();
        store.create_world(&snapshot).await.unwrap();
        store.checkpoint(&snapshot).await.unwrap();
        let start = SimulationCommand::Start;
        world.apply(start.clone()).unwrap();
        let snapshot = world.snapshot();
        store
            .record_command(&snapshot, &serde_json::to_string(&start).unwrap())
            .await
            .unwrap();
        let introduction = SimulationCommand::IntroduceOrganism {
            x: 0,
            y: 0,
            genome: simlife_domain::Genome::default(),
        };
        world.apply(introduction.clone()).unwrap();
        let snapshot = world.snapshot();
        store
            .record_command(&snapshot, &serde_json::to_string(&introduction).unwrap())
            .await
            .unwrap();
        let advance = SimulationCommand::Advance { ticks: 12 };
        world.apply(advance.clone()).unwrap();
        let snapshot = world.snapshot();
        store
            .record_command(&snapshot, &serde_json::to_string(&advance).unwrap())
            .await
            .unwrap();
        store.checkpoint(&snapshot).await.unwrap();
        assert_eq!(
            store
                .recover(snapshot.id)
                .await
                .unwrap()
                .unwrap()
                .state_hash,
            snapshot.state_hash
        );
        assert_eq!(
            store
                .replay(snapshot.id, snapshot.tick)
                .await
                .unwrap()
                .state_hash,
            snapshot.state_hash
        );
        sqlx::query("DELETE FROM worlds WHERE id=$1")
            .bind(snapshot.id.0)
            .execute(&store.pool)
            .await
            .unwrap();
    }
}
