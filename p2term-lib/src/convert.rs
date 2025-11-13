use anyhow::Context;

pub trait HexConvert: Sized {
    fn try_from_hex(bytes: &[u8]) -> anyhow::Result<Self>;
    fn to_hex(&self) -> String;
}

impl HexConvert for iroh_base::SecretKey {
    fn try_from_hex(bytes: &[u8]) -> anyhow::Result<Self> {
        let bytes: [u8; 32] = bytes.try_into().with_context(|| {
            format!(
                "expected {} bytes for a secret key hex, got {}",
                32,
                bytes.len()
            )
        })?;
        Ok(Self::from_bytes(&bytes))
    }

    #[inline]
    fn to_hex(&self) -> String {
        hex::encode(self.to_bytes())
    }
}

impl HexConvert for iroh_base::PublicKey {
    fn try_from_hex(bytes: &[u8]) -> anyhow::Result<Self> {
        let bytes = hex::decode(bytes).context("failed to decode input as hex")?;
        let len = bytes.len();
        let bytes: [u8; 32] = bytes
            .try_into()
            .map_err(|_| anyhow::anyhow!("expected 32 bytes for a secret key hex, got {len}",))?;
        Self::from_bytes(&bytes).context("Failed to parse public key from bytes")
    }

    #[inline]
    fn to_hex(&self) -> String {
        self.to_string()
    }
}
