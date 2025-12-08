use crate::client::server_handle::P2TermServerHandle;
use crate::client::shell_proxy::ClientShellProxy;
use crate::proto::ClientOpt;
use crate::streams::{ReadStream, WriteStream};
use anyhow::Context;

pub async fn run<W: WriteStream, R: ReadStream, S: ClientShellProxy>(
    mut server: P2TermServerHandle<W, R>,
    client_opt: &ClientOpt,
    shell_proxy: S,
) -> anyhow::Result<()> {
    server
        .handshake(client_opt)
        .await
        .context("server handshake failed")?;
    let (send, recv) = server.decompose();
    shell_proxy
        .run(send, recv)
        .await
        .context("failed to run shell proxy")
}
