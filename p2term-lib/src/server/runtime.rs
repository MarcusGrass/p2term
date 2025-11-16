use crate::error::unpack;
use crate::server::config::P2TermdCfg;
use crate::server::connection_handler::P2TermConnectionHandler;
use crate::server::router::P2TermRouter;
use crate::server::shell_proxy::ServerShellProxy;
use anyhow::Context;
use tokio::signal::unix::SignalKind;

pub async fn run<Router, S>(config: P2TermdCfg, mut router: Router) -> anyhow::Result<()>
where
    Router: P2TermRouter,
    S: ServerShellProxy,
{
    let handler = P2TermConnectionHandler::new(config.access);
    router.start::<S>(config.secret_key, handler).await?;
    let mut term = tokio::signal::unix::signal(SignalKind::terminate())
        .context("failed to add signal handler for SIGTERM")?;
    let mut int = tokio::signal::unix::signal(SignalKind::interrupt())
        .context("failed to add signal handler for SIGINT")?;
    tokio::select! {
        _ = term.recv() => {
            tracing::info!("received SIGTERM signal, shutting down");
        },
        _ = int.recv() => {
            tracing::info!("received SIGINT, shutting down");
        },
    }
    let timed_shutdown = tokio::time::timeout(std::time::Duration::from_secs(5), router.shutdown());
    tokio::select! {
        _ = term.recv() => {
            tracing::warn!("received SIGTERM signal during shutdown, forcefully exiting");
        }
        _ = int.recv() => {
            tracing::warn!("received SIGINT signal during shutdown, forcefully exiting");
        }
        res = timed_shutdown => match res {
            Ok(Ok(())) => {}
            Ok(Err(e)) => tracing::warn!("failed to shutdown router: {}", unpack(&*e)),
            Err(_elapsed) => tracing::warn!("failed to shutdown router in 5 seconds, exiting forcefully"),
        }
    }
    Ok(())
}
