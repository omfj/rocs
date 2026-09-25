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
const CLIPBOARD_SVG: &str = include_str!("assets/icons/clipboard.svg");
const CHECK_SVG: &str = include_str!("assets/icons/check.svg");

pub(super) fn router(docs: &Path, watch: bool) -> Router<AppState> {
    let router = Router::new()
        .route("/style.css", get(stylesheet))
        .route("/icons/clipboard.svg", get(clipboard_icon))
        .route("/icons/check.svg", get(check_icon))
        .nest_service("/files", ServeDir::new(docs));

    if watch {
        router.route("/events", get(reload_events))
    } else {
        router
    }
}

async fn stylesheet() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        STYLE_CSS,
    )
}

async fn clipboard_icon() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "image/svg+xml")], CLIPBOARD_SVG)
}

async fn check_icon() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "image/svg+xml")], CHECK_SVG)
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
