use std::{
    io::ErrorKind,
    net::SocketAddr,
    path::{Component, Path, PathBuf},
};

use anyhow::Result;
use axum::{
    Router,
    extract::{Path as UrlPath, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use pulldown_cmark::{Options, Parser as MarkdownParser, html};
use rust_embed::Embed;
use thiserror::Error;
use tower::ServiceBuilder;
use tower_http::{services::ServeDir, trace::TraceLayer};

use crate::cli::Args;

const ROOT_FILE: &str = "README.md";

#[derive(Clone)]
struct AppState {
    docs: PathBuf,
}

#[derive(Embed)]
#[folder = "assets/"]
struct Assets;

#[derive(Debug, Error)]
enum PageError {
    #[error("Page not found")]
    NotFound,
    #[error("Could not read page: {0}")]
    Internal(#[from] std::io::Error),
}

impl IntoResponse for PageError {
    fn into_response(self) -> Response {
        let status = match self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        tracing::warn!(error = %self, %status, "page request failed");
        (status, self.to_string()).into_response()
    }
}

async fn index(State(state): State<AppState>) -> Result<Html<String>, PageError> {
    render_page(&state.docs, ROOT_FILE).await
}

async fn page(
    State(state): State<AppState>,
    UrlPath(path): UrlPath<String>,
) -> Result<Html<String>, PageError> {
    render_page(&state.docs, &path).await
}

async fn render_page(root: &Path, url_path: &str) -> Result<Html<String>, PageError> {
    let relative = Path::new(url_path);
    if !relative
        .components()
        .all(|part| matches!(part, Component::Normal(_)))
    {
        return Err(PageError::NotFound);
    }

    let mut file = root.join(relative);
    if file.is_dir() {
        file.push(ROOT_FILE);
    } else if file.extension().is_none() {
        file.set_extension("md");
    }
    if file.extension().is_none_or(|ext| ext != "md") {
        return Err(PageError::NotFound);
    }
    let file = file.canonicalize().map_err(|err| match err.kind() {
        ErrorKind::NotFound => PageError::NotFound,
        _ => PageError::Internal(err),
    })?;
    if !file.starts_with(root) {
        return Err(PageError::NotFound);
    }

    let markdown = tokio::fs::read_to_string(file).await?;
    let mut body = String::new();
    let options =
        Options::ENABLE_TABLES | Options::ENABLE_TASKLISTS | Options::ENABLE_STRIKETHROUGH;
    html::push_html(&mut body, MarkdownParser::new_ext(&markdown, options));

    Ok(Html(
        include_str!("../assets/page.html").replace("{{ content }}", &body),
    ))
}

async fn stylesheet() -> impl IntoResponse {
    let css = Assets::get("style.css").expect("embedded stylesheet");
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        css.data.to_vec(),
    )
}

pub async fn run(args: Args) -> Result<()> {
    let docs = args.docs.canonicalize()?;
    anyhow::ensure!(docs.is_dir(), "{} is not a directory", docs.display());

    let app = Router::new()
        .route("/", get(index))
        .route("/{*path}", get(page))
        .route("/_rocs/style.css", get(stylesheet))
        .nest_service("/_rocs/files", ServeDir::new(&docs))
        .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()))
        .with_state(AppState { docs });

    let listener = tokio::net::TcpListener::bind(SocketAddr::new(args.bind, args.port)).await?;
    tracing::info!("Listening on http://{}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}
