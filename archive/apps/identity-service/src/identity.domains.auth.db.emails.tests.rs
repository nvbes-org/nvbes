use nvbes_core::pagination::KeysetCursor;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use super::{
    change_unverified_primary_email, delete_secondary_email, fetch_email_address_for_update,
    insert_secondary_email, list_email_addresses, mark_secondary_verified, promote_secondary_email,
};

struct EmailFixture {
    db: PgPool,
    tenant_id: Uuid,
    principal_id: Uuid,
    primary_id: Uuid,
    primary_email: String,
}

impl EmailFixture {
    async fn seed() -> Option<Self> {
        let db = crate::test_support::shared_test_pool();
        if !crate::test_support::is_test_database_available(&db).await {
            eprintln!("skipping email fixture: database not available");
            return None;
        }
        crate::test_support::ensure_test_database(&db).await;
        let tenant_id = Uuid::new_v4();
        let principal_id = Uuid::new_v4();
        let primary_id = Uuid::new_v4();
        let primary_email = format!("primary-{principal_id}@example.test");

        sqlx::query(
            "INSERT INTO tenants (id, kind, name, slug, status) VALUES ($1, 'personal', 'Email test', $2, 'active')",
        )
        .bind(tenant_id)
        .bind(format!("email-{tenant_id}"))
        .execute(&db)
        .await
        .expect("tenant");
        sqlx::query(
            "INSERT INTO principals (id, tenant_id, principal_kind, status, display_name) VALUES ($1, $2, 'human', 'active', 'Email test')",
        )
        .bind(principal_id)
        .bind(tenant_id)
        .execute(&db)
        .await
        .expect("principal");
        sqlx::query(
            "INSERT INTO users (principal_id, email, email_verified_at, status) VALUES ($1, $2, NOW(), 'active')",
        )
        .bind(principal_id)
        .bind(&primary_email)
        .execute(&db)
        .await
        .expect("user");
        sqlx::query(
            "INSERT INTO user_email_addresses (id, principal_id, email, normalized_email, is_primary, verified_at) VALUES ($1, $2, $3, $3, TRUE, NOW())",
        )
        .bind(primary_id)
        .bind(principal_id)
        .bind(&primary_email)
        .execute(&db)
        .await
        .expect("primary email");

        Some(Self {
            db,
            tenant_id,
            principal_id,
            primary_id,
            primary_email,
        })
    }

    async fn insert_secondary(&self, email: &str) -> crate::domains::auth::types::EmailAddressView {
        let mut tx = self.db.begin().await.expect("transaction");
        let address = insert_secondary_email(&mut tx, self.principal_id, email)
            .await
            .expect("secondary email");
        tx.commit().await.expect("commit");
        address
    }

    async fn cleanup(self) {
        sqlx::query("DELETE FROM tenants WHERE id = $1")
            .bind(self.tenant_id)
            .execute(&self.db)
            .await
            .expect("fixture cleanup");
    }
}

#[tokio::test]
async fn email_listing_normalizes_secondary_addresses_and_honors_keyset_cursor() {
    let Some(fixture) = EmailFixture::seed().await else {
        return;
    };
    let secondary = fixture
        .insert_secondary("  Secondary.User@Example.TEST  ")
        .await;
    assert_eq!(secondary.email, "secondary.user@example.test");
    assert!(!secondary.is_primary);
    assert!(!secondary.verified);

    let first_page = list_email_addresses(&fixture.db, fixture.principal_id, None, 10)
        .await
        .expect("email listing");
    assert_eq!(first_page.len(), 2);

    let cursor = KeysetCursor {
        created_at: secondary.created_at,
        id: secondary.id,
    };
    let older = list_email_addresses(&fixture.db, fixture.principal_id, Some(&cursor), 10)
        .await
        .expect("cursor listing");
    assert!(older.iter().all(|address| address.id != secondary.id));

    let mut tx = fixture.db.begin().await.expect("transaction");
    let missing = fetch_email_address_for_update(&mut tx, fixture.principal_id, Uuid::new_v4())
        .await
        .expect_err("unknown email must fail");
    assert_eq!(missing.code, "email_address_not_found");
    tx.rollback().await.expect("rollback");
    fixture.cleanup().await;
}

#[tokio::test]
async fn primary_promotion_requires_verification_and_preserves_previous_primary() {
    let Some(fixture) = EmailFixture::seed().await else {
        return;
    };
    let secondary = fixture.insert_secondary("next@example.test").await;

    let mut tx = fixture.db.begin().await.expect("transaction");
    let unverified = promote_secondary_email(&mut tx, fixture.principal_id, secondary.id, 0)
        .await
        .expect_err("unverified address must fail");
    assert_eq!(unverified.code, "email_not_verified");
    tx.rollback().await.expect("rollback");

    mark_secondary_verified(&fixture.db, fixture.principal_id, secondary.id)
        .await
        .expect("verification");
    let mut tx = fixture.db.begin().await.expect("transaction");
    let (promoted, previous_email) =
        promote_secondary_email(&mut tx, fixture.principal_id, secondary.id, 0)
            .await
            .expect("promotion");
    tx.commit().await.expect("commit");
    assert!(promoted.is_primary);
    assert_eq!(promoted.email, "next@example.test");
    assert_eq!(previous_email, fixture.primary_email);

    let user_email: String = sqlx::query_scalar("SELECT email FROM users WHERE principal_id = $1")
        .bind(fixture.principal_id)
        .fetch_one(&fixture.db)
        .await
        .expect("user email");
    assert_eq!(user_email, promoted.email);
    let former_primary: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM user_email_addresses WHERE principal_id = $1 AND normalized_email = $2 AND is_primary = FALSE AND verified_at IS NOT NULL)",
    )
    .bind(fixture.principal_id)
    .bind(&fixture.primary_email)
    .fetch_one(&fixture.db)
    .await
    .expect("former primary");
    assert!(former_primary);
    fixture.cleanup().await;
}

#[tokio::test]
async fn promotion_enforces_minimum_age_and_primary_cannot_be_deleted() {
    let Some(fixture) = EmailFixture::seed().await else {
        return;
    };
    let secondary = fixture.insert_secondary("young@example.test").await;
    mark_secondary_verified(&fixture.db, fixture.principal_id, secondary.id)
        .await
        .expect("verification");

    let mut tx = fixture.db.begin().await.expect("transaction");
    let too_young = promote_secondary_email(&mut tx, fixture.principal_id, secondary.id, 24)
        .await
        .expect_err("new address must not promote");
    assert_eq!(too_young.code, "email_too_new_for_primary");
    tx.rollback().await.expect("rollback");

    let mut tx = fixture.db.begin().await.expect("transaction");
    let primary_delete = delete_secondary_email(&mut tx, fixture.principal_id, fixture.primary_id)
        .await
        .expect_err("primary email must not delete");
    assert_eq!(primary_delete.code, "primary_email_cannot_be_deleted");
    tx.rollback().await.expect("rollback");
    fixture.cleanup().await;
}

#[tokio::test]
async fn deleting_secondary_email_revokes_its_email_factor() {
    let Some(fixture) = EmailFixture::seed().await else {
        return;
    };
    let secondary = fixture.insert_secondary("factor@example.test").await;
    sqlx::query(
        "INSERT INTO mfa_factors (principal_id, factor_type, status, factor_data) VALUES ($1, 'email', 'active', jsonb_build_object('email_id', $2::text))",
    )
    .bind(fixture.principal_id)
    .bind(secondary.id)
    .execute(&fixture.db)
    .await
    .expect("email factor");

    let mut tx = fixture.db.begin().await.expect("transaction");
    delete_secondary_email(&mut tx, fixture.principal_id, secondary.id)
        .await
        .expect("secondary deletion");
    tx.commit().await.expect("commit");

    let row = sqlx::query(
        "SELECT e.deleted_at IS NOT NULL AS deleted, f.status::text AS factor_status FROM user_email_addresses e JOIN mfa_factors f ON f.principal_id = e.principal_id WHERE e.id = $1 AND f.factor_data->>'email_id' = $1::text",
    )
    .bind(secondary.id)
    .fetch_one(&fixture.db)
    .await
    .expect("deleted email state");
    assert!(row.get::<bool, _>("deleted"));
    assert_eq!(row.get::<String, _>("factor_status"), "revoked");
    fixture.cleanup().await;
}

#[tokio::test]
async fn unverified_primary_change_updates_identity_and_rejects_invalid_email() {
    let Some(fixture) = EmailFixture::seed().await else {
        return;
    };
    let mut tx = fixture.db.begin().await.expect("transaction");
    assert!(
        change_unverified_primary_email(&mut tx, fixture.principal_id, "not-an-email")
            .await
            .is_err()
    );
    change_unverified_primary_email(
        &mut tx,
        fixture.principal_id,
        "  Pending.Primary@Example.TEST ",
    )
    .await
    .expect("primary change");
    tx.commit().await.expect("commit");

    let row = sqlx::query(
        "SELECT email, status::text AS status, email_verified_at FROM users WHERE principal_id = $1",
    )
    .bind(fixture.principal_id)
    .fetch_one(&fixture.db)
    .await
    .expect("updated user");
    assert_eq!(
        row.get::<String, _>("email"),
        "pending.primary@example.test"
    );
    assert_eq!(row.get::<String, _>("status"), "pending_verification");
    assert!(
        row.get::<Option<chrono::DateTime<chrono::Utc>>, _>("email_verified_at")
            .is_none()
    );
    fixture.cleanup().await;
}
