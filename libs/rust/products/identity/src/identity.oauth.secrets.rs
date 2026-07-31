use crate::{IdentityError, IdentityResult};

pub fn hash_client_secret(secret: &str) -> IdentityResult<String> {
    use argon2::{Algorithm, Argon2, Params, PasswordHasher, Version, password_hash::SaltString};
    use password_hash::rand_core::OsRng;

    let salt = SaltString::generate(&mut OsRng);
    let params = Params::new(65_536, 3, 4, None)
        .map_err(|error| IdentityError::internal("client_secret_hash_failed", error.to_string()))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    argon2
        .hash_password(secret.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|error| IdentityError::internal("client_secret_hash_failed", error.to_string()))
}

pub fn verify_client_secret(secret: &str, hash_value: &str) -> IdentityResult<()> {
    use argon2::{Argon2, PasswordVerifier, password_hash::PasswordHash};

    let parsed_hash = PasswordHash::new(hash_value).map_err(|error| {
        IdentityError::internal("client_secret_hash_invalid", error.to_string())
    })?;
    Argon2::default()
        .verify_password(secret.as_bytes(), &parsed_hash)
        .map_err(|_| IdentityError::unauthorized("invalid_client", "The client secret is invalid."))
}

#[cfg(test)]
mod tests {
    use super::{hash_client_secret, verify_client_secret};

    #[test]
    fn hashes_and_verifies_client_secret() {
        let hash = hash_client_secret("secret").expect("hash secret");

        verify_client_secret("secret", &hash).expect("secret should verify");
        verify_client_secret("wrong", &hash).expect_err("wrong secret should fail");
    }
}
