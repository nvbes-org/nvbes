use uuid::Uuid;

use crate::invitations;

use super::{connect, migrate};

#[tokio::test]
async fn invitation_acceptance_is_one_time_and_never_persists_the_code() {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL is required");
    let pool = connect(&database_url, 2)
        .await
        .expect("test database connects");
    migrate(&pool).await.expect("identity migrations apply");
    let result = invitations::run_synthetic_smoke(
        &pool,
        &format!("inviter-{}@example.invalid", Uuid::new_v4()),
        &format!("invited-{}@example.invalid", Uuid::new_v4()),
        "Synthetic-invitation-password!",
    )
    .await
    .expect("invitation lifecycle succeeds");

    assert!(result.invitation_accepted);
    assert!(result.code_stored_as_hash);
    assert!(result.second_acceptance_rejected);
}
