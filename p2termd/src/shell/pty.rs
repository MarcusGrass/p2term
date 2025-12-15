use anyhow::Context;
use p2term_lib::proto::DEFAULT_TERM;
use portable_pty::{CommandBuilder, PtySize};
use std::io::{Read, Write};
use std::path::Path;

pub struct PtyWriter {
    pty_sender: tokio::sync::mpsc::Sender<ShellMessage>,
}

impl PtyWriter {
    pub async fn write_chunk(&self, chunk: &[u8]) -> anyhow::Result<()> {
        match chunk.len() {
            0 => Ok(()),
            1 => self
                .pty_sender
                .send(ShellMessage::Byte(chunk[0]))
                .await
                .context("failed to send message to pty sender"),
            _ => self
                .pty_sender
                .send(ShellMessage::Chunk(chunk.to_vec()))
                .await
                .context("failed to send message to pty sender"),
        }
    }
}

pub struct PtyReader {
    pty_bytes_recv: tokio::sync::mpsc::Receiver<Vec<u8>>,
}

impl PtyReader {
    pub async fn read_bytes(&mut self) -> anyhow::Result<Vec<u8>> {
        self.pty_bytes_recv
            .recv()
            .await
            .context("failed to receive message from pty receiver")
    }
}

enum ShellMessage {
    Byte(u8),
    Chunk(Vec<u8>),
}

pub fn subshell_pty_task(
    shell: &str,
    cwd: Option<&Path>,
    term: Option<&str>,
) -> anyhow::Result<(
    PtyWriter,
    PtyReader,
    tokio::sync::mpsc::Receiver<anyhow::Error>,
)> {
    let pty_sys = portable_pty::native_pty_system();
    let term = term.unwrap_or(DEFAULT_TERM);
    let mut cmd = CommandBuilder::new(shell);
    cmd.env("TERM", term);
    cmd.arg("-l");
    if let Some(cwd) = cwd {
        cmd.cwd(cwd);
    }
    let pty = pty_sys
        .openpty(PtySize::default())
        .context("failed to open pty for shell")?;
    let _child = pty
        .slave
        .spawn_command(cmd)
        .context("failed to spawn shell")?;
    let reader = pty
        .master
        .try_clone_reader()
        .context("failed to clone pty reader")?;
    let writer = pty
        .master
        .take_writer()
        .context("failed to take pty writer")?;
    let (input_to_pty, mut bytes_to_pty) = tokio::sync::mpsc::channel(128);
    let (err_sender, err_receiver) = tokio::sync::mpsc::channel(2);
    let err_c = err_sender.clone();
    std::thread::spawn(move || {
        if let Err(e) = subshell_writer_task(&mut bytes_to_pty, writer) {
            let _ = err_c.blocking_send(e);
        }
    });
    let (pty_sender, pty_bytes_recv) = tokio::sync::mpsc::channel(128);
    std::thread::spawn(move || {
        if let Err(e) = subshell_reader_task(&pty_sender, reader) {
            let _ = err_sender.blocking_send(e);
        }
    });
    Ok((
        PtyWriter {
            pty_sender: input_to_pty,
        },
        PtyReader { pty_bytes_recv },
        err_receiver,
    ))
}

fn subshell_writer_task(
    input: &mut tokio::sync::mpsc::Receiver<ShellMessage>,
    mut writer: Box<dyn Write + Send>,
) -> anyhow::Result<()> {
    loop {
        let msg = input
            .blocking_recv()
            .context("failed to receive message from input channel")?;
        match msg {
            ShellMessage::Byte(b) => writer.write_all(&[b]).context("failed to write to pty")?,
            ShellMessage::Chunk(chunk) => {
                writer.write_all(&chunk).context("failed to write to pty")?;
            }
        }
    }
}

fn subshell_reader_task(
    output: &tokio::sync::mpsc::Sender<Vec<u8>>,
    mut reader: Box<dyn Read + Send>,
) -> anyhow::Result<()> {
    let mut buf = [0u8; 4096];
    loop {
        let read_bytes = reader.read(&mut buf).context("failed to read from pty")?;
        if read_bytes == 0 {
            return Ok(());
        }
        output
            .blocking_send(buf[..read_bytes].to_vec())
            .context("failed to send message to output channel")?;
    }
}
