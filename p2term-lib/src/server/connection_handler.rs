use crate::connection::{P2TermServerConnection, ReadStream, WriteStream};
use crate::error::unpack;
use crate::server::config::P2TermdAccess;
use crate::server::shell_proxy::ServerShellProxy;
use anyhow::Context;
use iroh::endpoint::{Connection, RecvStream, SendStream};
use iroh::protocol::{AcceptError, ProtocolHandler};
use iroh_base::PublicKey;
use std::fmt::Debug;
use std::marker::PhantomData;

pub trait ConnectionHandler<W, R>: Sized + Debug + Send + Sync + 'static {
    fn serve(
        &self,
        connection: impl P2TermServerConnection<W, R>,
    ) -> impl Future<Output = Result<(), AcceptError>> + Send;
}

#[derive(Debug)]
pub struct P2TermConnectionHandler<W, R, S> {
    access: P2TermdAccess,
    _pd: PhantomData<(W, R, S)>,
}

impl<W, R, S> P2TermConnectionHandler<W, R, S> {
    pub fn new(access: P2TermdAccess) -> Self {
        Self {
            access,
            _pd: PhantomData,
        }
    }
}

impl<W, R, S> ConnectionHandler<W, R> for P2TermConnectionHandler<W, R, S>
where
    W: WriteStream,
    R: ReadStream,
    S: ServerShellProxy,
{
    #[allow(clippy::default_trait_access)]
    async fn serve(
        &self,
        connection: impl P2TermServerConnection<W, R>,
    ) -> Result<(), AcceptError> {
        let peer = connection.peer();
        if !self.access.is_allowed(&peer) {
            tracing::warn!("rejected connection from peer={peer}");
            return Err(AcceptError::NotAllowed {
                meta: Default::default(),
            });
        }
        tracing::debug!("accepted connection from peer={peer}");
        if let Err(e) = serve_client::<W, R, S>(connection, peer).await {
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
) -> anyhow::Result<()> {
    let mut client = connection
        .accept(peer)
        .await
        .context("failed to accept client")?;
    client.recv_hello().await?;
    let (write, read) = client.decompose();
    S::run::<W, R>(write, read).await
}

#[derive(Debug)]
pub struct P2TermIrohProtocolHandler<H>(H);

impl<H> P2TermIrohProtocolHandler<H> {
    #[must_use]
    pub fn new(inner: H) -> Self {
        Self(inner)
    }
}

impl<H> ProtocolHandler for P2TermIrohProtocolHandler<H>
where
    H: ConnectionHandler<SendStream, RecvStream>,
{
    #[allow(clippy::default_trait_access)]
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        self.0.serve(connection).await
    }
}
