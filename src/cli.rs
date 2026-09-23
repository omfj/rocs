use std::{
    net::{IpAddr, Ipv4Addr},
    path::PathBuf,
};

use clap::Parser;

#[derive(Parser)]
#[command(about = "Easily serve your Markdown documentation as a website")]
pub struct Args {
    /// Folder containing README.md and other Markdown files
    #[arg(default_value = "./docs")]
    pub docs: PathBuf,

    /// IP address to listen on
    ///
    /// Defaults to 127.0.0.1 (localhost). If you want to listen on all interfaces, use
    /// `--host 0.0.0.0`.
    #[arg(short, long, default_value_t = IpAddr::V4(Ipv4Addr::LOCALHOST))]
    pub bind: IpAddr,

    /// Port to listen on
    #[arg(short, long, default_value_t = 3000)]
    pub port: u16,
}
