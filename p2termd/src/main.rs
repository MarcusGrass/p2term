mod observability;
mod shell;

use crate::observability::setup_observability;
use crate::shell::handler::ShellProxyImpl;
use anyhow::Context;
use clap::Parser;
use p2term_lib::server::config::P2TermdCfg;
use p2term_lib::server::router::{P2TermRouter, P2TermRouterImpl};
use p2term_lib::server::shell_proxy::ServerShellProxy;
use std::path::PathBuf;

#[derive(Debug, clap::Parser)]
pub struct Args {
    /// The path to a configuration file.
    /// Will allow access from anyone by public key if left empty, and use a generated key-pair for routing
    #[clap(long, short)]
    config_file: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    setup_observability();
    run::<P2TermRouterImpl, ShellProxyImpl>(args).await
}

async fn run<Router, Shell>(args: Args) -> anyhow::Result<()>
where
    Router: P2TermRouter,
    Shell: ServerShellProxy,
{
    let config = if let Some(config_file) = args.config_file {
        let bytes = std::fs::read(&config_file)
            .with_context(|| format!("failed to read config file at {}", config_file.display()))?;
        P2TermdCfg::parse_toml(&bytes)?
    } else {
        let cfg = P2TermdCfg::default();
        tracing::warn!(
            "no config file supplied, using generated key-pair with public_key={} and allowing any connection",
            cfg.secret_key.public()
        );
        cfg
    };
    p2term_lib::server::runtime::run::<Router, Shell>(config).await
}
