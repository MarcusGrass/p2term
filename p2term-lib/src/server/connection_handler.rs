use crate::error::unpack;
use crate::server::config::{P2TermdAccess, ShellCfg};
use crate::server::connection::P2TermServerConnection;
use crate::server::shell_proxy::ServerShellProxy;
use crate::streams::{ReadStream, WriteStream};
use anyhow::Context;
use iroh::endpoint::Connection;
use iroh::protocol::{AcceptError, ProtocolHandler};
use iroh_base::PublicKey;
use std::fmt::Debug;
use std::marker::PhantomData;

pub trait ConnectionHandler: Sized + Debug + Send + Sync + 'static {
    fn serve<W, R>(
        &self,
        connection: impl P2TermServerConnection<W, R>,
    ) -> impl Future<Output = Result<(), AcceptError>> + Send
    where
        W: WriteStream,
        R: ReadStream;
}

#[derive(Debug)]
pub struct P2TermConnectionHandler<S> {
    access: P2TermdAccess,
    shell_cfg: ShellCfg,
    _pd: PhantomData<S>,
}

impl<S> P2TermConnectionHandler<S> {
    #[must_use]
    pub fn new(access: P2TermdAccess, shell_cfg: ShellCfg) -> Self {
        Self {
            access,
            shell_cfg,
            _pd: PhantomData,
        }
    }
}

impl<S> ConnectionHandler for P2TermConnectionHandler<S>
where
    S: ServerShellProxy,
{
    #[allow(clippy::default_trait_access)]
    async fn serve<W, R>(
        &self,
        connection: impl P2TermServerConnection<W, R>,
    ) -> Result<(), AcceptError>
    where
        W: WriteStream,
        R: ReadStream,
    {
        let peer = connection.peer();
        if !self.access.is_allowed(&peer) {
            tracing::warn!("rejected connection from peer={peer}");
            return Err(AcceptError::NotAllowed {
                meta: Default::default(),
            });
        }
        tracing::info!("accepted connection from peer={peer}");
        if let Err(e) = serve_client::<W, R, S>(connection, peer, &self.shell_cfg).await {
            tracing::warn!(
                "failed to serve client connection to peer={peer}: {}",
                unpack(&*e)
            );
        }
        Ok(())
    }
}

async fn serve_client<W: WriteStream, R: ReadStream, S: ServerShellProxy>(
    connection: impl P2TermServerConnection<W, R>,
    peer: PublicKey,
    shell_cfg: &ShellCfg,
) -> anyhow::Result<()> {
    let mut client = connection
        .accept(peer)
        .await
        .context("failed to accept client")?;
    let client_opt = client.recv_hello().await?;
    shell_cfg.validate_opt(&client_opt)?;
    let (write, read) = client.decompose();
    S::run::<W, R>(write, read, shell_cfg, client_opt).await
}

impl<S> ProtocolHandler for P2TermConnectionHandler<S>
where
    S: ServerShellProxy,
{
    #[allow(clippy::default_trait_access)]
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        self.serve(connection).await
    }
}
