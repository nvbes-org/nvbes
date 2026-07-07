use chrono::{DateTime, Utc};
use sqlx::{Row, postgres::PgRow};
use tonic::Status;
use uuid::Uuid;

use crate::grpc::{
    pb::nvbes::developer::v1 as developer,
    service_status::{non_empty, parse_uuid, sql_status},
};

pub async fn record_token_debug_session(
    db: &sqlx::PgPool,
    actor_principal_id: Uuid,
    request: developer::RecordTokenDebugSessionRequest,
) -> Result<developer::TokenDebugSession, Status> {
    let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
    let token_hash_prefix = validate_token_hash_prefix(request.token_hash_prefix)?;
    let access_decision = validate_access_decision(request.access_decision)?;

    let row = sqlx::query(
        r#"
        INSERT INTO developer_token_debug_sessions (
          tenant_id,
          actor_principal_id,
          token_hash_prefix,
          active,
          access_decision
        )
        VALUES ($1, $2, $3, $4, $5)
        RETURNING
          id,
          tenant_id,
          actor_principal_id,
          token_hash_prefix,
          active,
          access_decision,
          created_at
        "#,
    )
    .bind(tenant_id)
    .bind(actor_principal_id)
    .bind(&token_hash_prefix)
    .bind(request.active)
    .bind(&access_decision)
    .fetch_one(db)
    .await
    .map_err(sql_status)?;

    Ok(session_from_row(row))
}

fn session_from_row(row: PgRow) -> developer::TokenDebugSession {
    developer::TokenDebugSession {
        session_id: row.get::<Uuid, _>("id").to_string(),
        tenant_id: row.get::<Uuid, _>("tenant_id").to_string(),
        actor_principal_id: row.get::<Uuid, _>("actor_principal_id").to_string(),
        token_hash_prefix: row.get("token_hash_prefix"),
        active: row.get("active"),
        access_decision: row.get("access_decision"),
        created_at: time_string(row.get("created_at")),
    }
}

fn validate_token_hash_prefix(value: String) -> Result<String, Status> {
    let value = non_empty(value, "token_hash_prefix")?;
    if value.len() != 16 || !value.chars().all(|character| character.is_ascii_hexdigit()) {
        return Err(Status::invalid_argument(
            "token_hash_prefix must be a 16 character hex prefix",
        ));
    }
    Ok(value)
}

fn validate_access_decision(value: String) -> Result<String, Status> {
    let value = non_empty(value, "access_decision")?;
    if !matches!(
        value.as_str(),
        "allowed" | "expired" | "invalid" | "tenant_mismatch"
    ) {
        return Err(Status::invalid_argument(
            "access_decision is not supported for token debug sessions",
        ));
    }
    Ok(value)
}

fn time_string(value: DateTime<Utc>) -> String {
    value.to_rfc3339()
}

#[cfg(test)]
#[path = "developer.grpc.tokens.contract_tests.rs"]
mod contract_tests;

#[cfg(test)]
mod tests {
    use super::{validate_access_decision, validate_token_hash_prefix};

    #[test]
    fn token_hash_prefix_must_be_short_hex_prefix() {
        assert_eq!(
            validate_token_hash_prefix("0123456789abcdef".to_string()).unwrap(),
            "0123456789abcdef"
        );
        assert!(validate_token_hash_prefix("0123456789abcdeg".to_string()).is_err());
        assert!(validate_token_hash_prefix("abc".to_string()).is_err());
    }

    #[test]
    fn access_decision_is_limited_to_account_decisions() {
        assert_eq!(
            validate_access_decision("tenant_mismatch".to_string()).unwrap(),
            "tenant_mismatch"
        );
        assert!(validate_access_decision("maybe".to_string()).is_err());
    }
}
