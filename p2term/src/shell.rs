use crate::pty::{PtyReader, PtyWriter, subshell_pty_task};
use anyhow::Context;
use std::io::Read;
use std::io::{Stdout, Write};
use std::time::Duration;
use termion::raw::{IntoRawMode, RawTerminal};

pub async fn shell_proxy() -> anyhow::Result<()> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| {
        eprintln!("SHELL not set, defaulting to /bin/bash");
        "/bin/bash".to_string()
    });
    eprintln!("Spawning shell: {}", shell);
    let term_raw = std::io::stdout()
        .into_raw_mode()
        .context("Failed to enter raw mode")?;

    let (pty_write, pty_read) = subshell_pty_task(&shell)?;
    tokio::try_join!(
        proxy_child_stdin(pty_write, termion::async_stdin()),
        proxy_child_stdout(pty_read, term_raw)
    )?;
    Ok(())
}

async fn proxy_child_stdin(
    child_stdin: PtyWriter,
    mut this_stdin: termion::AsyncReader,
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
            child_stdin.write_chunk(&buf[..read_bytes])?;
        } else {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
}

async fn proxy_child_stdout(
    mut pty_reader: PtyReader,
    mut stdout_raw: RawTerminal<Stdout>,
) -> anyhow::Result<()> {
    loop {
        let next = pty_reader.read_bytes().await?;
        stdout_raw.write_all(&next)?;
        stdout_raw.flush()?;
    }
}
