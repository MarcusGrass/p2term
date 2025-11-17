use crate::shell::pty::{PtyReader, PtyWriter, subshell_pty_task};
use anyhow::Context;
use p2term_lib::connection::{ReadStream, WriteStream};
use p2term_lib::proto::ClientOpt;
use p2term_lib::server::config::ShellCfg;
use p2term_lib::server::shell_proxy::ServerShellProxy;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Debug)]
pub struct ShellProxyImpl;

impl ServerShellProxy for ShellProxyImpl {
    async fn run<W, R>(
        output_stream: W,
        input_stream: R,
        shell_cfg: &ShellCfg,
        client_opt: ClientOpt,
    ) -> anyhow::Result<()>
    where
        W: WriteStream,
        R: ReadStream,
    {
        let shell = client_opt
            .shell
            .as_deref()
            .unwrap_or(shell_cfg.default_shell.as_str());
        let (pty_write, pty_read) = subshell_pty_task(shell, client_opt.cwd.as_deref())?;
        tokio::try_join!(
            proxy_child_stdin(pty_write, input_stream),
            proxy_child_stdout(pty_read, output_stream)
        )?;
        Ok(())
    }
}

async fn proxy_child_stdin<R: ReadStream>(
    child_stdin: PtyWriter,
    mut input_stream: R,
) -> anyhow::Result<()> {
    let mut buf = [0u8; 4096];
    loop {
        let read_bytes = AsyncReadExt::read(&mut input_stream, &mut buf)
            .await
            .context("failed to read from stdin")?;
        if read_bytes == 0 {
            return Ok(());
        }
        if read_bytes > 0 {
            child_stdin.write_chunk(&buf[..read_bytes])?;
        }
    }
}

async fn proxy_child_stdout<W>(mut pty_reader: PtyReader, mut write: W) -> anyhow::Result<()>
where
    W: WriteStream,
{
    loop {
        let next = pty_reader.read_bytes().await?;
        write
            .write_all(&next)
            .await
            .context("failed to write bytes from term over stream")?;
    }
}
