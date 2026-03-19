use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json,
};
use c2_core::session::Session;
use serde::{Deserialize, Serialize};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_sessions))
        .route("/", post(create_session))
        .route("/:id", get(get_session))
        .route("/:id", delete(delete_session))
        .route("/:id/chat", post(chat))
}

async fn list_sessions(State(state): State<AppState>) -> impl IntoResponse {
    match Session::list(&state.db).await {
        Ok(sessions) => Json(sessions).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
struct CreateSessionRequest {
    title: Option<String>,
    directory: Option<String>,
}

async fn create_session(
    State(state): State<AppState>,
    Json(req): Json<CreateSessionRequest>,
) -> impl IntoResponse {
    let dir = req.directory.unwrap_or_else(|| ".".to_string());
    let title = req.title.unwrap_or_else(|| "New session".to_string());
    let session = Session::new(dir, title);
    match session.save(&state.db).await {
        Ok(_) => Json(&session).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn get_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match Session::get(&state.db, &id).await {
        Ok(Some(s)) => Json(s).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn delete_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match Session::delete(&state.db, &id).await {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
struct ChatRequest {
    prompt: String,
}

async fn chat(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<ChatRequest>,
) -> impl IntoResponse {
    // TODO: wire to Processor and stream SSE response
    // Phase 1 stub: just echo
    format!("Session {id}: received prompt: {}", req.prompt)
}
