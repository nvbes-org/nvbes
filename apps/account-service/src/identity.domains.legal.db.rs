use crate::http::error::AppError;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use std::net::IpAddr;
use uuid::Uuid;

use nvbes_core::pagination::KeysetCursor;

#[derive(Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct UserConsent {
    pub id: Uuid,
    pub principal_id: Uuid,
    pub consent_type: String,
    pub document_version: String,
    pub ip_address: Option<String>,
    pub granted_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

/// Anonymize an IP address string:
/// - IPv4: Mask the last octet (e.g., 192.168.1.123 -> 192.168.1.0)
/// - IPv6: Mask the host segment preserving only the /48 prefix (e.g., 2001:db8:85a3::8a2e:370:7334 -> 2001:db8:85a3::)
pub fn anonymize_ip_str(ip_str: &str) -> Option<String> {
    let ip: IpAddr = ip_str.parse().ok()?;
    let masked = match ip {
        IpAddr::V4(ipv4) => {
            let octets = ipv4.octets();
            IpAddr::V4(std::net::Ipv4Addr::new(octets[0], octets[1], octets[2], 0))
        }
        IpAddr::V6(ipv6) => {
            let segments = ipv6.segments();
            IpAddr::V6(std::net::Ipv6Addr::new(
                segments[0],
                segments[1],
                segments[2],
                0,
                0,
                0,
                0,
                0,
            ))
        }
    };
    Some(masked.to_string())
}

/// Records a user's consent for a specific document version.
/// Performs an upsert: if the consent already exists, it is marked as granted (revoked_at reset to NULL).
pub async fn record_consent(
    db: &PgPool,
    principal_id: Uuid,
    consent_type: &str,
    document_version: &str,
    ip_address: Option<&str>,
) -> Result<UserConsent, AppError> {
    let masked_ip = ip_address.and_then(anonymize_ip_str);

    let row = sqlx::query(
        r#"
        INSERT INTO user_consents (principal_id, consent_type, document_version, ip_address, granted_at, revoked_at)
        VALUES ($1, $2, $3, $4::inet, NOW(), NULL)
        ON CONFLICT (principal_id, document_version, consent_type)
        DO UPDATE SET granted_at = NOW(), revoked_at = NULL, ip_address = EXCLUDED.ip_address
        RETURNING id, principal_id, consent_type, document_version, ip_address::text, granted_at, revoked_at
        "#
    )
    .bind(principal_id)
    .bind(consent_type)
    .bind(document_version)
    .bind(masked_ip)
    .fetch_one(db)
    .await?;

    Ok(UserConsent {
        id: row.get("id"),
        principal_id: row.get("principal_id"),
        consent_type: row.get("consent_type"),
        document_version: row.get("document_version"),
        ip_address: row.get("ip_address"),
        granted_at: row.get("granted_at"),
        revoked_at: row.get("revoked_at"),
    })
}

/// Revokes a user's consent for a specific document version.
pub async fn revoke_consent(
    db: &PgPool,
    principal_id: Uuid,
    consent_type: &str,
    document_version: &str,
) -> Result<(), AppError> {
    let result = sqlx::query(
        r#"
        UPDATE user_consents
        SET revoked_at = NOW()
        WHERE principal_id = $1 AND consent_type = $2 AND document_version = $3 AND revoked_at IS NULL
        "#
    )
    .bind(principal_id)
    .bind(consent_type)
    .bind(document_version)
    .execute(db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::not_found(
            "consent_not_found",
            "Active consent for this version and type was not found.",
        ));
    }

    Ok(())
}

/// Returns whether an active (non-revoked) consent exists for the given principal, type, and version.
pub async fn is_consent_active(
    db: &PgPool,
    principal_id: Uuid,
    consent_type: &str,
    document_version: &str,
) -> Result<bool, AppError> {
    let row = sqlx::query(
        r#"
        SELECT 1
        FROM user_consents
        WHERE principal_id = $1 AND consent_type = $2 AND document_version = $3 AND revoked_at IS NULL
        LIMIT 1
        "#,
    )
    .bind(principal_id)
    .bind(consent_type)
    .bind(document_version)
    .fetch_optional(db)
    .await?;

    Ok(row.is_some())
}

/// Lists all consent records for a principal, including revoked entries.
pub async fn list_consents(
    db: &PgPool,
    principal_id: Uuid,
    cursor: Option<&KeysetCursor>,
    limit: i64,
) -> Result<Vec<UserConsent>, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT id, principal_id, consent_type, document_version, ip_address::text, granted_at, revoked_at
        FROM user_consents
        WHERE principal_id = $1
          AND (
            $2::timestamp with time zone IS NULL
            OR (granted_at, id) < ($2, $3)
          )
        ORDER BY granted_at DESC, id DESC
        LIMIT $4
        "#
    )
    .bind(principal_id)
    .bind(cursor.map(|value| value.created_at))
    .bind(cursor.map(|value| value.id))
    .bind(limit)
    .fetch_all(db)
    .await?;

    let consents = rows
        .into_iter()
        .map(|row| UserConsent {
            id: row.get("id"),
            principal_id: row.get("principal_id"),
            consent_type: row.get("consent_type"),
            document_version: row.get("document_version"),
            ip_address: row.get("ip_address"),
            granted_at: row.get("granted_at"),
            revoked_at: row.get("revoked_at"),
        })
        .collect();

    Ok(consents)
}
