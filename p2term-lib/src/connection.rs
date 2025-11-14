use crate::client_handle::P2TermClientHandle;
use anyhow::Context;
use iroh::endpoint::{Connection, RecvStream, SendStream};
use iroh_base::PublicKey;
use std::fmt::Debug;
use tokio::io::{AsyncRead, AsyncWrite};

pub trait WriteStream: AsyncWrite + Debug + Unpin + Send + Sync + 'static {}
impl WriteStream for SendStream {}
pub trait ReadStream: AsyncRead + Debug + Unpin + Send + Sync + 'static {}
impl ReadStream for RecvStream {}

pub trait P2TermClientConnection {
    fn peer(&self) -> iroh::PublicKey;
    fn open(&self) -> impl Future<Output = anyhow::Result<(impl WriteStream, impl ReadStream)>>;
}

impl P2TermClientConnection for Connection {
    fn peer(&self) -> iroh::PublicKey {
        self.remote_id()
    }

    async fn open(&self) -> anyhow::Result<(impl WriteStream, impl ReadStream)> {
        let (send, recv) = self
            .open_bi()
            .await
            .context("failed to open bidirectional connection to server")?;
        Ok((send, recv))
    }
}

pub trait P2TermServerConnection<W, R>: Send {
    fn peer(&self) -> iroh::PublicKey;
    fn accept(
        &self,
        peer: PublicKey,
    ) -> impl Future<Output = anyhow::Result<P2TermClientHandle<W, R>>> + Send;
}

impl P2TermServerConnection<SendStream, RecvStream> for Connection {
    fn peer(&self) -> PublicKey {
        self.remote_id()
    }

    async fn accept(
        &self,
        peer: PublicKey,
    ) -> anyhow::Result<P2TermClientHandle<SendStream, RecvStream>> {
        let (send, recv) = self
            .accept_bi()
            .await
            .context("failed to accept bidirectional connection from client")?;
        Ok(P2TermClientHandle::new(peer, send, recv))
    }
}
