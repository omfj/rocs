use std::{
    io::ErrorKind,
    path::{Component, Path},
};

use axum::{
    Router,
    extract::{Path as UrlPath, State},
    response::Html,
    routing::get,
};
use pulldown_cmark::{Options, Parser as MarkdownParser, html};

use super::{AppState, error};

const ROOT_FILE: &str = "README.md";
const RELOAD_HTML: &str = include_str!("assets/fragments/reload.html");
const PAGE_HTML: &str = include_str!("assets/page.html");

pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(index))
        .route("/{*path}", get(page))
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
