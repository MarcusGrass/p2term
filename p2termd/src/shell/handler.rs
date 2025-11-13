use crate::shell::pty::{PtyReader, PtyWriter, subshell_pty_task};
use anyhow::Context;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub async fn shell_proxy<R, W>(input_stream: R, output_stream: W) -> anyhow::Result<()>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let shell = std::env::var("SHELL").unwrap_or_else(|_| {
        eprintln!("SHELL not set, defaulting to /bin/bash");
        "/bin/bash".to_string()
    });
    let (pty_write, pty_read) = subshell_pty_task(&shell)?;
    tokio::try_join!(
        proxy_child_stdin(pty_write, input_stream),
        proxy_child_stdout(pty_read, output_stream)
    )?;
    Ok(())
}

async fn proxy_child_stdin<R: AsyncRead + Unpin>(
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
        if buf[..read_bytes].contains(&3) {
            anyhow::bail!("Exit on ctrl-c");
        }
        if read_bytes > 0 {
            child_stdin.write_chunk(&buf[..read_bytes])?;
        }
    }
}

async fn proxy_child_stdout<W>(mut pty_reader: PtyReader, mut write: W) -> anyhow::Result<()>
where
    W: AsyncWrite + Unpin,
{
    loop {
        let next = pty_reader.read_bytes().await?;
        write
            .write_all(&next)
            .await
            .context("failed to write bytes from term over stream")?;
    }
}
