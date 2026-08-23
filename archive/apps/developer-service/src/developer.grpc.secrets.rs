use chrono::{DateTime, Utc};
use sqlx::{Postgres, Row, Transaction, postgres::PgRow};
use tonic::Status;
use uuid::Uuid;

use crate::grpc::{
    pb::nvbes::developer::v1 as developer,
    service_status::{non_empty, parse_uuid, sql_status},
};

pub async fn list_secret_versions(
    db: &sqlx::PgPool,
    request: developer::ListSecretVersionsRequest,
) -> Result<developer::ListSecretVersionsResponse, Status> {
    let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
    let client_id = non_empty(request.client_id, "client_id")?;
    Ok(developer::ListSecretVersionsResponse {
        secret_versions: list_secret_version_rows(db, tenant_id, &client_id).await?,
    })
}

pub async fn record_secret_rotation(
    db: &sqlx::PgPool,
    actor_id: Uuid,
    request: developer::RecordSecretRotationRequest,
) -> Result<developer::SecretRotation, Status> {
    let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
    let client_id = non_empty(request.client_id, "client_id")?;
    let current_hash = non_empty(request.current_secret_hash, "current_secret_hash")?;
    let new_hash = non_empty(request.new_secret_hash, "new_secret_hash")?;
    let new_last4 = non_empty(request.new_secret_last4, "new_secret_last4")?;
    let overlap_ends_at = parse_time(&request.overlap_ends_at, "overlap_ends_at")?;
    let rotated_at = parse_time(&request.rotated_at, "rotated_at")?;

    let mut tx = db.begin().await.map_err(sql_status)?;
    let previous_version_id = ensure_previous_overlap_version(
        &mut tx,
        tenant_id,
        &client_id,
        &current_hash,
        overlap_ends_at,
    )
    .await?;
    let active_version_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO developer_client_secret_versions (
          tenant_id,
          client_id,
          status,
          client_secret_hash,
          secret_last4,
          created_at
        )
        VALUES ($1, $2, 'active', $3, $4, $5)
        RETURNING id
        "#,
    )
    .bind(tenant_id)
    .bind(&client_id)
    .bind(new_hash)
    .bind(new_last4)
    .bind(rotated_at)
    .fetch_one(&mut *tx)
    .await
    .map_err(sql_status)?;

    sqlx::query(
        r#"
        UPDATE developer_secret_rotations
        SET revoked_at = $3
        WHERE tenant_id = $1
          AND client_id = $2
          AND revoked_at IS NULL
        "#,
    )
    .bind(tenant_id)
    .bind(&client_id)
    .bind(rotated_at)
    .execute(&mut *tx)
    .await
    .map_err(sql_status)?;

    sqlx::query(
        r#"
        INSERT INTO developer_secret_rotations (
          tenant_id,
          client_id,
          previous_version_id,
          active_version_id,
          previous_version_expires_at,
          overlap_ends_at,
          rotated_by,
          created_at
        )
        VALUES ($1, $2, $3, $4, $5, $5, $6, $7)
        "#,
    )
    .bind(tenant_id)
    .bind(&client_id)
    .bind(previous_version_id)
    .bind(active_version_id)
    .bind(overlap_ends_at)
    .bind(actor_id)
    .bind(rotated_at)
    .execute(&mut *tx)
    .await
    .map_err(sql_status)?;
    tx.commit().await.map_err(sql_status)?;

    Ok(developer::SecretRotation {
        client_id,
        active_version_id: active_version_id.to_string(),
        previous_version_id: previous_version_id.to_string(),
        overlap_ends_at: overlap_ends_at.to_rfc3339(),
        rotated_at: rotated_at.to_rfc3339(),
    })
}

pub async fn revoke_secret_version(
    db: &sqlx::PgPool,
    request: developer::RevokeSecretVersionRequest,
) -> Result<developer::SecretVersionRevocation, Status> {
    let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
    let version_id = parse_uuid(&request.version_id, "version_id")?;
    let client_id = sqlx::query_scalar::<_, String>(
        r#"
        UPDATE developer_client_secret_versions
        SET status = 'revoked',
            revoked_at = NOW()
        WHERE tenant_id = $1
          AND id = $2
          AND revoked_at IS NULL
        RETURNING client_id
        "#,
    )
    .bind(tenant_id)
    .bind(version_id)
    .fetch_optional(db)
    .await
    .map_err(sql_status)?
    .ok_or_else(|| Status::not_found("developer secret was not found"))?;

    Ok(developer::SecretVersionRevocation {
        client_id,
        version_id: version_id.to_string(),
    })
}

async fn list_secret_version_rows(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
    client_id: &str,
) -> Result<Vec<developer::SecretVersion>, Status> {
    sqlx::query(
        r#"
        SELECT id, client_id, status::text AS status, secret_last4, created_at, expires_at, revoked_at
        FROM developer_client_secret_versions
        WHERE tenant_id = $1
          AND client_id = $2
        ORDER BY created_at DESC
        "#,
    )
    .bind(tenant_id)
    .bind(client_id)
    .fetch_all(db)
    .await
    .map_err(sql_status)
    .map(|rows| rows.into_iter().map(secret_version_from_row).collect())
}

async fn ensure_previous_overlap_version(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    client_id: &str,
    current_hash: &str,
    overlap_ends_at: DateTime<Utc>,
) -> Result<Uuid, Status> {
    if let Some(id) = sqlx::query_scalar::<_, Uuid>(
        r#"
        UPDATE developer_client_secret_versions
        SET status = 'overlap',
            expires_at = $3
        WHERE tenant_id = $1
          AND client_id = $2
          AND status = 'active'
          AND revoked_at IS NULL
        RETURNING id
        "#,
    )
    .bind(tenant_id)
    .bind(client_id)
    .bind(overlap_ends_at)
    .fetch_optional(&mut **tx)
    .await
    .map_err(sql_status)?
    {
        return Ok(id);
    }

    sqlx::query_scalar(
        r#"
        INSERT INTO developer_client_secret_versions (
          tenant_id,
          client_id,
          status,
          client_secret_hash,
          secret_last4,
          expires_at
        )
        VALUES ($1, $2, 'overlap', $3, 'unknown', $4)
        RETURNING id
        "#,
    )
    .bind(tenant_id)
    .bind(client_id)
    .bind(current_hash)
    .bind(overlap_ends_at)
    .fetch_one(&mut **tx)
    .await
    .map_err(sql_status)
}

fn secret_version_from_row(row: PgRow) -> developer::SecretVersion {
    developer::SecretVersion {
        id: row.get::<Uuid, _>("id").to_string(),
        client_id: row.get("client_id"),
        status: row.get("status"),
        secret_last4: row.get("secret_last4"),
        created_at: time_string(row.get("created_at")),
        expires_at: row
            .get::<Option<DateTime<Utc>>, _>("expires_at")
            .map(time_string)
            .unwrap_or_default(),
        revoked_at: row
            .get::<Option<DateTime<Utc>>, _>("revoked_at")
            .map(time_string)
            .unwrap_or_default(),
    }
}

fn parse_time(value: &str, field: &'static str) -> Result<DateTime<Utc>, Status> {
    DateTime::parse_from_rfc3339(value.trim())
        .map(|value| value.with_timezone(&Utc))
        .map_err(|_| Status::invalid_argument(format!("{field} must be an RFC3339 timestamp")))
}

fn time_string(value: DateTime<Utc>) -> String {
    value.to_rfc3339()
}
