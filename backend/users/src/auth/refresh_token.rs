use argon2::password_hash::rand_core::{OsRng, RngCore};
use sha2::{Digest, Sha256};

// Random string the client keeps, we only save its hash
pub fn generate_refresh_token() -> String {
    let mut random_bytes = [0u8; 32];
    OsRng.fill_bytes(&mut random_bytes);
    hex::encode(random_bytes)
}

pub fn hash_refresh_token(refresh_token: &str) -> String {
    let hash = Sha256::digest(refresh_token.as_bytes());
    hex::encode(hash)
}
