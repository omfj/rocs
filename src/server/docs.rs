use std::{
    io::ErrorKind,
    path::{Component, Path},
    sync::OnceLock,
};

use axum::{
    Router,
    extract::{Path as UrlPath, State},
    response::Html,
    routing::get,
};
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser as MarkdownParser, Tag, TagEnd, html};
use syntect::{highlighting::ThemeSet, html::highlighted_html_for_string, parsing::SyntaxSet};

use super::{AppState, error};

const ROOT_FILE: &str = "README.md";
const RELOAD_HTML: &str = include_str!("assets/fragments/reload.html");
const PAGE_HTML: &str = include_str!("assets/page.html");

static SYNTAXES: OnceLock<SyntaxSet> = OnceLock::new();
static THEMES: OnceLock<ThemeSet> = OnceLock::new();

pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(index))
        .route("/{*path}", get(page))
}

async fn index(State(state): State<AppState>) -> Result<Html<String>, error::PageError> {
    render_page(
        &state.docs,
        ROOT_FILE,
        &state.highlight_style,
        state.reloads.is_some(),
    )
    .await
}

async fn page(
    State(state): State<AppState>,
    UrlPath(path): UrlPath<String>,
) -> Result<Html<String>, error::PageError> {
    render_page(
        &state.docs,
        &path,
        &state.highlight_style,
        state.reloads.is_some(),
    )
    .await
}

async fn render_page(
    root: &Path,
    url_path: &str,
    highlight_style: &str,
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
    html::push_html(
        &mut body,
        highlighted_events(&markdown, options, highlight_style).into_iter(),
    );

    let script = if watch { RELOAD_HTML } else { "" };

    Ok(Html(
        PAGE_HTML
            .replace("{{ content }}", &body)
            .replace("{{ watch_script }}", script),
    ))
}

fn highlighted_events<'a>(
    markdown: &'a str,
    options: Options,
    highlight_style: &str,
) -> Vec<Event<'a>> {
    let syntaxes = SYNTAXES.get_or_init(SyntaxSet::load_defaults_newlines);
    let themes = THEMES.get_or_init(ThemeSet::load_defaults);
    let theme = &themes
        .themes
        .get(highlight_style)
        .or_else(|| themes.themes.get("base16-ocean.dark"))
        .unwrap();

    let mut events = Vec::new();
    let mut code: Option<(String, String)> = None;

    for event in MarkdownParser::new_ext(markdown, options) {
        match event {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info))) => {
                let language = info.split_whitespace().next().unwrap_or("");
                if syntaxes.find_syntax_by_token(language).is_some() {
                    code = Some((language.to_owned(), String::new()));
                } else {
                    events.push(Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info))));
                }
            }
            Event::Text(text) if code.is_some() => {
                code.as_mut().unwrap().1.push_str(&text);
            }
            Event::End(TagEnd::CodeBlock) if code.is_some() => {
                let (language, source) = code.take().unwrap();
                let syntax = syntaxes.find_syntax_by_token(&language).unwrap();
                match highlighted_html_for_string(&source, syntaxes, syntax, theme) {
                    Ok(markup) => events.push(Event::Html(markup.into())),
                    Err(_) => {
                        events.push(Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(
                            language.into(),
                        ))));
                        events.push(Event::Text(source.into()));
                        events.push(Event::End(TagEnd::CodeBlock));
                    }
                }
            }
            other => events.push(other),
        }
    }
    events
}
