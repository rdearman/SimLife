use anyhow::{Context, Result};
use simlife_persistence::Store;
use std::env;
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();
    let database_url = env::var("DATABASE_URL").context("DATABASE_URL must be set")?;
    let bind = env::var("SIMLIFE_BIND").unwrap_or_else(|_| "127.0.0.1:3000".into());
    let store = Store::connect(&database_url)
        .await
        .context("connect to PostgreSQL and apply migrations")?;
    let listener = TcpListener::bind(&bind)
        .await
        .context("bind HTTP listener")?;
    tracing::info!(address=%bind, "SimLife server listening");
    let app = simlife_server::app(store)
        .await
        .context("recover worlds from PostgreSQL")?;
    axum::serve(listener, app).await?;
    Ok(())
}
