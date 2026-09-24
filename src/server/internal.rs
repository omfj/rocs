use std::path::Path;

use axum::{
    Router,
    extract::State,
    http::header,
    response::IntoResponse,
    response::sse::{Event, KeepAlive, Sse},
    routing::get,
};
use futures::stream;
use tokio::sync::broadcast;
use tower_http::services::ServeDir;

use super::AppState;

const STYLE_CSS: &str = include_str!("assets/style.css");

pub(super) fn router(docs: &Path, watch: bool) -> Router<AppState> {
    let router = Router::new()
        .route("/style.css", get(stylesheet))
        .nest_service("/files", ServeDir::new(docs));

    if watch {
        router.route("/events", get(reload_events))
    } else {
        router
    }
}

async fn reload_events(
    State(state): State<AppState>,
) -> Sse<impl futures::Stream<Item = Result<Event, std::convert::Infallible>>> {
    let receiver = state
        .reloads
        .expect("reloads channel is not available")
        .subscribe();
    let events = stream::unfold(receiver, |mut receiver| async move {
        match receiver.recv().await {
            Ok(()) | Err(broadcast::error::RecvError::Lagged(_)) => Some((
                Ok(Event::default().event("reload").data("change detected")),
                receiver,
            )),
            Err(broadcast::error::RecvError::Closed) => None,
        }
    });
    Sse::new(events).keep_alive(KeepAlive::default())
}

async fn stylesheet() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        STYLE_CSS,
    )
}
