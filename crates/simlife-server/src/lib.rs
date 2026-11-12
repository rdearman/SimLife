use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use simlife_domain::{SimulationCommand, WorldConfig, WorldId, WorldSnapshot};
use simlife_engine::{EngineError, World};
use simlife_persistence::Store;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{broadcast, Mutex};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    worlds: Arc<Mutex<HashMap<WorldId, Arc<Mutex<World>>>>>,
    updates: broadcast::Sender<WorldSnapshot>,
    store: Arc<Store>,
}
pub async fn app(store: Store) -> Result<Router, sqlx::Error> {
    let (updates, _) = broadcast::channel(256);
    let recovered = store
        .recover_all()
        .await?
        .into_iter()
        .filter_map(|snapshot| {
            World::restore(&snapshot)
                .ok()
                .map(|world| (snapshot.id, Arc::new(Mutex::new(world))))
        })
        .collect();
    let state = AppState {
        worlds: Arc::new(Mutex::new(recovered)),
        updates,
        store: Arc::new(store),
    };
    Ok(Router::new()
        .route(
            "/api/v1/health",
            get(|| async { Json(serde_json::json!({"status":"ok"})) }),
        )
        .route("/api/v1/worlds", get(list_worlds).post(create_world))
        .route("/api/v1/worlds/:id", get(get_world))
        .route("/api/v1/worlds/:id/commands", post(command))
        .route("/api/v1/worlds/:id/stream", get(websocket))
        .with_state(state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http()))
}
#[derive(serde::Deserialize)]
struct CreateRequest {
    name: String,
    seed: u64,
    #[serde(default = "default_tick")]
    tick_duration_seconds: u32,
}
fn default_tick() -> u32 {
    1
}
async fn create_world(
    State(s): State<AppState>,
    Json(input): Json<CreateRequest>,
) -> Result<(StatusCode, Json<WorldSnapshot>), ApiError> {
    let config = WorldConfig {
        name: input.name,
        seed: input.seed,
        tick_duration_seconds: input.tick_duration_seconds,
    };
    config.validate().map_err(ApiError::bad_request)?;
    let id = WorldId::new();
    let world = World::new(id, config).map_err(ApiError::bad_request)?;
    let snapshot = world.snapshot();
    s.store
        .create_world(&snapshot)
        .await
        .map_err(ApiError::internal)?;
    s.store
        .checkpoint(&snapshot)
        .await
        .map_err(ApiError::internal)?;
    s.worlds
        .lock()
        .await
        .insert(id, Arc::new(Mutex::new(world)));
    let _ = s.updates.send(snapshot.clone());
    Ok((StatusCode::CREATED, Json(snapshot)))
}
async fn list_worlds(State(s): State<AppState>) -> Json<Vec<WorldSnapshot>> {
    let worlds: Vec<_> = s.worlds.lock().await.values().cloned().collect();
    let mut snapshots = Vec::with_capacity(worlds.len());
    for world in worlds {
        snapshots.push(world.lock().await.snapshot());
    }
    Json(snapshots)
}
async fn get_world(
    State(s): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<WorldSnapshot>, ApiError> {
    let id = parse_id(&id)?;
    let world = s
        .worlds
        .lock()
        .await
        .get(&id)
        .cloned()
        .ok_or_else(ApiError::not_found)?;
    let snapshot = world.lock().await.snapshot();
    Ok(Json(snapshot))
}
async fn command(
    State(s): State<AppState>,
    Path(id): Path<String>,
    Json(command): Json<SimulationCommand>,
) -> Result<Json<WorldSnapshot>, ApiError> {
    let id = parse_id(&id)?;
    let world = s
        .worlds
        .lock()
        .await
        .get(&id)
        .cloned()
        .ok_or_else(ApiError::not_found)?;
    let mut world = world.lock().await;
    world.apply(command.clone()).map_err(ApiError::engine)?;
    let snapshot = world.snapshot();
    s.store
        .record_command(&snapshot, &serde_json::to_string(&command).unwrap())
        .await
        .map_err(ApiError::internal)?;
    // Durable checkpoints are explicit/coarse-grained; never persist every simulation tick.
    s.store
        .checkpoint(&snapshot)
        .await
        .map_err(ApiError::internal)?;
    let _ = s.updates.send(snapshot.clone());
    drop(world);
    Ok(Json(snapshot))
}
async fn websocket(
    State(s): State<AppState>,
    Path(id): Path<String>,
    ws: WebSocketUpgrade,
) -> Result<Response, ApiError> {
    let id = parse_id(&id)?;
    if !s.worlds.lock().await.contains_key(&id) {
        return Err(ApiError::not_found());
    }
    Ok(ws.on_upgrade(move |socket| stream_world(socket, s, id)))
}
async fn stream_world(mut socket: WebSocket, s: AppState, id: WorldId) {
    let mut rx = s.updates.subscribe();
    let world = s.worlds.lock().await.get(&id).cloned();
    if let Some(world) = world {
        if let Ok(json) = serde_json::to_string(&world.lock().await.snapshot()) {
            if socket.send(Message::Text(json)).await.is_err() {
                return;
            }
        }
    }
    loop {
        match rx.recv().await {
            Ok(snapshot) if snapshot.id == id => {
                if let Ok(json) = serde_json::to_string(&snapshot) {
                    if socket.send(Message::Text(json)).await.is_err() {
                        break;
                    }
                }
            }
            Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) => continue,
            Err(broadcast::error::RecvError::Closed) => break,
        }
    }
}
fn parse_id(raw: &str) -> Result<WorldId, ApiError> {
    Uuid::parse_str(raw)
        .map(WorldId)
        .map_err(|_| ApiError::bad_request("invalid world id"))
}

struct ApiError(StatusCode, String);
impl ApiError {
    fn bad_request(e: impl ToString) -> Self {
        Self(StatusCode::BAD_REQUEST, e.to_string())
    }
    fn not_found() -> Self {
        Self(StatusCode::NOT_FOUND, "world not found".into())
    }
    fn internal(e: impl ToString) -> Self {
        tracing::error!(error=%e.to_string(), "request failed");
        Self(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal server error".into(),
        )
    }
    fn engine(e: EngineError) -> Self {
        Self(StatusCode::CONFLICT, e.to_string())
    }
}
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(serde_json::json!({"error":self.1}))).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::StreamExt;
    #[test]
    fn create_request_rejects_empty_name() {
        let c = WorldConfig {
            name: " ".into(),
            seed: 1,
            tick_duration_seconds: 1,
        };
        assert!(c.validate().is_err());
    }
    #[test]
    fn rejects_malformed_world_id() {
        assert!(parse_id("nope").is_err());
    }

    #[test]
    fn command_json_is_validated_as_a_protocol_type() {
        assert!(
            serde_json::from_str::<SimulationCommand>(r#"{"command":"advance","ticks":1}"#).is_ok()
        );
        assert!(
            serde_json::from_str::<SimulationCommand>(r#"{"command":"advance","ticks":0}"#).is_ok()
        );
        assert!(serde_json::from_str::<SimulationCommand>(r#"{"command":"teleport"}"#).is_err());
    }

    #[tokio::test]
    async fn websocket_broadcast_channel_delivers_world_updates() {
        let (tx, mut rx) = broadcast::channel(4);
        let config = WorldConfig {
            name: "stream".into(),
            seed: 1,
            tick_duration_seconds: 1,
        };
        let world = World::new(WorldId::new(), config).unwrap();
        let mut world = world;
        world.apply(SimulationCommand::Start).unwrap();
        world
            .apply(SimulationCommand::Advance { ticks: 1 })
            .unwrap();
        let expected = world.snapshot();
        tx.send(expected.clone()).unwrap();
        let received = rx.recv().await.unwrap();
        assert_eq!(received.tick, expected.tick);
        assert_eq!(received.id, expected.id);
    }

    #[tokio::test]
    async fn websocket_endpoint_delivers_world_updates() {
        let Ok(url) = std::env::var("DATABASE_URL") else {
            eprintln!("skipping WebSocket integration test: DATABASE_URL is unset");
            return;
        };
        let store = Store::connect(&url).await.unwrap();
        let app = app(store).await.unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let client = reqwest::Client::new();
        let created: WorldSnapshot = client
            .post(format!("http://{address}/api/v1/worlds"))
            .json(
                &serde_json::json!({"name":"websocket-test","seed":177,"tick_duration_seconds":1}),
            )
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .json()
            .await
            .unwrap();
        let (mut socket, _) = tokio_tungstenite::connect_async(format!(
            "ws://{address}/api/v1/worlds/{}/stream",
            created.id.0
        ))
        .await
        .unwrap();
        let initial = tokio::time::timeout(std::time::Duration::from_secs(2), socket.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap()
            .into_text()
            .unwrap();
        assert_eq!(
            serde_json::from_str::<WorldSnapshot>(&initial)
                .unwrap()
                .tick,
            0
        );
        client
            .post(format!(
                "http://{address}/api/v1/worlds/{}/commands",
                created.id.0
            ))
            .json(&SimulationCommand::Start)
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap();
        let started = tokio::time::timeout(std::time::Duration::from_secs(2), socket.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap()
            .into_text()
            .unwrap();
        assert_eq!(
            serde_json::from_str::<WorldSnapshot>(&started)
                .unwrap()
                .tick,
            0
        );
        client
            .post(format!(
                "http://{address}/api/v1/worlds/{}/commands",
                created.id.0
            ))
            .json(&SimulationCommand::Advance { ticks: 1 })
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap();
        let update = tokio::time::timeout(std::time::Duration::from_secs(2), socket.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap()
            .into_text()
            .unwrap();
        assert_eq!(
            serde_json::from_str::<WorldSnapshot>(&update).unwrap().tick,
            1
        );
        server.abort();
    }
}
