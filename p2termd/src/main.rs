mod observability;
mod shell;

use crate::observability::setup_observability;
use crate::shell::handler::ShellProxyImpl;
use anyhow::Context;
use clap::Parser;
use p2term_lib::error::unpack;
use p2term_lib::server::config::P2TermdCfg;
use p2term_lib::server::router::{P2TermRouter, P2TermRouterImpl};
use p2term_lib::server::shell_proxy::ServerShellProxy;
use std::path::PathBuf;
use tokio::signal::unix::SignalKind;

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
    let router = P2TermRouterImpl::default();
    run::<P2TermRouterImpl, ShellProxyImpl>(args, router).await
}

async fn run<Router, Shell>(args: Args, router: Router) -> anyhow::Result<()>
where
    Router: P2TermRouter,
    Shell: ServerShellProxy,
{
    let config = if let Some(config_file) = args.config_file {
        let bytes = std::fs::read(&config_file)
            .with_context(|| format!("failed to read config file at {}", config_file.display()))?;
        P2TermdCfg::config_from_toml(&bytes)?
    } else {
        let cfg = P2TermdCfg::default();
        tracing::warn!(
            "no config file supplied, using generated key-pair with public_key={} and allowing any connection",
            cfg.secret_key.public()
        );
        cfg
    };
    let (shutdown_send, shutdown_recv) = tokio::sync::mpsc::channel(2);
    let mut router_task = tokio::task::spawn(p2term_lib::server::runtime::run::<Router, Shell>(
        config,
        router,
        shutdown_recv,
    ));
    let mut term = tokio::signal::unix::signal(SignalKind::terminate())
        .context("failed to add signal handler for SIGTERM")?;
    let mut int = tokio::signal::unix::signal(SignalKind::interrupt())
        .context("failed to add signal handler for SIGINT")?;
    tokio::select! {
        _ = term.recv() => {
            tracing::info!("received SIGTERM signal, shutting down");
            if shutdown_send.try_send(()).is_err() {
                tracing::warn!("failed to send shutdown signal to router, exiting immediately");
                return Ok(())
            }
        },
        _ = int.recv() => {
            tracing::info!("received SIGINT, shutting down");
            if shutdown_send.try_send(()).is_err() {
                tracing::warn!("failed to send shutdown signal to router, exiting immediately");
                return Ok(())
            }
        },
        res = &mut router_task => match res {
            Ok(Ok(())) => {
                tracing::info!("router exited");
                return Ok(())
            }
            Ok(Err(e)) => {
                tracing::error!("router returned err on shutdown: {}", unpack(&*e));
            }
            Err(e) => tracing::warn!("failed to run router: {}", unpack(&e)),
        }
    }
    let timer = tokio::time::sleep(std::time::Duration::from_secs(5));
    tokio::select! {
        _ = term.recv() => {
            tracing::warn!("received SIGTERM signal during shutdown, forcefully exiting");
            if shutdown_send.try_send(()).is_err() {
                tracing::warn!("failed to send shutdown signal to router, exiting immediately");
                return Ok(())
            }
        }
        _ = int.recv() => {
            tracing::warn!("received SIGINT signal during shutdown, forcefully exiting");
            if shutdown_send.try_send(()).is_err() {
                tracing::warn!("failed to send shutdown signal to router, exiting immediately");
                return Ok(())
            }
        }
        () = timer => {
            tracing::warn!("failed to shut down router in time, exiting immediately");
        }
        res = router_task => match res {
            Ok(Ok(())) => {
                tracing::info!("router shut down gracefully, exiting");
            }
            Ok(Err(e)) => {
                tracing::error!("router returned err on shutdown: {}", unpack(&*e));
            }
            Err(e) => {
                tracing::warn!("failed to shutdown router: {}", unpack(&e));
            }
        }
    }
    Ok(())
}
