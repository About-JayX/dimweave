use crate::daemon::{control::handler, SharedState};
use axum::{
    extract::{State, WebSocketUpgrade},
    response::IntoResponse,
    routing::get,
    Router,
};
use tauri::AppHandle;

pub async fn start(port: u16, state: SharedState, app: AppHandle) {
    let shared = (state, app);
    let router = Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .route("/ws", get(ws_handler))
        .with_state(shared);

    let addr = format!("127.0.0.1:{port}");
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("cannot bind control server");
    eprintln!("[Daemon] control server on ws://{addr}/ws");
    axum::serve(listener, router).await.unwrap();
}

async fn ws_handler(
    State((state, app)): State<(SharedState, AppHandle)>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handler::handle_connection(socket, state, app))
}
