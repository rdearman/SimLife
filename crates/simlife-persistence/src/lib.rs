use simlife_domain::WorldSnapshot;
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
        sqlx::query("INSERT INTO worlds (id,name,seed,tick_duration_seconds,run_state,tick,state_hash) VALUES ($1,$2,$3::numeric,$4,$5,$6,$7)")
            .bind(snapshot.id.0).bind(&snapshot.config.name).bind(snapshot.config.seed.to_string())
            .bind(snapshot.config.tick_duration_seconds as i32).bind(format!("{:?}", snapshot.state).to_lowercase())
            .bind(snapshot.tick as i64).bind(&snapshot.state_hash).execute(&self.pool).await?;
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
        sqlx::query("UPDATE worlds SET run_state=$2,tick=$3,state_hash=$4 WHERE id=$1")
            .bind(snapshot.id.0)
            .bind(format!("{:?}", snapshot.state).to_lowercase())
            .bind(snapshot.tick as i64)
            .bind(&snapshot.state_hash)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }
    pub async fn recover(
        &self,
        id: simlife_domain::WorldId,
    ) -> Result<Option<WorldSnapshot>, sqlx::Error> {
        let row: Option<(serde_json::Value,)> = sqlx::query_as("SELECT snapshot FROM world_checkpoints WHERE world_id=$1 ORDER BY tick DESC, created_at DESC LIMIT 1")
            .bind(id.0).fetch_optional(&self.pool).await?;
        row.map(|(v,)| serde_json::from_value(v).map_err(|e| sqlx::Error::Decode(Box::new(e))))
            .transpose()
    }
    pub async fn recover_all(&self) -> Result<Vec<WorldSnapshot>, sqlx::Error> {
        let rows: Vec<(serde_json::Value,)> = sqlx::query_as("SELECT DISTINCT ON (world_id) snapshot FROM world_checkpoints ORDER BY world_id, tick DESC, created_at DESC")
            .fetch_all(&self.pool).await?;
        rows.into_iter()
            .map(|(v,)| serde_json::from_value(v).map_err(|e| sqlx::Error::Decode(Box::new(e))))
            .collect()
    }
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
        let snapshot = World::new(
            WorldId::new(),
            WorldConfig {
                name: "persistence-test".into(),
                seed: 123,
                tick_duration_seconds: 1,
            },
        )
        .unwrap()
        .snapshot();
        store.create_world(&snapshot).await.unwrap();
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
        sqlx::query("DELETE FROM worlds WHERE id=$1")
            .bind(snapshot.id.0)
            .execute(&store.pool)
            .await
            .unwrap();
    }
}
