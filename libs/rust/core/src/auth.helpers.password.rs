use crate::http::error::AppError;
use argon2::{
    Algorithm, Argon2, Params, Version,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
};
use password_hash::rand_core::OsRng;

pub const TARGET_M_COST: u32 = 65536;
pub const TARGET_T_COST: u32 = 3;
pub const TARGET_P_COST: u32 = 4;

/// A static dummy hash with standard OWASP Argon2id parameters (m=65536, t=3, p=4)
/// used for constant-time dummy verification when an account is not found.
const DUMMY_ARGON2ID_HASH: &str = "$argon2id$v=19$m=65536,t=3,p=4$AAAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";

fn build_argon2<'a>(params: Params, pepper: Option<&'a [u8]>) -> Result<Argon2<'a>, AppError> {
    match pepper {
        Some(secret) if !secret.is_empty() => {
            Argon2::new_with_secret(secret, Algorithm::Argon2id, Version::V0x13, params).map_err(
                |_| {
                    AppError::internal(
                        "password_hash_failed",
                        "Failed to configure Argon2id secret.",
                    )
                },
            )
        }
        _ => Ok(Argon2::new(Algorithm::Argon2id, Version::V0x13, params)),
    }
}

pub fn hash_password(password: &str) -> Result<String, AppError> {
    hash_password_with_pepper(password, None)
}

pub fn hash_password_with_pepper(
    password: &str,
    pepper: Option<&[u8]>,
) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    let params = Params::new(TARGET_M_COST, TARGET_T_COST, TARGET_P_COST, None).map_err(|_| {
        AppError::internal(
            "password_hash_failed",
            "Failed to configure Argon2id parameters.",
        )
    })?;
    let argon2 = build_argon2(params, pepper)?;
    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| AppError::internal("password_hash_failed", "Failed to hash password."))
}

pub fn verify_password(password: &str, password_hash: &str) -> Result<bool, AppError> {
    verify_password_with_pepper(password, password_hash, None)
}

pub fn verify_password_with_pepper(
    password: &str,
    password_hash: &str,
    pepper: Option<&[u8]>,
) -> Result<bool, AppError> {
    let (valid, _) = verify_and_check_rehash(password, password_hash, pepper)?;
    Ok(valid)
}

/// Verifies a password and determines if the password hash needs to be upgraded (re-hashed).
/// Returns `Ok((is_valid, needs_rehash))`.
pub fn verify_and_check_rehash(
    password: &str,
    password_hash: &str,
    pepper: Option<&[u8]>,
) -> Result<(bool, bool), AppError> {
    let parsed_hash = PasswordHash::new(password_hash).map_err(|_| {
        AppError::internal("password_hash_invalid", "Stored password hash is invalid.")
    })?;

    let default_params = Params::new(TARGET_M_COST, TARGET_T_COST, TARGET_P_COST, None)
        .map_err(|_| AppError::internal("password_hash_failed", "Invalid default params"))?;

    // Attempt 1: Verify using current pepper and target params builder
    if let Ok(argon2) = build_argon2(default_params, pepper)
        && argon2
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok()
    {
        let params_match = parsed_hash.algorithm.as_str() == "argon2id"
            && parsed_hash.version == Some(19)
            && Params::try_from(&parsed_hash)
                .map(|p| {
                    p.m_cost() == TARGET_M_COST
                        && p.t_cost() == TARGET_T_COST
                        && p.p_cost() == TARGET_P_COST
                })
                .unwrap_or(false);
        return Ok((true, !params_match));
    }

    // Attempt 2: If pepper was provided but primary verification failed, attempt legacy unpeppered verification
    if pepper.is_some()
        && Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok()
    {
        // Verified using legacy hash without pepper -> valid, but MUST be re-hashed with pepper!
        return Ok((true, true));
    }

    Ok((false, false))
}

/// Performs a dummy Argon2id verification to consume constant computation time (~50-100ms)
/// when an account or password hash does not exist, preventing timing side-channel attacks.
pub fn dummy_verify_password(password: &str, pepper: Option<&[u8]>) {
    if let Ok(parsed_hash) = PasswordHash::new(DUMMY_ARGON2ID_HASH) {
        let params =
            Params::new(TARGET_M_COST, TARGET_T_COST, TARGET_P_COST, None).unwrap_or_default();
        if let Ok(argon2) = build_argon2(params, pepper) {
            let _ = argon2.verify_password(password.as_bytes(), &parsed_hash);
        }
    }
}
