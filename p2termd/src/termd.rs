use crate::config::P2TermdCfg;
use anyhow::Context;
use iroh::discovery::dns::DnsDiscovery;

pub(super) async fn termd(config: P2TermdCfg) -> anyhow::Result<()> {
    let ep = iroh::Endpoint::builder()
        .discovery(DnsDiscovery::n0_dns())
        .secret_key(config.secret_key)
        .bind()
        .await
        .context("Failed to bind endpoint")?;
    Ok(())
}
