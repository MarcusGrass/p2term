use anyhow::Context;
use iroh::endpoint::{RecvStream, SendStream};
use iroh::{Endpoint, PublicKey, SecretKey};
use p2term_lib::proto;
use p2term_lib::proto::ALPN;

pub(crate) async fn connect(
    peer: PublicKey,
    secret_key: SecretKey,
) -> anyhow::Result<(SendStream, RecvStream)> {
    let ep = Endpoint::builder()
        .alpns(vec![ALPN.to_vec()])
        .secret_key(secret_key)
        .bind()
        .await
        .context("failed to start endpoint")?;
    let con = ep
        .connect(peer, ALPN)
        .await
        .with_context(|| format!("failed to open connection to peer={peer}"))?;
    let (mut send, mut recv) = con
        .open_bi()
        .await
        .context("failed to open bidirectional stream")?;
    // Doing this here makes sure that the stream is open on both sides
    send.write_all(proto::HELLO).await?;
    let mut welcome_buf = [0u8; proto::WELCOME.len()];
    recv.read_exact(&mut welcome_buf)
        .await
        .context("failed to read stream open message")?;
    Ok((send, recv))
}
