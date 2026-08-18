use std::sync::Mutex;

use chrono::{Duration, Utc};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use super::{
    AuditAnchorConfig, AuditAnchorExternal, KmsSignResponse, anchor_exists, load_chain_snapshot,
    persist_receipt, required_env, run_once, run_once_with,
};
use crate::worker::test_support::database_pool;

static ENV_LOCK: Mutex<()> = Mutex::new(());
const ENVIRONMENT: [(&str, &str); 7] = [
    ("NVBES_AUDIT_ANCHOR_REGION", "fr-par"),
    ("NVBES_AUDIT_ANCHOR_KMS_KEY_ID", "kms-key"),
    ("NVBES_AUDIT_ANCHOR_KMS_AUTH_TOKEN", "kms-token"),
    ("NVBES_AUDIT_ANCHOR_BUCKET", "audit-bucket"),
    ("NVBES_AUDIT_ANCHOR_S3_ENDPOINT", "http://127.0.0.1:8333"),
    ("NVBES_AUDIT_ANCHOR_S3_ACCESS_KEY", "access-key"),
    ("NVBES_AUDIT_ANCHOR_S3_SECRET_KEY", "secret-key"),
];
const TENANT_PREFIX: &str = "identity-worker-audit-test";

#[test]
fn required_configuration_rejects_missing_and_blank_values() {
    let _guard = ENV_LOCK.lock().expect("environment lock");
    let name = "NVBES_TEST_AUDIT_ANCHOR_REQUIRED";
    remove_env(name);
    assert!(
        required_env(name)
            .expect_err("missing")
            .to_string()
            .contains(name)
    );
    set_env(name, "   ");
    assert!(required_env(name).is_err());
    set_env(name, "configured");
    assert_eq!(required_env(name).expect("value"), "configured");
    remove_env(name);
}

#[test]
fn audit_anchor_config_loads_every_required_boundary_value() {
    let _guard = ENV_LOCK.lock().expect("environment lock");
    clear_environment();
    assert!(AuditAnchorConfig::from_env().is_err());
    for (name, value) in ENVIRONMENT {
        set_env(name, value);
    }

    let config = AuditAnchorConfig::from_env().expect("anchor config");

    assert_eq!(config.region, "fr-par");
    assert_eq!(config.kms_key_id, "kms-key");
    assert_eq!(config.kms_auth_token, "kms-token");
    assert_eq!(config.bucket, "audit-bucket");
    assert_eq!(config.endpoint, "http://127.0.0.1:8333");
    assert_eq!(config.access_key, "access-key");
    assert_eq!(config.secret_key, "secret-key");
    clear_environment();
}

#[tokio::test]
async fn snapshot_selects_each_tenants_latest_chain_head_in_stable_order() {
    let db = database_pool().await;
    let tenant_b = insert_tenant(&db, "b").await;
    let tenant_a = insert_tenant(&db, "a").await;
    let old = insert_audit_event(&db, tenant_a, "hash-old", -60).await;
    let latest = insert_audit_event(&db, tenant_a, "hash-latest", -10).await;
    let only = insert_audit_event(&db, tenant_b, "hash-only", -30).await;

    let snapshot = load_chain_snapshot(&db).await.expect("snapshot");

    assert_eq!(snapshot.schema, "nvbes.audit-chain-snapshot.v1");
    assert!(snapshot.tenant_chains.len() >= 2);
    let chain_a = snapshot
        .tenant_chains
        .iter()
        .find(|chain| chain.tenant_id == tenant_a)
        .expect("tenant A");
    assert_eq!(chain_a.event_count, 2);
    assert_eq!(chain_a.event_id, latest);
    assert!(!chain_a.event_hash.is_empty());
    assert!(chain_a.event_created_at > Utc::now() - Duration::seconds(20));
    let chain_b = snapshot
        .tenant_chains
        .iter()
        .find(|chain| chain.tenant_id == tenant_b)
        .expect("tenant B");
    assert_eq!(chain_b.event_count, 1);
    assert_eq!(chain_b.event_id, only);
    assert_ne!(old, latest);
}

#[tokio::test]
async fn receipt_persistence_is_idempotent_and_discoverable_by_digest() {
    let db = database_pool().await;
    let digest = format!("digest-{}", Uuid::new_v4());
    let object_key = format!("anchors/{digest}.json");
    let signature = KmsSignResponse {
        key_id: "kms-key".to_string(),
        signature: "signature".to_string(),
    };
    assert!(!anchor_exists(&db, &digest).await.expect("absence"));

    persist_receipt(&db, &digest, &signature, &object_key, 2, 7)
        .await
        .expect("receipt");
    persist_receipt(&db, &digest, &signature, &object_key, 2, 7)
        .await
        .expect("idempotent receipt");

    assert!(anchor_exists(&db, &digest).await.expect("presence"));
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_external_anchors WHERE snapshot_digest = $1",
    )
    .bind(&digest)
    .fetch_one(&db)
    .await
    .expect("receipt count");
    assert_eq!(count, 1);
}

#[tokio::test]
async fn unchanged_snapshot_short_circuits_before_kms_and_object_storage() {
    let _guard = ENV_LOCK.lock().expect("environment lock");
    let db = database_pool().await;
    for (name, value) in ENVIRONMENT {
        set_env(name, value);
    }
    let snapshot = load_chain_snapshot(&db).await.expect("empty snapshot");
    let digest = hex::encode(Sha256::digest(
        serde_json::to_vec(&snapshot).expect("snapshot JSON"),
    ));
    persist_receipt(
        &db,
        &digest,
        &KmsSignResponse {
            key_id: "kms-key".to_string(),
            signature: "signature".to_string(),
        },
        &format!("anchors/{digest}.json"),
        0,
        0,
    )
    .await
    .expect("existing receipt");

    run_once(&db).await.expect("unchanged anchor");

    clear_environment();
}

#[tokio::test]
async fn changed_snapshot_is_signed_verified_written_and_receipted_end_to_end() {
    let db = database_pool().await;
    let tenant = insert_tenant(&db, "full-anchor").await;
    insert_audit_event(&db, tenant, "full-anchor-event", 0).await;
    let external = RecordingExternal::default();

    run_once_with(&db, &external).await.expect("full anchor");

    assert_eq!(*external.sign_calls.lock().expect("sign calls"), 1);
    assert_eq!(*external.verify_calls.lock().expect("verify calls"), 1);
    let writes = external.writes.lock().expect("writes");
    assert_eq!(writes.len(), 1);
    assert!(writes[0].0.starts_with("anchors/"));
    let anchor: serde_json::Value = serde_json::from_slice(&writes[0].1).expect("anchor JSON");
    assert_eq!(anchor["schema"], "nvbes.audit-anchor.v1");
    assert_eq!(anchor["signing_key_id"], "test-kms-key");
    assert_eq!(anchor["signature"], "test-signature");
    let digest = anchor["snapshot_digest_sha256"].as_str().expect("digest");
    assert!(anchor_exists(&db, digest).await.expect("receipt"));
}

#[tokio::test]
async fn signing_failure_stops_before_write_and_receipt() {
    let db = database_pool().await;
    let tenant = insert_tenant(&db, "signing-failure").await;
    insert_audit_event(&db, tenant, "signing-failure-event", 0).await;
    let external = RecordingExternal {
        fail_signing: true,
        ..RecordingExternal::default()
    };

    let error = run_once_with(&db, &external)
        .await
        .expect_err("signing failure");

    assert!(error.to_string().contains("test signing failure"));
    assert!(external.writes.lock().expect("writes").is_empty());
    assert_eq!(*external.verify_calls.lock().expect("verify calls"), 0);
}

#[derive(Default)]
struct RecordingExternal {
    sign_calls: Mutex<usize>,
    verify_calls: Mutex<usize>,
    writes: Mutex<Vec<(String, Vec<u8>)>>,
    fail_signing: bool,
}

impl AuditAnchorExternal for RecordingExternal {
    async fn sign(&self, digest: &[u8]) -> anyhow::Result<KmsSignResponse> {
        assert_eq!(digest.len(), 32);
        *self.sign_calls.lock().expect("sign calls") += 1;
        if self.fail_signing {
            anyhow::bail!("test signing failure");
        }
        Ok(KmsSignResponse {
            key_id: "test-kms-key".to_string(),
            signature: "test-signature".to_string(),
        })
    }

    async fn verify(&self, digest: &[u8], signature: &KmsSignResponse) -> anyhow::Result<()> {
        assert_eq!(digest.len(), 32);
        assert_eq!(signature.key_id, "test-kms-key");
        *self.verify_calls.lock().expect("verify calls") += 1;
        Ok(())
    }

    async fn write(&self, object_key: &str, body: Vec<u8>) -> anyhow::Result<()> {
        self.writes
            .lock()
            .expect("writes")
            .push((object_key.to_string(), body));
        Ok(())
    }
}

async fn insert_tenant(db: &PgPool, suffix: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO tenants (id, name, slug) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(format!("Audit test {suffix}"))
        .bind(format!("{TENANT_PREFIX}-{suffix}-{id}"))
        .execute(db)
        .await
        .expect("tenant");
    id
}

async fn insert_audit_event(db: &PgPool, tenant_id: Uuid, hash: &str, age_seconds: i64) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO audit_events
           (id, tenant_id, action, target_type, event_hash, created_at)
           VALUES ($1, $2, 'identity.test', 'test', $3, NOW() + make_interval(secs => $4))"#,
    )
    .bind(id)
    .bind(tenant_id)
    .bind(hash)
    .bind(age_seconds as f64)
    .execute(db)
    .await
    .expect("audit event");
    id
}

fn clear_environment() {
    for (name, _) in ENVIRONMENT {
        remove_env(name);
    }
}

fn set_env(name: &str, value: &str) {
    unsafe { std::env::set_var(name, value) };
}

fn remove_env(name: &str) {
    unsafe { std::env::remove_var(name) };
}
