use anyhow::Context;
use p2term_lib::error::unpack;
use portable_pty::{CommandBuilder, PtySize};
use std::io::{Read, Write};

pub struct PtyWriter {
    pty_sender: std::sync::mpsc::SyncSender<ShellMessage>,
}

impl PtyWriter {
    pub fn write_chunk(&self, chunk: &[u8]) -> anyhow::Result<()> {
        match chunk.len() {
            0 => Ok(()),
            1 => self
                .pty_sender
                .try_send(ShellMessage::Byte(chunk[0]))
                .context("failed to send message to pty sender"),
            _ => self
                .pty_sender
                .try_send(ShellMessage::Chunk(chunk.to_vec()))
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

pub fn subshell_pty_task(shell: &str) -> anyhow::Result<(PtyWriter, PtyReader)> {
    let pty_sys = portable_pty::native_pty_system();
    let cmd = CommandBuilder::new(shell);
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
    let (input_to_pty, bytes_to_pty) = std::sync::mpsc::sync_channel(128);
    std::thread::spawn(move || {
        if let Err(e) = subshell_writer_task(bytes_to_pty, writer) {
            eprintln!("Error in subshell writer thread: {}", unpack(&*e));
        }
    });
    let (pty_sender, pty_bytes_recv) = tokio::sync::mpsc::channel(128);
    std::thread::spawn(move || {
        if let Err(e) = subshell_reader_task(pty_sender, reader) {
            eprintln!("Error in subshell reader thread: {}", unpack(&*e));
        }
    });
    Ok((
        PtyWriter {
            pty_sender: input_to_pty,
        },
        PtyReader { pty_bytes_recv },
    ))
}

fn subshell_writer_task(
    input: std::sync::mpsc::Receiver<ShellMessage>,
    mut writer: Box<dyn Write + Send>,
) -> anyhow::Result<()> {
    loop {
        let msg = input
            .recv()
            .context("failed to receive message from input channel")?;
        match msg {
            ShellMessage::Byte(b) => writer.write_all(&[b]).context("failed to write to pty")?,
            ShellMessage::Chunk(chunk) => {
                writer.write_all(&chunk).context("failed to write to pty")?
            }
        }
    }
}

fn subshell_reader_task(
    output: tokio::sync::mpsc::Sender<Vec<u8>>,
    mut reader: Box<dyn Read + Send>,
) -> anyhow::Result<()> {
    let mut buf = [0u8; 4096];
    loop {
        let read_bytes = reader.read(&mut buf).context("failed to read from pty")?;
        output
            .try_send(buf[..read_bytes].to_vec())
            .context("failed to send message to output channel")?;
    }
}
