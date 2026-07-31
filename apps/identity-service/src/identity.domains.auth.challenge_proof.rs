use sqlx::PgPool;

use crate::http::error::AppError;

pub async fn require_pow_solution(
    db: &PgPool,
    pow_nonce: Option<&str>,
    pow_solution: Option<&str>,
) -> Result<(), AppError> {
    let nonce = pow_nonce
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::bad_request("pow_missing", "PoW challenge required."))?;
    let solution = pow_solution
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::bad_request("pow_missing", "PoW solution required."))?;

    crate::domains::auth::pow::verify_solution(db, nonce, solution).await
}
