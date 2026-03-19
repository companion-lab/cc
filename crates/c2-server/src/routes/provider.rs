use axum::{Router, extract::State, response::IntoResponse, routing::get, Json};
use serde_json::json;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/", get(list_providers))
}

async fn list_providers(State(state): State<AppState>) -> impl IntoResponse {
    let provider_id = state.config.provider.as_ref().map(|p| p.id.as_str()).unwrap_or("anthropic");
    let model = state.config.model.as_deref().unwrap_or("(default)");
    Json(json!({ "provider": provider_id, "model": model }))
}
