use super::{
    OAuthClientKeyPurpose, active_jwks, ensure_client_owned, insert_initial_keys, map_key,
    validation,
};
use serde_json::json;
use sqlx::{PgPool, Row};
use uuid::Uuid;

struct KeyFixture {
    db: PgPool,
    tenant_id: Uuid,
    client_id: String,
}

impl KeyFixture {
    async fn seed(security_profile: &str) -> Self {
        let db = crate::test_support::shared_test_pool();
        crate::test_support::ensure_test_database(&db).await;
        let tenant_id = Uuid::new_v4();
        let client_id = format!("key-test-{tenant_id}");
        let secret_hash = nvbes_product_identity::oauth::hash_client_secret("")
            .expect("empty public-client secret should hash");

        sqlx::query(
            "INSERT INTO tenants (id, kind, name, slug, status) VALUES ($1, 'personal', 'Key test', $2, 'active')",
        )
        .bind(tenant_id)
        .bind(format!("key-test-{tenant_id}"))
        .execute(&db)
        .await
        .expect("tenant");
        sqlx::query(
            "INSERT INTO oauth_clients (client_id, client_secret_hash, name, redirect_uris, tenant_id, owner_scope_type, owner_scope_id, client_type, security_profile) VALUES ($1, $2, 'Key test', ARRAY['https://client.example.test/callback'], $3, 'tenant', $3, 'public', $4::oauth_security_profile)",
        )
        .bind(&client_id)
        .bind(secret_hash)
        .bind(tenant_id)
        .bind(security_profile)
        .execute(&db)
        .await
        .expect("OAuth client");

        Self {
            db,
            tenant_id,
            client_id,
        }
    }

    async fn cleanup(self) {
        sqlx::query("DELETE FROM tenants WHERE id = $1")
            .bind(self.tenant_id)
            .execute(&self.db)
            .await
            .expect("fixture cleanup");
    }
}

fn supported_rsa_jwk() -> serde_json::Value {
    json!({
        "kty": "RSA",
        "kid": "client-key-1",
        "alg": "RS256",
        "use": "sig",
        "n": "sXchDaQebHnPiGvyDOAT4saGEUetSyoQZBhXQjVhgWk",
        "e": "AQAB"
    })
}

#[test]
fn key_purpose_parsing_is_strict_and_round_trips() {
    for (value, expected) in [
        (
            "client_authentication",
            OAuthClientKeyPurpose::ClientAuthentication,
        ),
        ("request_object", OAuthClientKeyPurpose::RequestObject),
    ] {
        let purpose = OAuthClientKeyPurpose::parse(value).expect("supported purpose");
        assert_eq!(purpose, expected);
        assert_eq!(purpose.as_str(), value);
    }
    assert!(OAuthClientKeyPurpose::parse("signing").is_err());
}

#[test]
fn key_identifier_must_be_present_trimmed_and_bounded() {
    assert_eq!(
        validation::key_id(&json!({ "kid": " key-1 " })).expect("valid kid"),
        "key-1"
    );
    for jwk in [
        json!({}),
        json!({ "kid": "" }),
        json!({ "kid": "   " }),
        json!({ "kid": "x".repeat(256) }),
        json!({ "kid": 42 }),
    ] {
        assert!(validation::key_id(&jwk).is_err());
    }
}

#[test]
fn client_authentication_keys_accept_supported_public_material() {
    assert!(
        validation::validate_public_jwk(
            OAuthClientKeyPurpose::ClientAuthentication,
            &supported_rsa_jwk(),
            false,
        )
        .is_ok()
    );
}

#[test]
fn client_keys_reject_private_or_profile_incompatible_material() {
    for private_parameter in ["d", "p", "q", "dp", "dq", "qi", "oth", "k"] {
        let mut jwk = supported_rsa_jwk();
        jwk[private_parameter] = json!("private");
        assert!(
            validation::validate_public_jwk(
                OAuthClientKeyPurpose::ClientAuthentication,
                &jwk,
                false,
            )
            .is_err(),
            "private parameter {private_parameter} must be rejected"
        );
    }

    assert!(
        validation::validate_public_jwk(
            OAuthClientKeyPurpose::ClientAuthentication,
            &supported_rsa_jwk(),
            true,
        )
        .is_err()
    );
}

#[test]
fn request_object_keys_require_high_assurance_signing_material() {
    assert!(
        validation::validate_public_jwk(
            OAuthClientKeyPurpose::RequestObject,
            &supported_rsa_jwk(),
            false,
        )
        .is_err()
    );
}

#[tokio::test]
async fn initial_keys_are_queryable_and_retirement_honors_grace_and_immediate_revocation() {
    let fixture = KeyFixture::seed("standard").await;
    let jwk = supported_rsa_jwk();
    let mut tx = fixture.db.begin().await.expect("transaction");
    insert_initial_keys(
        &mut tx,
        fixture.tenant_id,
        &fixture.client_id,
        Some(&jwk),
        None,
        false,
    )
    .await
    .expect("initial key");
    tx.commit().await.expect("commit");

    let active = active_jwks(
        &fixture.db,
        fixture.tenant_id,
        &fixture.client_id,
        OAuthClientKeyPurpose::ClientAuthentication,
    )
    .await
    .expect("active JWKS");
    assert_eq!(active, json!({ "keys": [jwk] }));

    let mut tx = fixture.db.begin().await.expect("transaction");
    super::retire_current_keys(
        &mut tx,
        fixture.tenant_id,
        &fixture.client_id,
        OAuthClientKeyPurpose::ClientAuthentication,
        60,
    )
    .await
    .expect("graceful retirement");
    tx.commit().await.expect("commit");
    assert_eq!(
        active_jwks(
            &fixture.db,
            fixture.tenant_id,
            &fixture.client_id,
            OAuthClientKeyPurpose::ClientAuthentication,
        )
        .await
        .expect("retiring key remains usable")["keys"]
            .as_array()
            .expect("keys array")
            .len(),
        1
    );

    let mut tx = fixture.db.begin().await.expect("transaction");
    super::retire_current_keys(
        &mut tx,
        fixture.tenant_id,
        &fixture.client_id,
        OAuthClientKeyPurpose::ClientAuthentication,
        0,
    )
    .await
    .expect("immediate retirement");
    tx.commit().await.expect("commit");
    let inactive = active_jwks(
        &fixture.db,
        fixture.tenant_id,
        &fixture.client_id,
        OAuthClientKeyPurpose::ClientAuthentication,
    )
    .await
    .expect("inactive JWKS");
    assert_eq!(inactive, json!({ "keys": [] }));
    fixture.cleanup().await;
}

#[tokio::test]
async fn client_key_persistence_enforces_ownership_shape_and_maps_rows() {
    let fixture = KeyFixture::seed("standard").await;
    let mut tx = fixture.db.begin().await.expect("transaction");
    assert!(
        insert_initial_keys(
            &mut tx,
            fixture.tenant_id,
            &fixture.client_id,
            None,
            Some(&json!({ "not_keys": [] })),
            false,
        )
        .await
        .is_err()
    );
    assert!(
        !ensure_client_owned(&mut tx, fixture.tenant_id, &fixture.client_id)
            .await
            .expect("standard client")
    );
    let missing = ensure_client_owned(&mut tx, fixture.tenant_id, "missing-client")
        .await
        .expect_err("missing client");
    assert_eq!(missing.code, "oauth_client_not_found");

    super::insert_key(
        &mut tx,
        fixture.tenant_id,
        &fixture.client_id,
        OAuthClientKeyPurpose::ClientAuthentication,
        &supported_rsa_jwk(),
        false,
    )
    .await
    .expect("stored key");
    let row = sqlx::query(
        "SELECT id, purpose::text AS purpose, kid, jwk, status::text AS status, activated_at, retire_at, revoked_at FROM oauth_client_keys WHERE tenant_id = $1 AND client_id = $2",
    )
    .bind(fixture.tenant_id)
    .bind(&fixture.client_id)
    .fetch_one(&mut *tx)
    .await
    .expect("stored row");
    assert_eq!(row.get::<String, _>("kid"), "client-key-1");
    let view = map_key(&row);
    assert_eq!(view.purpose, "client_authentication");
    assert_eq!(view.status, "active");
    tx.commit().await.expect("commit");
    fixture.cleanup().await;
}
