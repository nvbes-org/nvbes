use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{Duration, Utc};
use rand::RngCore;
use serde::Serialize;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth;

const INVITATION_TTL_DAYS: i64 = 7;

#[derive(Debug, Serialize)]
pub struct InvitationSyntheticResult {
    pub invited_principal_id: Uuid,
    pub invitation_accepted: bool,
    pub code_stored_as_hash: bool,
    pub second_acceptance_rejected: bool,
}

pub async fn run_synthetic_smoke(
    db: &PgPool,
    inviter_email: &str,
    invited_email: &str,
    password: &str,
) -> anyhow::Result<InvitationSyntheticResult> {
    let inviter = auth::create_synthetic_identity(db, inviter_email, password).await?;
    let invitation = create(db, inviter, invited_email).await?;
    let invited_principal_id = accept(db, invited_email, password, &invitation.code).await?;
    let second_acceptance_rejected = accept(db, invited_email, password, &invitation.code)
        .await
        .is_err();
    let code_stored_as_hash: bool = sqlx::query_scalar(
        "SELECT code_hash <> convert_to($2, 'UTF8') FROM identity_invitations WHERE id=$1",
    )
    .bind(invitation.id)
    .bind(invitation.code)
    .fetch_one(db)
    .await?;
    Ok(InvitationSyntheticResult {
        invited_principal_id,
        invitation_accepted: true,
        code_stored_as_hash,
        second_acceptance_rejected,
    })
}

struct InvitationSecret {
    id: Uuid,
    code: String,
}

async fn create(
    db: &PgPool,
    invited_by_principal_id: Uuid,
    invited_email: &str,
) -> anyhow::Result<InvitationSecret> {
    let email_hash = hash(&auth::normalize_email(invited_email)?);
    let code = random_code();
    let id = Uuid::new_v4();
    let mut tx = db.begin().await?;
    sqlx::query("UPDATE identity_invitations SET state='expired' WHERE email_hash=$1 AND state='pending' AND expires_at<=clock_timestamp()")
        .bind(email_hash.as_slice())
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO identity_invitations(id,invited_by_principal_id,email_hash,code_hash,state,expires_at) VALUES($1,$2,$3,$4,'pending',$5)")
        .bind(id)
        .bind(invited_by_principal_id)
        .bind(email_hash.as_slice())
        .bind(hash(&code).as_slice())
        .bind(Utc::now() + Duration::days(INVITATION_TTL_DAYS))
        .execute(&mut *tx)
        .await?;
    auth::audit(
        &mut tx,
        invited_by_principal_id,
        "identity.invitation.created",
    )
    .await?;
    tx.commit().await?;
    Ok(InvitationSecret { id, code })
}

async fn accept(db: &PgPool, email: &str, password: &str, code: &str) -> anyhow::Result<Uuid> {
    let email = auth::normalize_email(email)?;
    let mut tx = db.begin().await?;
    let invitation_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM identity_invitations WHERE code_hash=$1 AND email_hash=$2 AND state='pending' AND expires_at>clock_timestamp() FOR UPDATE",
    )
    .bind(hash(code).as_slice())
    .bind(hash(&email).as_slice())
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| anyhow::anyhow!("invitation is invalid or expired"))?;
    let principal_id =
        auth::create_active_human(&mut tx, &email, password, "identity.invitation.accepted")
            .await?;
    sqlx::query("UPDATE identity_invitations SET state='accepted',accepted_at=clock_timestamp(),accepted_principal_id=$2 WHERE id=$1 AND state='pending'")
        .bind(invitation_id)
        .bind(principal_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(principal_id)
}

fn random_code() -> String {
    let mut bytes = [0_u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn hash(value: &str) -> [u8; 32] {
    Sha256::digest(value.as_bytes()).into()
}
