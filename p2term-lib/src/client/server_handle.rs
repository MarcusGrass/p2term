use crate::proto::ClientOpt;
use crate::streams::{ReadStream, WriteStream};
use anyhow::Context;
use iroh::Endpoint;
use iroh::endpoint::{RecvStream, SendStream};
use iroh_base::{PublicKey, SecretKey};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub struct P2TermServerHandle<W, R> {
    send_stream: W,
    recv_stream: R,
}

impl<W, R> P2TermServerHandle<W, R> {
    pub fn new(w: W, r: R) -> Self {
        Self {
            send_stream: w,
            recv_stream: r,
        }
    }
}

impl P2TermServerHandle<SendStream, RecvStream> {
    pub async fn connect(secret_key: SecretKey, peer: PublicKey) -> anyhow::Result<Self> {
        let ep = Endpoint::builder()
            .alpns(vec![crate::proto::ALPN.to_vec()])
            .secret_key(secret_key)
            .bind()
            .await
            .context("failed to start client endpoint")?;
        let con = ep
            .connect(peer, crate::proto::ALPN)
            .await
            .with_context(|| format!("failed to open connection to peer={peer}"))?;
        let (send_stream, recv_stream) = con
            .open_bi()
            .await
            .context("failed to open bidirectional stream to server")?;
        Ok(Self {
            send_stream,
            recv_stream,
        })
    }
}

impl<W, R> P2TermServerHandle<W, R>
where
    W: WriteStream,
    R: ReadStream,
{
    pub async fn handshake(&mut self, client_opt: &ClientOpt) -> anyhow::Result<()> {
        let bytes = postcard::to_allocvec(client_opt).context("failed to serialize client opt")?;
        let bytes_len: u16 = bytes
            .len()
            .try_into()
            .with_context(|| format!("client opt len too large {}", bytes.len()))?;
        // Write vectored is a good candidate here, but for some reason
        // it only manages the first buffer every time in practice
        self.send_stream.write_u16_le(bytes_len).await?;
        self.send_stream.write_all(&bytes).await?;
        let mut resp_buf = [0u8; crate::proto::WELCOME.len()];
        self.recv_stream
            .read_exact(&mut resp_buf)
            .await
            .context("failed to read server welcome message")?;
        Ok(())
    }

    pub fn decompose(self) -> (W, R) {
        (self.send_stream, self.recv_stream)
    }
}
