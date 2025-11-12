mod config;
mod termd;

use crate::termd::termd;
use anyhow::Context;
use clap::Parser;
use iroh::SecretKey;
use p2term_lib::convert::HexConvert;
use p2term_lib::crypto::generate_secret_key;
use std::path::PathBuf;

#[derive(Debug, clap::Parser)]
pub struct Args {
    /// The path to a configuration file.
    /// Will allow access from anyone by public key if left empty, and use a generated key-pair for routing
    #[clap(long, short)]
    config_file: Option<PathBuf>,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    tracing_subscriber::fmt::init();
    run(args).await.unwrap();
}

async fn run(args: Args) -> anyhow::Result<()> {
    let config = if let Some(config_file) = args.config_file {
        let bytes = std::fs::read(&config_file).with_context(|| format!("failed to read config file at {}", config_file.display()))?;
        config::P2TermdCfg::parse_toml(&bytes)?
    } else {
        let cfg = config::P2TermdCfg::default();
        tracing::warn!("no config file supplied, using generated key-pair with public_key={} and allowin any connection", cfg.secret_key.public());
        cfg
    };
    termd(config).await
}
