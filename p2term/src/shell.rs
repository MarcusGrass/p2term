use anyhow::Context;
use p2term_lib::client::shell_proxy::ClientShellProxy;
use p2term_lib::streams::{ReadStream, WriteStream};
use std::io::Read;
use std::io::{Stdout, Write};
use std::time::Duration;
use termion::raw::{IntoRawMode, RawTerminal};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

#[derive(Debug)]
pub struct ShellProxy;

impl ClientShellProxy for ShellProxy {
    async fn run<W, R>(self, write: W, read: R) -> anyhow::Result<()>
    where
        W: WriteStream,
        R: ReadStream,
    {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| {
            eprintln!("SHELL not set, defaulting to /bin/bash");
            "/bin/bash".to_string()
        });
        eprintln!("Spawning shell: {shell}");
        let term_raw = std::io::stdout()
            .into_raw_mode()
            .context("Failed to enter raw mode")?;

        tokio::select! {
            to_child_task = proxy_child_stdin(termion::async_stdin(), write) => {
                to_child_task?;
            }
            from_child_task = proxy_child_stdout(read, term_raw) => {
                from_child_task?;
            }
        }
        Ok(())
    }
}

async fn proxy_child_stdin<W: AsyncWrite + Unpin>(
    mut this_stdin: termion::AsyncReader,
    mut writer: W,
) -> anyhow::Result<()> {
    let mut buf = [0u8; 4096];
    loop {
        let read_bytes = this_stdin
            .read(&mut buf)
            .context("failed to read from stdin")?;
        if read_bytes > 0 {
            writer
                .write_all(&buf[..read_bytes])
                .await
                .context("failed to write bytes from term over stream")?;
        } else {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
}

async fn proxy_child_stdout<R: AsyncRead + Unpin>(
    mut reader: R,
    mut stdout_raw: RawTerminal<Stdout>,
) -> anyhow::Result<()> {
    let mut buf = [0u8; 4096];
    loop {
        let read_bytes = reader
            .read(&mut buf)
            .await
            .context("failed to read bytes from stream")?;
        if read_bytes == 0 {
            return Ok(());
        }
        stdout_raw.write_all(&buf[..read_bytes])?;
        stdout_raw.flush()?;
    }
}
