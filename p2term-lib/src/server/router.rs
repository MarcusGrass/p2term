use crate::proto::ALPN;
use crate::server::config::P2TermdCfg;
use crate::server::connection_handler::{P2TermConnectionHandler, P2TermIrohProtocolHandler};
use crate::server::shell_proxy::ServerShellProxy;
use anyhow::Context;
use iroh::discovery::dns::DnsDiscovery;
use iroh::endpoint::{RecvStream, SendStream};
use iroh::protocol::{Router, RouterBuilder};

pub trait P2TermRouter: Sized {
    fn create<S>(config: P2TermdCfg) -> impl Future<Output = anyhow::Result<Self>>
    where
        S: ServerShellProxy;
    fn shutdown(&mut self) -> impl Future<Output = anyhow::Result<()>>;
}

pub struct P2TermRouterImpl {
    inner: Router,
}

impl P2TermRouter for P2TermRouterImpl {
    async fn create<S>(config: P2TermdCfg) -> anyhow::Result<Self>
    where
        S: ServerShellProxy,
    {
        let ep = iroh::Endpoint::builder()
            .discovery(DnsDiscovery::n0_dns())
            .secret_key(config.secret_key)
            .alpns(vec![crate::proto::ALPN.to_vec()])
            .bind()
            .await
            .context("Failed to bind endpoint")?;
        let handler: P2TermConnectionHandler<SendStream, RecvStream, S> =
            P2TermConnectionHandler::new(config.access);
        let router = RouterBuilder::new(ep)
            .accept(ALPN, P2TermIrohProtocolHandler::new(handler))
            .spawn();
        Ok(Self { inner: router })
    }

    async fn shutdown(&mut self) -> anyhow::Result<()> {
        self.inner
            .shutdown()
            .await
            .context("failed to shutdown router")
    }
}
