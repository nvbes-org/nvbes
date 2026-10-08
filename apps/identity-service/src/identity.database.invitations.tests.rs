use sqlx::PgPool;
use uuid::Uuid;

use crate::invitations;

#[sqlx::test(migrations = "./migrations")]
async fn invitation_acceptance_is_one_time_and_never_persists_the_code(pool: PgPool) {
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
