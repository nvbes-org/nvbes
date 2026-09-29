use serde::Serialize;
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    audit::{self, AuditInput},
    error::AccountResult,
};

#[derive(Debug, Serialize)]
pub struct PrivacyJobResult {
    pub(crate) exports_completed: u64,
    pub(crate) closures_completed: u64,
}

pub async fn process_pending(db: &PgPool) -> AccountResult<PrivacyJobResult> {
    sqlx::query("UPDATE account_exports SET status='expired',document=NULL,updated_at=clock_timestamp() WHERE status='completed' AND expires_at<=clock_timestamp()")
        .execute(db)
        .await?;
    let exports = sqlx::query_as::<_, (Uuid, Uuid)>("UPDATE account_exports SET status='processing',updated_at=clock_timestamp() WHERE id IN (SELECT id FROM account_exports WHERE status='pending' ORDER BY requested_at FOR UPDATE SKIP LOCKED LIMIT 20) RETURNING id,principal_id")
        .fetch_all(db)
        .await?;
    let mut exports_completed = 0;
    for (id, principal_id) in exports {
        complete_export(db, id, principal_id).await?;
        exports_completed += 1;
    }
    let closures = sqlx::query_as::<_, (Uuid, Uuid)>("UPDATE account_closures SET status='processing',updated_at=clock_timestamp() WHERE id IN (SELECT id FROM account_closures WHERE status='pending' AND execute_after<=clock_timestamp() ORDER BY execute_after FOR UPDATE SKIP LOCKED LIMIT 20) RETURNING id,principal_id")
        .fetch_all(db)
        .await?;
    let mut closures_completed = 0;
    for (id, principal_id) in closures {
        complete_closure(db, id, principal_id).await?;
        closures_completed += 1;
    }
    Ok(PrivacyJobResult {
        exports_completed,
        closures_completed,
    })
}

async fn complete_export(db: &PgPool, id: Uuid, principal_id: Uuid) -> AccountResult<()> {
    let document: Value = sqlx::query_scalar(
        "SELECT jsonb_build_object(
            'exported_at', clock_timestamp(),
            'principal_id', $1,
            'profile', (SELECT to_jsonb(p)-'lifecycle_status'-'closed_at' FROM account_profiles p WHERE principal_id=$1),
            'preferences', (SELECT to_jsonb(p) FROM account_preferences p WHERE principal_id=$1),
            'consents', COALESCE((SELECT jsonb_agg(jsonb_build_object('id',c.id,'consent_type',c.consent_type,'document_version',c.document_version,'granted_at',c.granted_at,'revoked_at',c.revoked_at) ORDER BY c.granted_at DESC, c.id DESC) FROM account_consents c WHERE c.principal_id=$1), '[]'::jsonb),
            'teams', COALESCE((SELECT jsonb_agg(jsonb_build_object('id',t.id,'name',t.name,'role',m.role,'created_at',t.created_at)) FROM account_team_memberships m JOIN account_teams t ON t.id=m.team_id WHERE m.principal_id=$1), '[]'::jsonb)
        )",
    )
    .bind(principal_id)
    .fetch_one(db)
    .await?;
    sqlx::query("UPDATE account_exports SET status='completed',document=$2,completed_at=clock_timestamp(),expires_at=clock_timestamp()+interval '24 hours',updated_at=clock_timestamp() WHERE id=$1 AND status='processing'")
        .bind(id)
        .bind(document)
        .execute(db)
        .await?;
    Ok(())
}

async fn complete_closure(db: &PgPool, id: Uuid, principal_id: Uuid) -> AccountResult<()> {
    let mut tx = db.begin().await?;
    sqlx::query("DELETE FROM account_teams WHERE owner_principal_id=$1")
        .bind(principal_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM account_team_memberships WHERE principal_id=$1")
        .bind(principal_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM account_consents WHERE principal_id=$1")
        .bind(principal_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM account_preferences WHERE principal_id=$1")
        .bind(principal_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE account_exports SET document=NULL,status='expired',updated_at=clock_timestamp() WHERE principal_id=$1 AND status='completed'")
        .bind(principal_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE account_profiles SET firstname=NULL,lastname=NULL,username=NULL,birthdate=NULL,region=NULL,lifecycle_status='closed',closed_at=clock_timestamp(),updated_at=clock_timestamp() WHERE principal_id=$1")
        .bind(principal_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE account_closures SET status='completed',completed_at=clock_timestamp(),updated_at=clock_timestamp() WHERE id=$1 AND status='processing'")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    audit::record(
        &mut tx,
        AuditInput {
            principal_id,
            actor_principal_id: principal_id,
            event_type: "account.closure.completed",
            resource_type: "closure",
            resource_id: Some(id),
            correlation_id: Uuid::new_v4(),
            details: json!({"account_data_redacted": true}),
        },
    )
    .await?;
    audit::enqueue(
        &mut tx,
        "account.closure.completed.v1",
        id,
        json!({"saga_id": id, "principal_id": principal_id}),
    )
    .await?;
    tx.commit().await?;
    Ok(())
}
