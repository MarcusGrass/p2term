use crate::proto::ALPN;
use crate::server::connection_handler::P2TermConnectionHandler;
use crate::server::shell_proxy::ServerShellProxy;
use anyhow::Context;
use iroh::discovery::dns::DnsDiscovery;
use iroh::protocol::{Router, RouterBuilder};
use iroh_base::SecretKey;

pub trait P2TermRouter: Sized {
    fn create<S>(
        secret_key: SecretKey,
        handler: P2TermConnectionHandler<S>,
    ) -> impl Future<Output = anyhow::Result<Self>>
    where
        S: ServerShellProxy;
    fn shutdown(&mut self) -> impl Future<Output = anyhow::Result<()>>;
}

pub struct P2TermRouterImpl {
    inner: Router,
}

impl P2TermRouter for P2TermRouterImpl {
    async fn create<S>(
        secret_key: SecretKey,
        handler: P2TermConnectionHandler<S>,
    ) -> anyhow::Result<Self>
    where
        S: ServerShellProxy,
    {
        let ep = iroh::Endpoint::builder()
            .discovery(DnsDiscovery::n0_dns())
            .secret_key(secret_key)
            .alpns(vec![crate::proto::ALPN.to_vec()])
            .bind()
            .await
            .context("Failed to bind endpoint")?;
        let router = RouterBuilder::new(ep).accept(ALPN, handler).spawn();
        Ok(Self { inner: router })
    }

    async fn shutdown(&mut self) -> anyhow::Result<()> {
        self.inner
            .shutdown()
            .await
            .context("failed to shutdown router")
    }
}
