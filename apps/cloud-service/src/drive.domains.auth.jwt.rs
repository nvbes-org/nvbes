use std::sync::{OnceLock, RwLock};

use nvbes_identity_sdk::{IdentityJwtVerifier, SdkError};

use crate::http::error::AppError;

const DRIVE_AUDIENCE: &str = "nvbes-cloud-service";

pub type IdentityJwtClaims = nvbes_identity_sdk::IdentityAccessTokenClaims;

static VERIFIER: OnceLock<RwLock<Option<CachedVerifier>>> = OnceLock::new();

struct CachedVerifier {
    issuer: String,
    verifier: IdentityJwtVerifier,
}

pub async fn verify_identity_access_token(token: &str) -> Result<IdentityJwtClaims, AppError> {
    verifier()?
        .verify_access_token(token)
        .await
        .map_err(map_verification_error)
}

fn verifier() -> Result<IdentityJwtVerifier, AppError> {
    let issuer = identity_base_url().trim_end_matches('/').to_string();
    let cache = VERIFIER.get_or_init(|| RwLock::new(None));
    if let Some(verifier) = cache
        .read()
        .map_err(|_| verifier_cache_error())?
        .as_ref()
        .filter(|cached| cached.issuer == issuer)
        .map(|cached| cached.verifier.clone())
    {
        return Ok(verifier);
    }

    let verifier =
        IdentityJwtVerifier::new(&issuer, DRIVE_AUDIENCE).map_err(map_verification_error)?;
    *cache.write().map_err(|_| verifier_cache_error())? = Some(CachedVerifier {
        issuer,
        verifier: verifier.clone(),
    });
    Ok(verifier)
}

fn identity_base_url() -> String {
    std::env::var("NVBES_IDENTITY_SERVICE_BASE_URL")
        .unwrap_or_else(|_| "http://localhost:8080".to_string())
}

fn map_verification_error(error: SdkError) -> AppError {
    match error {
        SdkError::TokenValidation(message) => AppError::unauthorized("invalid_token", message),
        other => AppError::internal("identity_jwks_unavailable", other.to_string()),
    }
}

fn verifier_cache_error() -> AppError {
    AppError::internal(
        "identity_jwt_verifier_unavailable",
        "Identity JWT verifier cache is unavailable.",
    )
}

#[cfg(test)]
mod tests {
    use super::{DRIVE_AUDIENCE, map_verification_error};
    use nvbes_identity_sdk::SdkError;

    #[test]
    fn cloud_verifier_uses_the_exact_cloud_audience() {
        assert_eq!(DRIVE_AUDIENCE, "nvbes-cloud-service");
    }

    #[test]
    fn invalid_tokens_are_exposed_as_unauthorized() {
        let error = map_verification_error(SdkError::invalid_token("wrong audience"));

        assert_eq!(error.code, "invalid_token");
    }
}
