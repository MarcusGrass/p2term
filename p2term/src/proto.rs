use anyhow::Context;
use iroh::endpoint::{RecvStream, SendStream};
use iroh::{Endpoint, PublicKey, SecretKey};
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
    let (send, recv) = con
        .open_bi()
        .await
        .context("failed to open bidirectional stream")?;
    Ok((send, recv))
}
