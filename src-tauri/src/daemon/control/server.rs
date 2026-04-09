use crate::daemon::{control::claude_sdk_handler, control::handler, SharedState};
use axum::{
    extract::{State, WebSocketUpgrade},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use tauri::AppHandle;

pub async fn start(port: u16, state: SharedState, app: AppHandle) -> anyhow::Result<()> {
    let shared = (state, app);
    let router = Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .route("/ws", get(ws_handler))
        .route("/claude", get(claude_sdk_handler::ws_handler))
        .route("/claude/events", post(claude_sdk_handler::events_handler))
        .with_state(shared);

    let addr = format!("127.0.0.1:{port}");
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| anyhow::anyhow!("cannot bind control server on {addr}: {e}"))?;
    eprintln!("[Daemon] control server on ws://{addr}/ws");
    axum::serve(listener, router)
        .await
        .map_err(|e| anyhow::anyhow!("control server error: {e}"))
}

async fn ws_handler(
    State((state, app)): State<(SharedState, AppHandle)>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handler::handle_connection(socket, state, app))
}
