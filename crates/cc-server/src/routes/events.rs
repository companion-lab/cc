use axum::{Router, extract::State, response::IntoResponse, routing::get};
use axum::response::sse::{Event, Sse};
use crate::state::AppState;
use futures::stream::{self, StreamExt};
use tokio_stream::wrappers::BroadcastStream;
use std::convert::Infallible;

pub fn router() -> Router<AppState> {
    Router::new().route("/", get(sse_handler))
}

async fn sse_handler(State(state): State<AppState>) -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>> {
    let rx = state.bus.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|msg| async move {
        match msg {
            Ok(event) => {
                let data = serde_json::to_string(&event).unwrap_or_default();
                Some(Ok(Event::default().data(data)))
            }
            Err(_) => None,
        }
    });
    Sse::new(stream)
}
