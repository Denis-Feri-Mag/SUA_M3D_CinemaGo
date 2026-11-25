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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refresh_token_is_64_hex_characters() {
        let refresh_token = generate_refresh_token();
        assert_eq!(refresh_token.len(), 64);
        assert!(refresh_token
            .chars()
            .all(|character| character.is_ascii_hexdigit()));
    }

    #[test]
    fn refresh_tokens_are_different() {
        assert_ne!(generate_refresh_token(), generate_refresh_token());
    }

    #[test]
    fn hash_is_the_same_for_the_same_token() {
        let refresh_token = generate_refresh_token();
        assert_eq!(
            hash_refresh_token(&refresh_token),
            hash_refresh_token(&refresh_token)
        );
        assert_ne!(hash_refresh_token(&refresh_token), refresh_token);
    }
}
