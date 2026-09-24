mod docs;
mod error;
mod internal;

use std::{net::SocketAddr, path::PathBuf};

use anyhow::Result;
use axum::Router;
use tokio::sync::broadcast;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;

use crate::{cli::Args, watcher::FileWatcher};

#[derive(Clone)]
struct AppState {
    docs: PathBuf,
    reloads: Option<broadcast::Sender<()>>,
}

pub async fn run(args: Args) -> Result<()> {
    let docs = args.docs.canonicalize()?;
    anyhow::ensure!(docs.is_dir(), "{} is not a directory", docs.display());

    let watcher = if args.watch {
        Some(FileWatcher::new(&docs)?)
    } else {
        None
    };

    let app = Router::new()
        .merge(docs::router())
        .nest("/_rocs", internal::router(&docs, watcher.is_some()))
        .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()));

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
