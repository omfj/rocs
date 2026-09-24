mod error;

use std::{
    io::ErrorKind,
    net::SocketAddr,
    path::{Component, Path, PathBuf},
};

use anyhow::Result;
use axum::{
    Router,
    extract::{Path as UrlPath, State},
    http::header,
    response::sse::{Event, KeepAlive, Sse},
    response::{Html, IntoResponse},
    routing::get,
};
use futures::stream;
use pulldown_cmark::{Options, Parser as MarkdownParser, html};
use tokio::sync::broadcast;
use tower::ServiceBuilder;
use tower_http::{services::ServeDir, trace::TraceLayer};

use crate::{cli::Args, watcher::FileWatcher};

const ROOT_FILE: &str = "README.md";

const RELOAD_HTML: &str = include_str!("assets/fragments/reload.html");
const PAGE_HTML: &str = include_str!("assets/page.html");
const STYLE_CSS: &str = include_str!("assets/style.css");

#[derive(Clone)]
struct AppState {
    docs: PathBuf,
    reloads: Option<broadcast::Sender<()>>,
}

async fn index(State(state): State<AppState>) -> Result<Html<String>, error::PageError> {
    render_page(&state.docs, ROOT_FILE, state.reloads.is_some()).await
}

async fn page(
    State(state): State<AppState>,
    UrlPath(path): UrlPath<String>,
) -> Result<Html<String>, error::PageError> {
    render_page(&state.docs, &path, state.reloads.is_some()).await
}

async fn render_page(
    root: &Path,
    url_path: &str,
    watch: bool,
) -> Result<Html<String>, error::PageError> {
    let relative = Path::new(url_path);
    if !relative
        .components()
        .all(|part| matches!(part, Component::Normal(_)))
    {
        return Err(error::PageError::NotFound);
    }

    let mut file = root.join(relative);
    if file.is_dir() {
        file.push(ROOT_FILE);
    } else if file.extension().is_none() {
        file.set_extension("md");
    }
    if file.extension().is_none_or(|ext| ext != "md") {
        return Err(error::PageError::NotFound);
    }
    let file = file.canonicalize().map_err(|err| match err.kind() {
        ErrorKind::NotFound => error::PageError::NotFound,
        _ => error::PageError::Internal(err),
    })?;
    if !file.starts_with(root) {
        return Err(error::PageError::NotFound);
    }

    let markdown = tokio::fs::read_to_string(file).await?;
    let mut body = String::new();
    let options =
        Options::ENABLE_TABLES | Options::ENABLE_TASKLISTS | Options::ENABLE_STRIKETHROUGH;
    html::push_html(&mut body, MarkdownParser::new_ext(&markdown, options));

    let script = if watch { RELOAD_HTML } else { "" };

    Ok(Html(
        PAGE_HTML
            .replace("{{ content }}", &body)
            .replace("{{ watch_script }}", script),
    ))
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

pub async fn run(args: Args) -> Result<()> {
    let docs = args.docs.canonicalize()?;
    anyhow::ensure!(docs.is_dir(), "{} is not a directory", docs.display());

    let watcher = if args.watch {
        Some(FileWatcher::new(&docs)?)
    } else {
        None
    };

    let mut app = Router::new()
        .route("/", get(index))
        .route("/{*path}", get(page));

    // Internal routes
    app = app
        .route("/_rocs/style.css", get(stylesheet))
        .nest_service("/_rocs/files", ServeDir::new(&docs));

    // Add the reload event if watch mode is enabled
    if watcher.is_some() {
        app = app.route("/_rocs/events", get(reload_events));
    }

    // Logging middleware
    app = app.layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()));

    // Add the app state
    let app = app.with_state(AppState {
        docs,
        reloads: watcher.as_ref().map(FileWatcher::reloads),
    });

    let listener = tokio::net::TcpListener::bind(SocketAddr::new(args.bind, args.port)).await?;
    tracing::info!("Listening on http://{}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}
