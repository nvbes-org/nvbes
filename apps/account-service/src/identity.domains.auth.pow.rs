use rand::RngCore;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::http::error::AppError;

#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct PowChallenge {
    pub nonce: String,
    pub difficulty: u32,
}

pub async fn issue_challenge(
    db: &PgPool,
    difficulty: u32,
    ttl_seconds: i64,
) -> Result<PowChallenge, AppError> {
    prune_expired(db).await?;
    let nonce = generate_nonce();
    sqlx::query(
        r#"
        INSERT INTO pow_challenges (nonce, difficulty, expires_at)
        VALUES ($1, $2, NOW() + ($3 || ' seconds')::interval)
        "#,
    )
    .bind(&nonce)
    .bind(difficulty as i32)
    .bind(ttl_seconds)
    .execute(db)
    .await?;
    Ok(PowChallenge { nonce, difficulty })
}

pub async fn verify_solution(db: &PgPool, nonce: &str, solution: &str) -> Result<(), AppError> {
    let mut transaction = db.begin().await?;
    let row = sqlx::query(
        r#"
        SELECT difficulty, expires_at, consumed_at
        FROM pow_challenges
        WHERE nonce = $1
        FOR UPDATE
        "#,
    )
    .bind(nonce)
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or_else(|| AppError::bad_request("pow_invalid_nonce", "Invalid or expired PoW nonce."))?;

    let consumed: Option<chrono::DateTime<chrono::Utc>> = row.get("consumed_at");
    if consumed.is_some() {
        return Err(AppError::bad_request(
            "pow_nonce_consumed",
            "PoW nonce already used.",
        ));
    }

    let expires: chrono::DateTime<chrono::Utc> = row.get("expires_at");
    if expires <= chrono::Utc::now() {
        return Err(AppError::bad_request(
            "pow_nonce_expired",
            "PoW nonce has expired.",
        ));
    }

    let difficulty: i32 = row.get("difficulty");
    if !check_pow(nonce, solution, difficulty as u32) {
        return Err(AppError::bad_request(
            "pow_solution_invalid",
            "Invalid PoW solution.",
        ));
    }

    sqlx::query("UPDATE pow_challenges SET consumed_at = NOW() WHERE nonce = $1")
        .bind(nonce)
        .execute(&mut *transaction)
        .await?;
    transaction.commit().await?;

    Ok(())
}

fn generate_nonce() -> String {
    let id = Uuid::new_v4();
    let mut nonce_bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut nonce_bytes);
    let nonce_hex: String = nonce_bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!("{}:{}", id.simple(), nonce_hex)
}

pub fn check_pow(nonce: &str, solution: &str, difficulty: u32) -> bool {
    let input = format!("{nonce}:{solution}");
    let hash = Sha256::digest(input.as_bytes());
    leading_zero_bits(&hash) >= difficulty
}

fn leading_zero_bits(hash: &[u8]) -> u32 {
    let mut count = 0u32;
    for &byte in hash.iter() {
        if byte == 0 {
            count += 8;
        } else {
            count += byte.leading_zeros();
            break;
        }
    }
    count
}

async fn prune_expired(db: &PgPool) -> Result<(), AppError> {
    sqlx::query("DELETE FROM pow_challenges WHERE expires_at <= NOW()")
        .execute(db)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leading_zero_bits_empty_hash() {
        let hash = [0u8; 32];
        assert_eq!(leading_zero_bits(&hash), 256);
    }

    #[test]
    fn leading_zero_bits_all_ones() {
        let hash = [0xFFu8; 32];
        assert_eq!(leading_zero_bits(&hash), 0);
    }

    #[test]
    fn pow_check_validates_difficulty() {
        let nonce = "test";
        let difficulty = 16u32;
        let mut solution = 0u64;
        loop {
            let candidate = solution.to_string();
            if check_pow(nonce, &candidate, difficulty) {
                assert!(check_pow(nonce, &candidate, difficulty));
                break;
            }
            solution += 1;
            if solution > 1_000_000 {
                panic!("Could not find valid solution");
            }
        }
    }

    #[test]
    fn pow_rejects_wrong_solution() {
        assert!(!check_pow("test", "wrong", 16));
    }
}
