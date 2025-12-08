use crate::proto::ALPN;
use crate::server::connection_handler::P2TermConnectionHandler;
use crate::server::shell_proxy::ServerShellProxy;
use anyhow::Context;
use iroh::discovery::dns::DnsDiscovery;
use iroh::protocol::{Router, RouterBuilder};
use iroh_base::SecretKey;

pub trait P2TermRouter: Sized + Send + 'static {
    fn start<S>(
        &mut self,
        secret_key: SecretKey,
        handler: P2TermConnectionHandler<S>,
    ) -> impl Future<Output = anyhow::Result<()>> + Send
    where
        S: ServerShellProxy;
    fn shutdown(&mut self) -> impl Future<Output = anyhow::Result<()>> + Send;
}

#[derive(Default)]
pub struct P2TermRouterImpl {
    inner: Option<Router>,
}

impl P2TermRouter for P2TermRouterImpl {
    async fn start<S>(
        &mut self,
        secret_key: SecretKey,
        handler: P2TermConnectionHandler<S>,
    ) -> anyhow::Result<()>
    where
        S: ServerShellProxy,
    {
        self.inner
            .take()
            .map(|r| tokio::task::spawn(async move { r.shutdown().await }));
        let ep = iroh::Endpoint::builder()
            .discovery(DnsDiscovery::n0_dns())
            .secret_key(secret_key)
            .alpns(vec![crate::proto::ALPN.to_vec()])
            .bind()
            .await
            .context("Failed to bind endpoint")?;
        let router = RouterBuilder::new(ep).accept(ALPN, handler).spawn();
        self.inner = Some(router);
        Ok(())
    }

    async fn shutdown(&mut self) -> anyhow::Result<()> {
        if let Some(inner) = &mut self.inner {
            inner.shutdown().await.context("failed to shutdown router")
        } else {
            Ok(())
        }
    }
}
