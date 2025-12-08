use anyhow::Context;

pub trait HexConvert: Sized {
    fn try_from_hex(bytes: &[u8]) -> anyhow::Result<Self>;
    fn to_hex(&self) -> String;
}

pub trait Bytes32Convert: Sized {
    fn from_bytes_32(bytes: &[u8]) -> anyhow::Result<Self>;
}

impl Bytes32Convert for iroh_base::SecretKey {
    fn from_bytes_32(bytes: &[u8]) -> anyhow::Result<Self> {
        let bytes: [u8; 32] = bytes.try_into().map_err(|_e| {
            anyhow::anyhow!("expected 32 bytes key material, got {}", bytes.len(),)
        })?;
        Ok(Self::from_bytes(&bytes))
    }
}

impl Bytes32Convert for iroh_base::PublicKey {
    fn from_bytes_32(bytes: &[u8]) -> anyhow::Result<Self> {
        let bytes: [u8; 32] = bytes.try_into().map_err(|_e| {
            anyhow::anyhow!("expected 32 bytes key material, got {}", bytes.len(),)
        })?;
        Self::from_bytes(&bytes).context("failed to parse public key")
    }
}

trait ToBytes32 {
    fn to_bytes_32(&self) -> [u8; 32];
}

impl ToBytes32 for iroh_base::SecretKey {
    #[inline]
    fn to_bytes_32(&self) -> [u8; 32] {
        self.to_bytes()
    }
}

impl ToBytes32 for iroh_base::PublicKey {
    #[inline]
    fn to_bytes_32(&self) -> [u8; 32] {
        *self.as_bytes()
    }
}

impl<B> HexConvert for B
where
    B: Bytes32Convert + ToBytes32,
{
    fn try_from_hex(bytes: &[u8]) -> anyhow::Result<Self> {
        let bytes = hex::decode(bytes).context("failed to decode input as hex")?;
        Self::from_bytes_32(&bytes)
    }

    #[inline]
    fn to_hex(&self) -> String {
        hex::encode(self.to_bytes_32())
    }
}
