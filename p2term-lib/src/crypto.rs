use rand_core::SeedableRng;

pub fn generate_secret_key() -> iroh_base::SecretKey {
    let mut rng = rand_chacha::ChaCha20Rng::from_os_rng();
    iroh_base::SecretKey::generate(&mut rng)
}
