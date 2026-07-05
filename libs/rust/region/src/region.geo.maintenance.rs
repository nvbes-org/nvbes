use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, Transaction};

use crate::geo::maxmind_types::MAXMIND_GEOLITE_SOURCE_CODES;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeoMaintenanceReport {
    pub expired_personal_ranges_disabled: u64,
    pub expired_unreferenced_relations_deleted: u64,
    pub expired_maxmind_evidence_scrubbed: u64,
    pub expired_maxmind_relations_deleted: u64,
}

pub async fn run_geo_maintenance(pool: &PgPool) -> Result<GeoMaintenanceReport, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let report = run_geo_maintenance_tx(&mut tx).await?;
    tx.commit().await?;
    Ok(report)
}

pub async fn run_geo_maintenance_tx(
    tx: &mut Transaction<'_, Postgres>,
) -> Result<GeoMaintenanceReport, sqlx::Error> {
    let expired_personal_ranges_disabled = sqlx::query(
        r#"
        UPDATE geo_personal_ip_ranges
        SET enabled = FALSE,
            updated_at = NOW()
        WHERE enabled = TRUE
          AND expires_at IS NOT NULL
          AND expires_at <= NOW()
        "#,
    )
    .execute(&mut **tx)
    .await?
    .rows_affected();

    let expired_unreferenced_relations_deleted = sqlx::query(
        r#"
        DELETE FROM geo_ip_network_relations relation
        WHERE relation.expires_at IS NOT NULL
          AND relation.expires_at <= NOW()
          AND NOT EXISTS (
              SELECT 1
              FROM geo_lookup_evidence evidence
              WHERE evidence.network_relation_id = relation.id
          )
        "#,
    )
    .execute(&mut **tx)
    .await?
    .rows_affected();

    let expired_maxmind_evidence_scrubbed = sqlx::query(
        r#"
        UPDATE geo_lookup_evidence evidence
        SET network_relation_id = NULL,
            country_code = NULL,
            accepted = FALSE,
            reason = 'maxmind geolite evidence expired per license retention',
            relation_snapshot = 'null'::jsonb
        FROM geo_ip_network_relations relation
        WHERE evidence.network_relation_id = relation.id
          AND relation.source_code = ANY($1::TEXT[])
          AND relation.expires_at IS NOT NULL
          AND relation.expires_at <= NOW()
        "#,
    )
    .bind(MAXMIND_GEOLITE_SOURCE_CODES.as_slice())
    .execute(&mut **tx)
    .await?
    .rows_affected();

    let expired_maxmind_relations_deleted = sqlx::query(
        r#"
        DELETE FROM geo_ip_network_relations relation
        WHERE relation.source_code = ANY($1::TEXT[])
          AND relation.expires_at IS NOT NULL
          AND relation.expires_at <= NOW()
        "#,
    )
    .bind(MAXMIND_GEOLITE_SOURCE_CODES.as_slice())
    .execute(&mut **tx)
    .await?
    .rows_affected();

    Ok(GeoMaintenanceReport {
        expired_personal_ranges_disabled,
        expired_unreferenced_relations_deleted,
        expired_maxmind_evidence_scrubbed,
        expired_maxmind_relations_deleted,
    })
}
