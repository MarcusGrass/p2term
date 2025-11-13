use crate::shell::handler::shell_proxy;
use anyhow::Context;
use iroh::endpoint::Connection;

pub async fn run_connection(connection: Connection) -> anyhow::Result<()> {
    let peer = connection.remote_id();
    let (send, recv) = connection
        .accept_bi()
        .await
        .with_context(|| format!("failed to accept bidirectional stream from peer={peer}"))?;
    shell_proxy(recv, send).await
}
