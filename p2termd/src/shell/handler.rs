use crate::shell::pty::{PtyReader, PtyWriter, subshell_pty_task};
use anyhow::Context;
use p2term_lib::error::unpack;
use p2term_lib::proto::ClientOpt;
use p2term_lib::server::config::ShellCfg;
use p2term_lib::server::shell_proxy::ServerShellProxy;
use p2term_lib::streams::{ReadStream, WriteStream};
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
        let (pty_write, pty_read, mut err_recv) =
            subshell_pty_task(shell, client_opt.cwd.as_deref(), client_opt.term.as_deref())?;

        let (input_res, output_res) = tokio::join!(
            proxy_child_stdin(pty_write, input_stream),
            proxy_child_stdout(pty_read, output_stream)
        );
        match (input_res, output_res) {
            (Ok(()), Ok(())) => {
                tracing::info!(
                    "shell session exited normally, both input and output streams closed"
                );
                Ok(())
            }
            (Ok(()), Err(_)) => {
                tracing::info!("shell session exited by peer leaving");
                Ok(())
            }
            (Err(_), Ok(())) => {
                tracing::info!("shell session exited by peer pty closing");
                Ok(())
            }
            (Err(e_in), Err(e_out)) => {
                // Drain thread errors, running in a task so we can wait
                while let Some(next_err) = err_recv.recv().await {
                    tracing::warn!("shell session child thread error: {}", unpack(&*next_err));
                }
                anyhow::bail!(
                    "shell session child input/output proxy both failed: in={}, out={}",
                    unpack(&*e_in),
                    unpack(&*e_out)
                );
            }
        }
    }
}

async fn proxy_child_stdin<R: ReadStream>(
    child_stdin: PtyWriter,
    mut input_stream: R,
) -> anyhow::Result<()> {
    let mut buf = [0u8; 4096];
    loop {
        let read_bytes = match AsyncReadExt::read(&mut input_stream, &mut buf).await {
            Ok(0) => return Ok(()),
            Ok(rb) => rb,
            Err(e) if e.kind() == std::io::ErrorKind::NotConnected => {
                return Ok(());
            }
            Err(e) => return Err(anyhow::anyhow!("failed to read from stdin: {}", unpack(&e))),
        };
        if read_bytes > 0 {
            child_stdin.write_chunk(&buf[..read_bytes]).await?;
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
