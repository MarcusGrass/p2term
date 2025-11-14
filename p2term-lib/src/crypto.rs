use crate::convert::{Bytes32Convert, HexConvert};
use anyhow::Context;
use rand_core::SeedableRng;
use std::path::Path;

#[must_use]
pub fn generate_secret_key() -> iroh_base::SecretKey {
    let mut rng = rand_chacha::ChaCha20Rng::from_os_rng();
    iroh_base::SecretKey::generate(&mut rng)
}

pub fn any_secret_key(
    hex_input: Option<&str>,
    file: Option<&Path>,
) -> anyhow::Result<iroh_base::SecretKey> {
    if let Some(hex_input) = hex_input {
        return iroh_base::SecretKey::try_from_hex(hex_input.as_bytes())
            .context("failed to parse secret key hex");
    }
    if let Some(file) = file {
        let bytes = std::fs::read(file)
            .with_context(|| format!("failed to read secret key file={}", file.display()))?;
        return iroh_base::SecretKey::from_bytes_32(&bytes)
            .with_context(|| format!("failed to parse secret key file={}", file.display()));
    }
    let sk = generate_secret_key();
    tracing::info!(
        "no secret key supplied, generated new one with public_key={}",
        sk.public()
    );
    Ok(sk)
}
