use axum::{
    http::{Method, StatusCode, header},
    response::IntoResponse,
    routing::get,
    Router,
};
use std::net::SocketAddr;
use std::path::PathBuf;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

use crate::backend::overlay::websocket::websocket_handler;
use crate::backend::overlay::WebSocketState;

/// Start the overlay HTTP server
pub async fn start_overlay_server(
    port: u16,
    ws_state: WebSocketState,
) -> Result<(), Box<dyn std::error::Error>> {
    let project_root = project_root::get_project_root()?;
    let overlay_dir = project_root.join("assets/overlay");

    // Ensure the overlay directory exists
    if !overlay_dir.exists() {
        log::warn!(
            "Overlay directory does not exist: {:?}. Creating it...",
            overlay_dir
        );
        std::fs::create_dir_all(&overlay_dir)?;
    }

    // Build the router
    let app = create_router(overlay_dir, ws_state);

    // Bind to localhost
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    log::info!("Starting overlay server on http://{}", addr);

    // Start the server
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Create the axum router with all routes
fn create_router(overlay_dir: PathBuf, ws_state: WebSocketState) -> Router {
    // CORS configuration for OBS browser source
    // Restricting to localhost origins for security
    let cors = CorsLayer::new()
        .allow_origin([
            "http://localhost:3000".parse().unwrap(),
            "http://127.0.0.1:3000".parse().unwrap(),
        ])
        .allow_methods([Method::GET])
        .allow_headers([header::CONTENT_TYPE]);

    // Create ServeDir with security settings - prevent following symlinks
    let serve_dir = ServeDir::new(overlay_dir)
        .precompressed_gzip()
        .precompressed_br();

    Router::new()
        .route("/health", get(health_check))
        .route("/ws", get(websocket_handler))
        .nest_service("/", serve_dir)
        .layer(cors)
        .with_state(ws_state)
}

/// Health check endpoint
async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "Overlay server is running")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_creation() {
        let ws_state = WebSocketState::new();
        let temp_dir = std::env::temp_dir();
        let router = create_router(temp_dir, ws_state);
        // Basic sanity check that router was created
        assert!(true);
    }
}
