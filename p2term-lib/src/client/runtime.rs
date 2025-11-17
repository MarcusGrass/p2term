use crate::client::shell_proxy::ClientShellProxy;
use crate::connection::{ReadStream, WriteStream};
use crate::proto::ClientOpt;
use crate::server_handle::P2TermServerHandle;
use anyhow::Context;

pub async fn run<W: WriteStream, R: ReadStream, S: ClientShellProxy>(
    mut server: P2TermServerHandle<W, R>,
    client_opt: &ClientOpt,
) -> anyhow::Result<()> {
    server
        .handshake(client_opt)
        .await
        .context("server handshake failed")?;
    let (send, recv) = server.decompose();
    S::run(send, recv)
        .await
        .context("failed to run shell proxy")
}
