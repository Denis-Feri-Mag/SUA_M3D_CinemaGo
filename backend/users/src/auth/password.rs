use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;

pub fn hash_password(password: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .expect("could not hash password")
        .to_string()
}

pub fn verify_password(password: &str, password_hash: &str) -> bool {
    let parsed_hash = match PasswordHash::new(password_hash) {
        Ok(parsed_hash) => parsed_hash,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correct_password_is_accepted() {
        let password_hash = hash_password("secret123");
        assert!(verify_password("secret123", &password_hash));
    }

    #[test]
    fn wrong_password_is_rejected() {
        let password_hash = hash_password("secret123");
        assert!(!verify_password("wrong-password", &password_hash));
    }

    #[test]
    fn same_password_gives_different_hashes() {
        // Each hash has its own random salt
        assert_ne!(hash_password("secret123"), hash_password("secret123"));
    }

    #[test]
    fn broken_hash_is_rejected() {
        assert!(!verify_password("secret123", "not-a-real-hash"));
    }
}
