use crate::server::client_handle::P2TermClientHandle;
use anyhow::Context;
use iroh::endpoint::{Connection, RecvStream, SendStream};
use iroh_base::PublicKey;

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
