use anyhow::Context;
use iroh::{PublicKey, SecretKey};
use p2term_lib::convert::HexConvert;
use p2term_lib::crypto::generate_secret_key;
use rustc_hash::FxHashSet;
use std::path::PathBuf;

#[derive(Debug, serde::Deserialize)]
struct P2TermdTomlCfg {
    secret_key_hex: Option<String>,
    secret_key_file: Option<PathBuf>,
    allowed_peers: Option<Vec<String>>,
}

#[derive(Debug)]
pub enum P2TermdAccess {
    Any,
    AllowedNodes(FxHashSet<PublicKey>),
}

impl P2TermdAccess {
    #[must_use]
    pub fn is_allowed(&self, peer: &PublicKey) -> bool {
        match self {
            Self::Any => true,
            Self::AllowedNodes(allowed) => allowed.contains(peer),
        }
    }
}

#[derive(Debug)]
pub struct P2TermdCfg {
    pub secret_key: SecretKey,
    pub access: P2TermdAccess,
}

impl Default for P2TermdCfg {
    fn default() -> Self {
        Self {
            secret_key: generate_secret_key(),
            access: P2TermdAccess::Any,
        }
    }
}

impl P2TermdCfg {
    pub fn parse_toml(bytes: &[u8]) -> anyhow::Result<Self> {
        let toml_cfg: P2TermdTomlCfg =
            toml::from_slice(bytes).context("failed to parse toml config")?;
        let secret_key = find_or_generate_secret_key(&toml_cfg)?;
        let access = create_access(toml_cfg.allowed_peers)?;
        Ok(Self { secret_key, access })
    }
}

fn find_or_generate_secret_key(toml: &P2TermdTomlCfg) -> anyhow::Result<SecretKey> {
    if let Some(secret_key_hex) = toml.secret_key_hex.as_deref() {
        SecretKey::try_from_hex(secret_key_hex.as_bytes())
            .context("supplied an invalid secret key hex")
    } else if let Some(secret_key_path) = toml.secret_key_file.as_deref() {
        let raw = std::fs::read(secret_key_path).with_context(|| {
            format!(
                "failed to read secret key file at {}",
                secret_key_path.display()
            )
        })?;
        SecretKey::try_from_hex(&raw)
    } else {
        let sk = generate_secret_key();
        tracing::info!(
            "no secret key supplied, generated a new one with pk={}",
            sk.public()
        );
        Ok(sk)
    }
}

fn create_access(allowed_peers: Option<Vec<String>>) -> anyhow::Result<P2TermdAccess> {
    let allowed_peers = allowed_peers.unwrap_or_default();
    if allowed_peers.is_empty() {
        tracing::warn!("allowing any peers, this is potentially insecure");
        return Ok(P2TermdAccess::Any);
    }
    let mut allowed = FxHashSet::default();
    for peer in allowed_peers {
        allowed.insert(
            PublicKey::try_from_hex(peer.as_bytes())
                .with_context(|| format!("invalid peer public key hex: {peer}"))?,
        );
    }
    Ok(P2TermdAccess::AllowedNodes(allowed))
}
