mod cli;
mod server;
mod watcher;

use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let args = cli::Args::parse();
    server::run(args).await
}
