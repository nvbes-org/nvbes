use sqlx::{Row, postgres::PgRow};
use tonic::Status;
use uuid::Uuid;

use crate::grpc::{
    pb::nvbes::developer::v1 as developer,
    service_status::{parse_uuid, sql_status},
};

pub async fn list_credential_summaries(
    db: &sqlx::PgPool,
    request: developer::ListCredentialSummariesRequest,
) -> Result<developer::ListCredentialSummariesResponse, Status> {
    let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
    Ok(developer::ListCredentialSummariesResponse {
        credentials: list_credential_rows(db, tenant_id).await?,
    })
}

pub async fn count_stale_secrets(
    db: &sqlx::PgPool,
    request: developer::CountStaleSecretsRequest,
) -> Result<developer::CountStaleSecretsResponse, Status> {
    let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
    let stale_secret_count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)::bigint
        FROM developer_client_secret_versions dcsv
        WHERE dcsv.tenant_id = $1
          AND dcsv.revoked_at IS NULL
          AND (
            (dcsv.status = 'overlap' AND dcsv.expires_at <= NOW())
            OR (dcsv.status = 'active' AND dcsv.created_at <= NOW() - INTERVAL '90 days')
          )
        "#,
    )
    .bind(tenant_id)
    .fetch_one(db)
    .await
    .map_err(sql_status)?;

    Ok(developer::CountStaleSecretsResponse { stale_secret_count })
}

pub async fn verify_client_secret_version(
    db: &sqlx::PgPool,
    request: developer::VerifyClientSecretVersionRequest,
) -> Result<developer::VerifyClientSecretVersionResponse, Status> {
    let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
    let hashes = sqlx::query_scalar::<_, String>(
        r#"
        SELECT client_secret_hash
        FROM developer_client_secret_versions
        WHERE tenant_id = $1
          AND client_id = $2
          AND revoked_at IS NULL
          AND (
            status = 'active'
            OR (status = 'overlap' AND (expires_at IS NULL OR expires_at > NOW()))
          )
        "#,
    )
    .bind(tenant_id)
    .bind(request.client_id)
    .fetch_all(db)
    .await
    .map_err(sql_status)?;

    let valid = hashes.into_iter().any(|hash| {
        nvbes_product_account::oauth::verify_client_secret(&request.client_secret, &hash).is_ok()
    });

    Ok(developer::VerifyClientSecretVersionResponse { valid })
}

async fn list_credential_rows(
    db: &sqlx::PgPool,
    tenant_id: Uuid,
) -> Result<Vec<developer::DeveloperCredentialSummary>, Status> {
    sqlx::query(
        r#"
        SELECT dcsv.id,
          dcsv.client_id,
          CONCAT(oc.name, ' secret ', dcsv.secret_last4) AS name,
          dcsv.status::text AS status,
          dcsv.secret_last4,
          ''::text AS owner_email,
          COALESCE(policy.scopes, ARRAY[]::text[]) AS scopes,
          oc.last_used_at,
          dcsv.created_at,
          dcsv.expires_at
        FROM developer_client_secret_versions dcsv
        INNER JOIN oauth_clients oc
          ON oc.tenant_id = dcsv.tenant_id
         AND oc.client_id = dcsv.client_id
         AND oc.revoked_at IS NULL
        LEFT JOIN LATERAL (
          SELECT array_agg(DISTINCT scope) AS scopes
          FROM oauth_client_policies ocp
          CROSS JOIN LATERAL unnest(ocp.allowed_scopes) AS scope(scope)
          WHERE ocp.client_id = oc.id AND ocp.status = 'active'
        ) policy ON true
        WHERE dcsv.tenant_id = $1
          AND dcsv.revoked_at IS NULL
        UNION ALL
        SELECT oc.id,
          oc.client_id,
          oc.name,
          'client'::text AS status,
          'unknown'::text AS secret_last4,
          ''::text AS owner_email,
          COALESCE(policy.scopes, ARRAY[]::text[]) AS scopes,
          oc.last_used_at,
          oc.created_at,
          NULL::timestamptz AS expires_at
        FROM oauth_clients oc
        LEFT JOIN LATERAL (
          SELECT array_agg(DISTINCT scope) AS scopes
          FROM oauth_client_policies ocp
          CROSS JOIN LATERAL unnest(ocp.allowed_scopes) AS scope(scope)
          WHERE ocp.client_id = oc.id AND ocp.status = 'active'
        ) policy ON true
        WHERE oc.tenant_id = $1
          AND oc.revoked_at IS NULL
          AND NOT EXISTS (
            SELECT 1
            FROM developer_client_secret_versions dcsv
            WHERE dcsv.tenant_id = oc.tenant_id
              AND dcsv.client_id = oc.client_id
          )
        ORDER BY created_at DESC
        "#,
    )
    .bind(tenant_id)
    .fetch_all(db)
    .await
    .map_err(sql_status)
    .map(|rows| rows.into_iter().map(credential_from_row).collect())
}

fn credential_from_row(row: PgRow) -> developer::DeveloperCredentialSummary {
    developer::DeveloperCredentialSummary {
        id: row.get::<Uuid, _>("id").to_string(),
        client_id: row.get("client_id"),
        name: row.get("name"),
        status: row.get("status"),
        secret_last4: row.get("secret_last4"),
        owner_email: row.get("owner_email"),
        scopes: row.get("scopes"),
        last_used_at: row
            .get::<Option<chrono::DateTime<chrono::Utc>>, _>("last_used_at")
            .map(|time| time.to_rfc3339())
            .unwrap_or_default(),
        created_at: row
            .get::<chrono::DateTime<chrono::Utc>, _>("created_at")
            .to_rfc3339(),
        expires_at: row
            .get::<Option<chrono::DateTime<chrono::Utc>>, _>("expires_at")
            .map(|time| time.to_rfc3339())
            .unwrap_or_default(),
    }
}

#[cfg(test)]
#[path = "developer.grpc.credentials.contract_tests.rs"]
mod contract_tests;
