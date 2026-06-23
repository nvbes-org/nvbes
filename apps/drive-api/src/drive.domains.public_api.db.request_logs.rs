use sqlx::PgPool;

use super::super::types::ApiRequestLogInsert;

pub async fn insert_api_request_log(
    db: &PgPool,
    input: ApiRequestLogInsert<'_>,
) -> Result<(), sqlx::Error> {
    let mut tx = db.begin().await?;
    let geo = crate::domains::public_api::geo::resolve_api_request_geo_tx(
        &mut tx,
        input.workspace_id,
        input.ip,
        input.request_id,
    )
    .await?;
    crate::domains::public_api::metrics::record_api_request_geo(input.status_code, &geo);

    sqlx::query(
        r#"
        INSERT INTO api_request_logs (
          workspace_id,
          api_key_id,
          actor_principal_id,
          request_id,
          method,
          path,
          status_code,
          error_code,
          scopes_used,
          ip,
          user_agent,
          geo_country_code,
          geo_source,
          geo_confidence,
          geo_network_kind,
          geo_risk_score,
          geo_risk_labels
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10::inet, $11, $12, $13, $14, $15, $16, $17)
        "#,
    )
    .bind(input.workspace_id)
    .bind(input.api_key_id)
    .bind(input.actor_principal_id)
    .bind(input.request_id)
    .bind(input.method)
    .bind(input.path)
    .bind(input.status_code)
    .bind(input.error_code)
    .bind(input.scopes_used)
    .bind(input.ip)
    .bind(input.user_agent)
    .bind(geo.country_code.as_deref())
    .bind(geo.source)
    .bind(geo.confidence)
    .bind(geo.network_kind)
    .bind(geo.risk_score)
    .bind(&geo.risk_labels)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}
