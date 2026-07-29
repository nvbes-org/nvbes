use chrono::{DateTime, Utc};
use nvbes_storage::{ObjectStore, S3ObjectStore};
use serde::Serialize;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row, postgres::PgPoolOptions};
use uuid::Uuid;

#[path = "identity.worker.audit_anchor.kms.rs"]
mod kms;

use kms::{KmsSignResponse, sign_digest, verify_signature};

#[derive(Debug, Serialize)]
struct TenantChainHead {
    tenant_id: Uuid,
    event_count: i64,
    event_id: Uuid,
    event_hash: String,
    event_created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct ChainSnapshot {
    schema: &'static str,
    tenant_chains: Vec<TenantChainHead>,
}

#[derive(Debug, Serialize)]
struct SignedAnchor<'a> {
    schema: &'static str,
    generated_at: DateTime<Utc>,
    snapshot_digest_sha256: &'a str,
    signature: &'a str,
    signing_key_id: &'a str,
    event_count: i64,
    tenant_chains: &'a [TenantChainHead],
}

struct AuditAnchorConfig {
    region: String,
    kms_key_id: String,
    kms_auth_token: String,
    bucket: String,
    endpoint: String,
    access_key: String,
    secret_key: String,
}

impl AuditAnchorConfig {
    fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            region: required_env("NVBES_AUDIT_ANCHOR_REGION")?,
            kms_key_id: required_env("NVBES_AUDIT_ANCHOR_KMS_KEY_ID")?,
            kms_auth_token: required_env("NVBES_AUDIT_ANCHOR_KMS_AUTH_TOKEN")?,
            bucket: required_env("NVBES_AUDIT_ANCHOR_BUCKET")?,
            endpoint: required_env("NVBES_AUDIT_ANCHOR_S3_ENDPOINT")?,
            access_key: required_env("NVBES_AUDIT_ANCHOR_S3_ACCESS_KEY")?,
            secret_key: required_env("NVBES_AUDIT_ANCHOR_S3_SECRET_KEY")?,
        })
    }
}

pub async fn run_once(db: &PgPool) -> anyhow::Result<()> {
    let config = AuditAnchorConfig::from_env()?;
    let snapshot = load_chain_snapshot(db).await?;
    let snapshot_bytes = serde_json::to_vec(&snapshot)?;
    let digest = Sha256::digest(&snapshot_bytes);
    let digest_hex = hex::encode(&digest);

    if anchor_exists(db, &digest_hex).await? {
        tracing::info!(
            tenant_chain_count = snapshot.tenant_chains.len(),
            "audit chain has not changed since the last external anchor"
        );
        return Ok(());
    }

    let signature = sign_digest(&config, &digest[..]).await?;
    verify_signature(&config, &digest[..], &signature).await?;
    let generated_at = Utc::now();
    let event_count = snapshot
        .tenant_chains
        .iter()
        .map(|chain| chain.event_count)
        .sum();
    let anchor = SignedAnchor {
        schema: "nvbes.audit-anchor.v1",
        generated_at,
        snapshot_digest_sha256: &digest_hex,
        signature: &signature.signature,
        signing_key_id: &signature.key_id,
        event_count,
        tenant_chains: &snapshot.tenant_chains,
    };
    let object_key = format!(
        "anchors/{}/{digest_hex}.json",
        generated_at.format("%Y/%m/%d")
    );

    write_worm_anchor(&config, &object_key, serde_json::to_vec(&anchor)?).await?;
    persist_receipt(
        db,
        &digest_hex,
        &signature,
        &object_key,
        snapshot.tenant_chains.len(),
        event_count,
    )
    .await?;

    tracing::info!(
        tenant_chain_count = snapshot.tenant_chains.len(),
        event_count,
        signing_key_id = %signature.key_id,
        "signed audit anchor written to external immutable storage"
    );
    Ok(())
}

pub async fn run_standalone() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_target(false)
        .json()
        .flatten_event(true)
        .init();
    tracing::info!("starting standalone signed audit anchor job");

    let database_url = required_env("NVBES_DATABASE_URL")?;
    let db = PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await?;
    run_once(&db).await
}

async fn load_chain_snapshot(db: &PgPool) -> anyhow::Result<ChainSnapshot> {
    let rows = sqlx::query(
        r#"
        WITH chain_counts AS (
          SELECT tenant_id, COUNT(*)::BIGINT AS event_count
          FROM audit_events
          GROUP BY tenant_id
        )
        SELECT
          counts.tenant_id,
          counts.event_count,
          latest.id AS event_id,
          latest.event_hash,
          latest.created_at AS event_created_at
        FROM chain_counts counts
        JOIN LATERAL (
          SELECT id, event_hash, created_at
          FROM audit_events
          WHERE tenant_id = counts.tenant_id
          ORDER BY created_at DESC, id DESC
          LIMIT 1
        ) latest ON TRUE
        ORDER BY counts.tenant_id
        "#,
    )
    .fetch_all(db)
    .await?;

    let tenant_chains = rows
        .into_iter()
        .map(|row| {
            Ok(TenantChainHead {
                tenant_id: row.try_get("tenant_id")?,
                event_count: row.try_get("event_count")?,
                event_id: row.try_get("event_id")?,
                event_hash: row.try_get("event_hash")?,
                event_created_at: row.try_get("event_created_at")?,
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()?;

    Ok(ChainSnapshot {
        schema: "nvbes.audit-chain-snapshot.v1",
        tenant_chains,
    })
}

async fn anchor_exists(db: &PgPool, digest: &str) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM audit_external_anchors WHERE snapshot_digest = $1)",
    )
    .bind(digest)
    .fetch_one(db)
    .await
}

async fn write_worm_anchor(
    config: &AuditAnchorConfig,
    object_key: &str,
    body: Vec<u8>,
) -> anyhow::Result<()> {
    let store = S3ObjectStore::new(
        config.bucket.clone(),
        &config.endpoint,
        None,
        &config.region,
        &config.access_key,
        &config.secret_key,
    )
    .await;
    store
        .put_object(object_key, Some("application/json"), body)
        .await?;
    Ok(())
}

async fn persist_receipt(
    db: &PgPool,
    digest: &str,
    signature: &KmsSignResponse,
    object_key: &str,
    tenant_chain_count: usize,
    event_count: i64,
) -> anyhow::Result<()> {
    let tenant_chain_count = i64::try_from(tenant_chain_count)?;
    sqlx::query(
        r#"
        INSERT INTO audit_external_anchors (
          snapshot_digest,
          signature,
          signing_key_id,
          object_key,
          tenant_chain_count,
          event_count
        )
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (snapshot_digest) DO NOTHING
        "#,
    )
    .bind(digest)
    .bind(&signature.signature)
    .bind(&signature.key_id)
    .bind(object_key)
    .bind(tenant_chain_count)
    .bind(event_count)
    .execute(db)
    .await?;
    Ok(())
}

fn required_env(name: &str) -> anyhow::Result<String> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("{name} is required for signed audit anchoring"))
}

#[cfg(test)]
mod tests {
    use super::required_env;

    #[test]
    fn missing_anchor_configuration_fails_closed() {
        let name = "NVBES_TEST_MISSING_AUDIT_ANCHOR_VALUE";
        unsafe {
            std::env::remove_var(name);
        }
        assert!(required_env(name).is_err());
    }
}
