mod proxy;

use crate::config::P2TermdAccess;
use crate::proto::proxy::run_connection;
use iroh::endpoint::Connection;
use iroh::protocol::{AcceptError, ProtocolHandler};
use p2term_lib::error::unpack;

#[derive(Debug)]
pub struct P2TermConnectionHandler {
    p2termd_access: P2TermdAccess,
}

impl P2TermConnectionHandler {
    #[must_use]
    pub fn new(p2termd_access: P2TermdAccess) -> Self {
        Self { p2termd_access }
    }
}

impl ProtocolHandler for P2TermConnectionHandler {
    #[allow(clippy::default_trait_access)]
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        let peer = connection.remote_id();
        if !self.p2termd_access.is_allowed(&peer) {
            tracing::warn!("rejected connection from peer={peer}");
            return Err(AcceptError::NotAllowed {
                meta: Default::default(),
            });
        }
        tracing::debug!("accepted connection from peer={peer}");
        tokio::spawn(async move {
            if let Err(e) = run_connection(connection).await {
                tracing::warn!("connection to peer={peer} failed: {}", unpack(&*e));
            }
        });
        Ok(())
    }
}
