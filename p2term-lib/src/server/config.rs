use crate::convert::HexConvert;
use crate::crypto::{any_secret_key, generate_secret_key};
use crate::proto::ClientOpt;
use anyhow::{Context, bail};
use iroh::{PublicKey, SecretKey};
use rustc_hash::FxHashSet;
use std::path::PathBuf;

#[derive(Debug, serde::Deserialize)]
struct P2TermdTomlCfg {
    secret_key_hex: Option<String>,
    secret_key_file: Option<PathBuf>,
    allowed_peers: Option<Vec<String>>,
    default_shell: Option<String>,
    allowed_shells: Option<Vec<String>>,
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
    pub shell_cfg: ShellCfg,
}

#[derive(Debug)]
pub struct ShellCfg {
    pub default_shell: String,
    pub allowed_shells: Vec<String>,
}

impl ShellCfg {
    fn from_overrides(default_shell: Option<String>, mut allowed_shells: Vec<String>) -> Self {
        let default_shell = establish_default_shell(default_shell);
        if !allowed_shells.contains(&default_shell) {
            allowed_shells.push(default_shell.clone());
        }
        Self {
            default_shell,
            allowed_shells,
        }
    }

    pub fn validate_opt(&self, client_opt: &ClientOpt) -> anyhow::Result<()> {
        if let Some(shell) = client_opt.shell.as_ref()
            && !self.allowed_shells.contains(shell)
        {
            bail!("disallowed shell: {shell} in client opt")
        }
        Ok(())
    }
}

impl Default for P2TermdCfg {
    fn default() -> Self {
        Self {
            secret_key: generate_secret_key(),
            access: P2TermdAccess::Any,
            shell_cfg: ShellCfg::from_overrides(None, vec![]),
        }
    }
}

impl P2TermdCfg {
    pub fn config_from_toml(bytes: &[u8]) -> anyhow::Result<Self> {
        let toml_cfg: P2TermdTomlCfg =
            toml::from_slice(bytes).context("failed to parse toml config")?;
        let secret_key = any_secret_key(
            toml_cfg.secret_key_hex.as_deref(),
            toml_cfg.secret_key_file.as_deref(),
        )?;
        let access = create_access(toml_cfg.allowed_peers)?;
        Ok(Self {
            secret_key,
            access,
            shell_cfg: ShellCfg::from_overrides(
                toml_cfg.default_shell,
                toml_cfg.allowed_shells.unwrap_or_default(),
            ),
        })
    }
}

fn establish_default_shell(default_shell: Option<String>) -> String {
    default_shell
        .or_else(|| std::env::var("SHELL").ok())
        .unwrap_or_else(|| {
            tracing::warn!(
                "$SHELL not set, and default shell not provided, defaulting to /bin/bash"
            );
            "/bin/bash".to_string()
        })
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
