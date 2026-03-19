pub mod routes;
pub mod state;

use anyhow::Result;
use axum::Router;
use cc_config::Config;
use cc_core::bus::Bus;
use cc_storage::Db;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

pub use state::AppState;

pub async fn serve(config: Config, db: Arc<Db>, bus: Arc<Bus>, addr: SocketAddr) -> Result<()> {
    let state = AppState::new(config, db, bus).await?;
    let app = router(state);

    tracing::info!("cc daemon listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .nest("/session", routes::session::router())
        .nest("/provider", routes::provider::router())
        .nest("/events", routes::events::router())
        .with_state(state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
}
