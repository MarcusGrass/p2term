use crate::shell::handler::shell_proxy;
use anyhow::Context;
use iroh::endpoint::Connection;
use p2term_lib::proto;

pub async fn run_connection(connection: Connection) -> anyhow::Result<()> {
    let peer = connection.remote_id();
    let (mut send, mut recv) = connection
        .accept_bi()
        .await
        .with_context(|| format!("failed to accept bidirectional stream from peer={peer}"))?;
    let mut hello_buf = [0u8; proto::HELLO.len()];
    // Doing this here makes sure that the connection is open on both sides before starting
    recv.read_exact(&mut hello_buf)
        .await
        .context("failed to read hello")?;
    send.write_all(proto::WELCOME).await?;
    shell_proxy(recv, send).await
}
