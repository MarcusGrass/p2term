use anyhow::Context;
use std::io::Read;
use std::io::{Stdout, Write};
use std::time::Duration;
use termion::raw::{IntoRawMode, RawTerminal};
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
    eprintln!("Spawning shell: {shell}");
    let term_raw = std::io::stdout()
        .into_raw_mode()
        .context("Failed to enter raw mode")?;

    tokio::try_join!(
        proxy_child_stdin(termion::async_stdin(), output_stream),
        proxy_child_stdout(input_stream, term_raw)
    )?;
    Ok(())
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
        if read_bytes == 1 && buf[0] == 3 {
            anyhow::bail!("Ctrl-C detected, exiting...\r");
        }
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
            println!("received EOF");
        }
        stdout_raw.write_all(&buf[..read_bytes])?;
        stdout_raw.flush()?;
    }
}
