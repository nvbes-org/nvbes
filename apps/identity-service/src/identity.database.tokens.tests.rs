use rsa::{
    RsaPrivateKey,
    pkcs8::{EncodePrivateKey, EncodePublicKey},
    rand_core::OsRng,
};
use uuid::Uuid;

use crate::{auth, tokens::TokenService, tokens_config::TokenConfig};

use super::{connect, migrate};

#[tokio::test]
async fn session_issued_tokens_derive_step_up_amr_and_survive_step_up_expiry() {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL is required");
    let pool = connect(&database_url, 2)
        .await
        .expect("test database connects");
    migrate(&pool).await.expect("identity migrations apply");
    let private = RsaPrivateKey::new(&mut OsRng, 2048).unwrap();
    let public = private.to_public_key();
    let service = TokenService::new(
        TokenConfig::from_values(
            "test",
            "http://identity.test".into(),
            "identity-key-1".into(),
            private
                .to_pkcs8_pem(Default::default())
                .unwrap()
                .to_string(),
            public.to_public_key_pem(Default::default()).unwrap(),
            "nvbes-account-service,nvbes-billing-service".into(),
        )
        .unwrap(),
    )
    .unwrap();
    let email = format!("synthetic-amr-{}@example.invalid", Uuid::new_v4());
    let principal_id = auth::create_synthetic_identity(&pool, &email, "Synthetic-amr-password!")
        .await
        .unwrap();
    let session_token = auth::authenticate(&pool, &email, "Synthetic-amr-password!")
        .await
        .unwrap();
    let session_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM identity_sessions WHERE token_hash=$1 AND principal_id=$2",
    )
    .bind(auth::hash_token(&session_token))
    .bind(principal_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(
        claims(&service, &pool, session_id, "account:read")
            .await
            .amr,
        ["pwd"]
    );
    sqlx::query("UPDATE identity_sessions SET step_up_expires_at=clock_timestamp()+interval '10 minutes',step_up_method='webauthn' WHERE id=$1")
        .bind(session_id)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        claims(&service, &pool, session_id, "account:close")
            .await
            .amr,
        ["pwd", "webauthn"]
    );
    sqlx::query("UPDATE identity_sessions SET step_up_expires_at=clock_timestamp()-interval '1 second' WHERE id=$1")
        .bind(session_id)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        claims(&service, &pool, session_id, "account:read")
            .await
            .amr,
        ["pwd"]
    );
}

async fn claims(
    service: &TokenService,
    pool: &sqlx::PgPool,
    session_id: Uuid,
    scope: &str,
) -> crate::tokens::AccessTokenClaims {
    let token = service
        .issue_for_active_session(pool, session_id, "nvbes-account-service", scope)
        .await
        .unwrap();
    service.verify(&token, "nvbes-account-service").unwrap()
}
