use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{outbox, privacy_jobs, profile, teams};

#[derive(Debug, Serialize)]
pub struct SyntheticAccountResult {
    pub owner_principal_id: Uuid,
    pub member_principal_id: Uuid,
    pub team_id: Uuid,
    pub member_role: String,
    pub team_members: i64,
    pub member_left: bool,
    pub consent_granted: bool,
    pub export_completed: bool,
    pub closure_cancelled: bool,
    pub audit_events: i64,
    pub outbox_events: i64,
    pub outbox_published: bool,
}

pub async fn run(
    db: &PgPool,
    owner_principal_id: Uuid,
    member_principal_id: Uuid,
) -> anyhow::Result<SyntheticAccountResult> {
    profile::ensure_profile(db, owner_principal_id).await?;
    profile::ensure_profile(db, member_principal_id).await?;
    let (team_id, member_role) =
        teams::create_and_join_for_synthetic(db, owner_principal_id, member_principal_id).await?;
    let team_members: i64 =
        sqlx::query_scalar("SELECT count(*) FROM account_team_memberships WHERE team_id=$1")
            .bind(team_id)
            .fetch_one(db)
            .await?;

    let consent_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO account_consents(id, principal_id, consent_type, document_version) VALUES($1,$2,'terms','v1')",
    )
    .bind(consent_id)
    .bind(owner_principal_id)
    .execute(db)
    .await?;
    let consent_granted: bool = sqlx::query_scalar(
        "SELECT count(*) = 1 FROM account_consents WHERE id=$1 AND revoked_at IS NULL",
    )
    .bind(consent_id)
    .fetch_one(db)
    .await?;

    teams::leave_for_synthetic(db, team_id, member_principal_id).await?;
    let member_left: bool =
        sqlx::query_scalar("SELECT count(*) = 1 FROM account_team_memberships WHERE team_id=$1")
            .bind(team_id)
            .fetch_one(db)
            .await?;

    let export_id = Uuid::new_v4();
    sqlx::query("INSERT INTO account_exports(id,principal_id,status) VALUES($1,$2,'pending')")
        .bind(export_id)
        .bind(owner_principal_id)
        .execute(db)
        .await?;
    privacy_jobs::process_pending(db).await?;
    let export_completed: bool = sqlx::query_scalar(
        "SELECT status='completed' AND document IS NOT NULL AND document ? 'consents' FROM account_exports WHERE id=$1",
    )
    .bind(export_id)
    .fetch_one(db)
    .await?;

    let closure_id = Uuid::new_v4();
    sqlx::query("INSERT INTO account_closures(id,principal_id,status,execute_after) VALUES($1,$2,'cancelled',clock_timestamp()+interval '7 days')")
        .bind(closure_id)
        .bind(member_principal_id)
        .execute(db)
        .await?;
    let closure_cancelled: bool =
        sqlx::query_scalar("SELECT status='cancelled' FROM account_closures WHERE id=$1")
            .bind(closure_id)
            .fetch_one(db)
            .await?;
    let audit_events: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM account_audit_events WHERE principal_id IN ($1,$2)",
    )
    .bind(owner_principal_id)
    .bind(member_principal_id)
    .fetch_one(db)
    .await?;
    let outbox_events: i64 =
        sqlx::query_scalar("SELECT count(*) FROM account_outbox WHERE aggregate_id IN ($1,$2,$3)")
            .bind(owner_principal_id)
            .bind(member_principal_id)
            .bind(team_id)
            .fetch_one(db)
            .await?;
    let published = outbox::publish_pending(db, 100).await?;
    let pending: i64 =
        sqlx::query_scalar("SELECT count(*) FROM account_outbox WHERE published_at IS NULL")
            .fetch_one(db)
            .await?;
    let outbox_published = published > 0 && pending == 0;
    Ok(SyntheticAccountResult {
        owner_principal_id,
        member_principal_id,
        team_id,
        member_role,
        team_members,
        member_left,
        consent_granted,
        export_completed,
        closure_cancelled,
        audit_events,
        outbox_events,
        outbox_published,
    })
}
