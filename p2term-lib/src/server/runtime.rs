use crate::server::config::P2TermdCfg;
use crate::server::connection_handler::P2TermConnectionHandler;
use crate::server::router::P2TermRouter;
use crate::server::shell_proxy::ServerShellProxy;

pub async fn run<Router, S>(
    config: P2TermdCfg,
    mut router: Router,
    mut stop_receiver: tokio::sync::mpsc::Receiver<()>,
) -> anyhow::Result<()>
where
    Router: P2TermRouter,
    S: ServerShellProxy,
{
    let handler = P2TermConnectionHandler::new(config.access, config.shell_cfg);
    router.start::<S>(config.secret_key, handler).await?;
    if stop_receiver.recv().await.is_none() {
        tracing::warn!("recieved ungraceful stop (sender dropped), exiting immediately");
        return Ok(());
    }
    router.shutdown().await
}
