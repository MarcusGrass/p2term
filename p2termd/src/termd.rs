use crate::config::P2TermdCfg;
use anyhow::{Context, bail};
use iroh::discovery::dns::DnsDiscovery;
use iroh::protocol::RouterBuilder;
use p2term_lib::error::unpack;
use p2term_lib::proto::ALPN;
use tokio::signal::unix::SignalKind;

pub(super) async fn termd(config: P2TermdCfg) -> anyhow::Result<()> {
    let ep = iroh::Endpoint::builder()
        .discovery(DnsDiscovery::n0_dns())
        .secret_key(config.secret_key)
        .alpns(vec![ALPN.to_vec()])
        .bind()
        .await
        .context("Failed to bind endpoint")?;
    let handler = crate::proto::P2TermConnectionHandler::new(config.access);
    let router = RouterBuilder::new(ep).accept(ALPN, handler).spawn();
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
    match tokio::time::timeout(std::time::Duration::from_secs(5), router.shutdown()).await {
        Ok(Ok(())) => {}
        Ok(Err(e)) => bail!("failed to shutdown router: {}", unpack(&e)),
        Err(_elapsed) => {
            bail!("failed to shutdown router in 5 seconds")
        }
    }
    Ok(())
}
