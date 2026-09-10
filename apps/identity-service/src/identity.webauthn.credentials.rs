use super::{WebauthnError, valid_credential_label};
use crate::oauth::store;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

#[derive(serde::Serialize, sqlx::FromRow)]
pub struct CredentialSummary {
    pub id: Uuid,
    pub label: String,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

pub async fn list(db: &PgPool, token: &str) -> Result<Vec<CredentialSummary>, WebauthnError> {
    let mut tx = db.begin().await?;
    let principal = owner(&mut tx, token, false).await?;
    let rows=sqlx::query_as("SELECT id,label,created_at,last_used_at FROM identity_webauthn_credentials WHERE principal_id=$1 AND revoked_at IS NULL ORDER BY created_at,id LIMIT 10")
        .bind(principal).fetch_all(&mut *tx).await?;
    tx.commit().await?;
    Ok(rows)
}

pub async fn rename(db: &PgPool, token: &str, id: Uuid, label: &str) -> Result<(), WebauthnError> {
    if !valid_credential_label(label) {
        return Err(WebauthnError::InvalidCeremony);
    }
    let mut tx = db.begin().await?;
    let principal = owner(&mut tx, token, true).await?;
    let current:Option<String>=sqlx::query_scalar("SELECT label FROM identity_webauthn_credentials WHERE id=$1 AND principal_id=$2 AND revoked_at IS NULL FOR UPDATE")
        .bind(id).bind(principal).fetch_optional(&mut *tx).await?;
    let current = current.ok_or(WebauthnError::InvalidCeremony)?;
    if current != label {
        sqlx::query("UPDATE identity_webauthn_credentials SET label=$1 WHERE id=$2")
            .bind(label)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        store::audit(&mut tx, principal, "identity.webauthn.renamed").await?;
    }
    tx.commit().await?;
    Ok(())
}

/// Revokes future uses of a credential, including already-started ceremonies.
/// Previously authenticated sessions retain their separate lifecycle.
pub async fn revoke(db: &PgPool, token: &str, id: Uuid) -> Result<(), WebauthnError> {
    let mut tx = db.begin().await?;
    let principal = owner(&mut tx, token, true).await?;
    let revoked:Option<bool>=sqlx::query_scalar("SELECT revoked_at IS NOT NULL FROM identity_webauthn_credentials WHERE id=$1 AND principal_id=$2 FOR UPDATE")
        .bind(id).bind(principal).fetch_optional(&mut *tx).await?;
    if !revoked.ok_or(WebauthnError::InvalidCeremony)? {
        // The principal lock serializes concurrent revocations/enrollments.
        let alternative:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM identity_webauthn_credentials WHERE principal_id=$1 AND id<>$2 AND revoked_at IS NULL AND passkey IS NOT NULL) OR EXISTS(SELECT 1 FROM identity_auth_factors WHERE principal_id=$1 AND kind='totp' AND state='active')")
            .bind(principal).bind(id).fetch_one(&mut *tx).await?;
        if !alternative {
            return Err(WebauthnError::LastFactor);
        }
        sqlx::query(
            "UPDATE identity_webauthn_credentials SET revoked_at=clock_timestamp() WHERE id=$1",
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;
        store::audit(&mut tx, principal, "identity.webauthn.revoked").await?;
    }
    tx.commit().await?;
    Ok(())
}

async fn owner(
    tx: &mut Transaction<'_, Postgres>,
    token: &str,
    strong: bool,
) -> Result<Uuid, WebauthnError> {
    let row:Option<Uuid>=sqlx::query_scalar("SELECT s.principal_id FROM identity_sessions s JOIN identity_principals p ON p.id=s.principal_id WHERE s.token_hash=$1 AND s.revoked_at IS NULL AND s.expires_at>clock_timestamp() AND p.status='active' AND (NOT $2 OR (s.primary_amr='webauthn' AND s.authenticated_at>clock_timestamp()-interval '5 minutes') OR (s.step_up_method IN ('totp','webauthn') AND s.step_up_at>clock_timestamp()-interval '5 minutes' AND s.step_up_expires_at>clock_timestamp())) FOR UPDATE OF s,p")
        .bind(store::hash(token)).bind(strong).fetch_optional(&mut **tx).await?;
    row.ok_or(WebauthnError::InvalidSession)
}

#[cfg(all(test, feature = "database-tests"))]
#[path = "identity.webauthn.credentials.tests.rs"]
mod tests;
