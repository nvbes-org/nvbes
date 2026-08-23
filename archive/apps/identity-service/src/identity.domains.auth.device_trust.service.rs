use chrono::Utc;
use serde_json::Value;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use super::policy::{
    display_name, generate_installation_token, initial_score, trust_level, updated_score,
    user_agent_family, valid_token,
};
use super::profile::{CoarseDeviceProfile, installation_token_hash, network_hash};
use crate::http::error::AppError;

#[derive(Debug, Clone)]
pub struct DeviceTrustAssessment {
    pub device_id: Uuid,
    pub cookie_token: Option<String>,
    pub trust_level: String,
    pub trust_score: i16,
    pub risk_delta: f64,
    pub profile_match: Option<bool>,
    pub known_device: bool,
}

pub async fn pre_auth_risk_score(
    db: &PgPool,
    principal_id: Uuid,
    installation_token: Option<&str>,
    device_profile: Option<&Value>,
    secret: &str,
) -> Result<f64, AppError> {
    let Some(token) = installation_token.filter(|value| valid_token(value)) else {
        return Ok(25.0);
    };
    let token_hash = installation_token_hash(secret, token);
    let row = sqlx::query(
        r#"
        SELECT profile_hash, trust_level, revoked_at, last_seen_at
        FROM account_devices
        WHERE principal_id = $1 AND installation_token_hash = $2
        LIMIT 1
        "#,
    )
    .bind(principal_id)
    .bind(token_hash)
    .fetch_optional(db)
    .await?;

    Ok(match row {
        None => 25.0,
        Some(row)
            if row
                .get::<Option<chrono::DateTime<Utc>>, _>("revoked_at")
                .is_some() =>
        {
            50.0
        }
        Some(row) => {
            let last_seen: Option<chrono::DateTime<Utc>> = row.get("last_seen_at");
            let is_expired =
                last_seen.is_some_and(|ls| ls + chrono::Duration::days(90) <= Utc::now());
            if is_expired || row.get::<String, _>("trust_level") == "restricted" {
                25.0
            } else {
                let previous_profile = row.get::<Option<String>, _>("profile_hash");
                let current_profile = device_profile
                    .and_then(CoarseDeviceProfile::from_value)
                    .map(|profile| profile.keyed_hash(secret));
                if matches!((previous_profile, current_profile), (Some(previous), Some(current)) if previous != current)
                {
                    30.0
                } else {
                    0.0
                }
            }
        }
    })
}

#[allow(clippy::too_many_arguments)]
pub async fn assess_authenticated_device(
    db: &PgPool,
    principal_id: Uuid,
    installation_token: Option<&str>,
    device_profile: Option<&Value>,
    ip: Option<&str>,
    user_agent: Option<&str>,
    amr: &[String],
    acr: &str,
    secret: &str,
) -> Result<DeviceTrustAssessment, AppError> {
    let supplied_token = installation_token.filter(|value| valid_token(value));
    let generated_token = supplied_token.is_none().then(generate_installation_token);
    let token = supplied_token
        .or(generated_token.as_deref())
        .expect("a supplied or generated device token is always available");
    let token_hash = installation_token_hash(secret, token);
    let profile = device_profile.and_then(CoarseDeviceProfile::from_value);
    let profile_hash = profile.as_ref().map(|profile| profile.keyed_hash(secret));
    let current_network_hash = network_hash(secret, ip);
    let ua_family = user_agent_family(user_agent);

    let existing = sqlx::query(
        r#"
        SELECT id, profile_hash, trust_level, trust_score, revoked_at
        FROM account_devices
        WHERE principal_id = $1 AND installation_token_hash = $2
        LIMIT 1
        "#,
    )
    .bind(principal_id)
    .bind(&token_hash)
    .fetch_optional(db)
    .await?;

    if let Some(row) = existing {
        if row
            .get::<Option<chrono::DateTime<Utc>>, _>("revoked_at")
            .is_none()
        {
            return update_known_device(
                db,
                row,
                profile_hash,
                current_network_hash,
                ua_family,
                amr,
                acr,
            )
            .await;
        }

        let replacement_token = generate_installation_token();
        return insert_new_device(
            db,
            principal_id,
            installation_token_hash(secret, &replacement_token),
            Some(replacement_token),
            profile_hash,
            current_network_hash,
            ua_family,
            amr,
            acr,
            user_agent,
        )
        .await;
    }

    insert_new_device(
        db,
        principal_id,
        token_hash,
        generated_token,
        profile_hash,
        current_network_hash,
        ua_family,
        amr,
        acr,
        user_agent,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn insert_new_device(
    db: &PgPool,
    principal_id: Uuid,
    token_hash: String,
    cookie_token: Option<String>,
    profile_hash: Option<String>,
    network_hash: Option<String>,
    ua_family: Option<String>,
    amr: &[String],
    acr: &str,
    user_agent: Option<&str>,
) -> Result<DeviceTrustAssessment, AppError> {
    let score = initial_score(amr, acr);
    let trust_level = trust_level(score);
    let device_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO account_devices (
          id, principal_id, installation_token_hash, profile_hash, display_name,
          trust_level, trust_score, successful_auth_count, last_network_hash,
          last_user_agent_family, last_verified_at, trusted_at, metadata
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, 1, $8, $9,
          CASE WHEN $10 THEN NOW() ELSE NULL END,
          CASE WHEN $6 = 'trusted' THEN NOW() ELSE NULL END,
          jsonb_build_object('profile_minimized', TRUE, 'created_from', 'successful_authentication'))
        "#,
    )
    .bind(device_id)
    .bind(principal_id)
    .bind(token_hash)
    .bind(profile_hash)
    .bind(display_name(user_agent))
    .bind(trust_level)
    .bind(score)
    .bind(network_hash)
    .bind(ua_family)
    .bind(acr == "aal2")
    .execute(db)
    .await?;

    Ok(DeviceTrustAssessment {
        device_id,
        cookie_token,
        trust_level: trust_level.to_string(),
        trust_score: score,
        risk_delta: 25.0,
        profile_match: None,
        known_device: false,
    })
}

async fn update_known_device(
    db: &PgPool,
    row: sqlx::postgres::PgRow,
    profile_hash: Option<String>,
    network_hash: Option<String>,
    ua_family: Option<String>,
    amr: &[String],
    acr: &str,
) -> Result<DeviceTrustAssessment, AppError> {
    let device_id: Uuid = row.get("id");
    let previous_profile: Option<String> = row.get("profile_hash");
    let profile_match = match (&previous_profile, &profile_hash) {
        (Some(previous), Some(current)) => Some(previous == current),
        _ => None,
    };
    let previous_score: i16 = row.get("trust_score");
    let previous_level: String = row.get("trust_level");
    let score = updated_score(previous_score, &previous_level, profile_match, amr, acr);
    let level = trust_level(score);
    let risk_delta = if profile_match == Some(false) {
        30.0
    } else {
        0.0
    };

    sqlx::query(
        r#"
        UPDATE account_devices
        SET profile_hash = COALESCE($2, profile_hash),
            trust_level = $3,
            trust_score = $4,
            successful_auth_count = successful_auth_count + 1,
            last_network_hash = COALESCE($5, last_network_hash),
            last_user_agent_family = COALESCE($6, last_user_agent_family),
            last_seen_at = NOW(),
            last_verified_at = CASE WHEN $7 THEN NOW() ELSE last_verified_at END,
            trusted_at = CASE WHEN $3 = 'trusted' THEN COALESCE(trusted_at, NOW()) ELSE trusted_at END,
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(device_id)
    .bind(profile_hash)
    .bind(level)
    .bind(score)
    .bind(network_hash)
    .bind(ua_family)
    .bind(acr == "aal2")
    .execute(db)
    .await?;

    Ok(DeviceTrustAssessment {
        device_id,
        cookie_token: None,
        trust_level: level.to_string(),
        trust_score: score,
        risk_delta,
        profile_match,
        known_device: true,
    })
}
