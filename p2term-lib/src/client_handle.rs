use crate::connection::{ReadStream, WriteStream};
use crate::proto::ClientOpt;
use anyhow::{Context, bail};
use iroh_base::EndpointId;
use std::fmt::Debug;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Debug)]
pub struct P2TermClientHandle<W, R> {
    peer: EndpointId,
    write_stream: W,
    read_stream: R,
}

impl<W, R> P2TermClientHandle<W, R> {
    pub fn new(peer: EndpointId, write: W, read: R) -> Self {
        Self {
            peer,
            write_stream: write,
            read_stream: read,
        }
    }
}

impl<W, R> P2TermClientHandle<W, R>
where
    W: WriteStream,
    R: ReadStream,
{
    pub(crate) async fn recv_hello(&mut self) -> anyhow::Result<ClientOpt> {
        let opt_len = self
            .read_stream
            .read_u16_le()
            .await
            .context("failed to read client opt length")? as usize;
        let mut buf = [0u8; crate::proto::OPT_MAX_LEN];
        if opt_len > crate::proto::OPT_MAX_LEN {
            bail!(
                "read an oversized client opt len of {opt_len} for peer={}",
                self.peer
            )
        }
        // Checked above, some day this will be compiled out, just keep the safety
        let sect = buf
            .get_mut(..opt_len)
            .with_context(|| format!("opt len buf out bounds from peer={}", self.peer))?;
        self.read_stream
            .read_exact(sect)
            .await
            .context("failed to read client opt")?;
        let opt = postcard::from_bytes(&buf[..opt_len])
            .with_context(|| format!("failed to parse client opt from peer={}", self.peer))?;
        self.write_stream
            .write_all(crate::proto::WELCOME)
            .await
            .with_context(|| format!("failed to send welcome message to peer={}", self.peer))?;
        Ok(opt)
    }

    pub(crate) fn decompose(self) -> (W, R) {
        (self.write_stream, self.read_stream)
    }
}
