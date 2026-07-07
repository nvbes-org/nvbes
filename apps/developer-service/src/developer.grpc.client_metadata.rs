use sqlx::{Row, postgres::PgRow};
use tonic::Status;

use crate::grpc::{
    pb::nvbes::developer::v1 as developer,
    service_status::{parse_uuid, sql_status},
};

pub async fn client_metadata(
    db: &sqlx::PgPool,
    request: developer::GetClientMetadataRequest,
) -> Result<developer::GetClientMetadataResponse, Status> {
    let tenant_id = parse_uuid(&request.tenant_id, "tenant_id")?;
    if request.client_ids.is_empty() {
        return Ok(developer::GetClientMetadataResponse {
            metadata: Vec::new(),
        });
    }

    let metadata = sqlx::query(
        r#"
        SELECT
          requested.client_id,
          m.status::text AS marketplace_status,
          cs.client_id IS NOT NULL AS consent_screen_configured,
          COALESCE(health.status, 'unknown') AS health_status
        FROM unnest($2::text[]) AS requested(client_id)
        LEFT JOIN developer_marketplace_apps m
          ON m.tenant_id = $1
         AND m.client_id = requested.client_id
        LEFT JOIN developer_consent_screens cs
          ON cs.tenant_id = $1
         AND cs.client_id = requested.client_id
        LEFT JOIN LATERAL (
          SELECT h.status::text AS status
          FROM developer_health_checks h
          WHERE h.tenant_id = $1
            AND h.target_type = 'oauth_client'
            AND h.target_id = requested.client_id
          ORDER BY h.checked_at DESC
          LIMIT 1
        ) health ON true
        "#,
    )
    .bind(tenant_id)
    .bind(&request.client_ids)
    .fetch_all(db)
    .await
    .map_err(sql_status)?
    .into_iter()
    .map(metadata_from_row)
    .collect();

    Ok(developer::GetClientMetadataResponse { metadata })
}

fn metadata_from_row(row: PgRow) -> developer::DeveloperClientMetadata {
    developer::DeveloperClientMetadata {
        client_id: row.get("client_id"),
        marketplace_status: row
            .get::<Option<String>, _>("marketplace_status")
            .unwrap_or_default(),
        consent_screen_configured: row.get("consent_screen_configured"),
        health_status: row.get("health_status"),
    }
}

#[cfg(test)]
#[path = "developer.grpc.client_metadata.contract_tests.rs"]
mod contract_tests;
