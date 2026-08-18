use sqlx::{PgPool, Postgres, Transaction};

use crate::{
    error::AppError, profile_models::AccountProfileRow, registration_models::RegistrationProjection,
};

pub async fn apply(db: &PgPool, projection: RegistrationProjection) -> Result<bool, AppError> {
    let mut tx = db.begin().await?;
    let fingerprint = event_fingerprint(&projection)?;
    let inserted = sqlx::query(
        r#"
        INSERT INTO account_inbox_events (
          event_id, event_type, principal_id, event_fingerprint
        )
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (event_id) DO NOTHING
        "#,
    )
    .bind(projection.event_id)
    .bind(&projection.event_type)
    .bind(projection.principal_id)
    .bind(&fingerprint)
    .execute(&mut *tx)
    .await?
    .rows_affected()
        == 1;

    if !inserted {
        ensure_matching_replay(&mut tx, &projection, &fingerprint).await?;
        tx.commit().await?;
        return Ok(false);
    }

    let closing: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
          SELECT 1 FROM account_closure_sagas
          WHERE principal_id = $1 AND status IN ('pending', 'dispatching', 'completed')
        )
        "#,
    )
    .bind(projection.principal_id)
    .fetch_one(&mut *tx)
    .await?;
    if closing {
        tx.commit().await?;
        return Ok(false);
    }

    initialize_profile(&mut tx, &projection).await?;
    initialize_preferences(&mut tx, &projection).await?;
    initialize_notifications(&mut tx, &projection).await?;
    insert_consents(&mut tx, &projection).await?;
    tx.commit().await?;
    Ok(true)
}

async fn initialize_profile(
    tx: &mut Transaction<'_, Postgres>,
    projection: &RegistrationProjection,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO account_profiles (principal_id, created_at, updated_at)
        VALUES ($1, $2, $2)
        ON CONFLICT (principal_id) DO NOTHING
        "#,
    )
    .bind(projection.principal_id)
    .bind(projection.registered_at)
    .execute(&mut **tx)
    .await?;
    let profile = sqlx::query_as::<_, AccountProfileRow>(
        r#"
        SELECT principal_id, firstname, lastname, username, birthdate, region,
               created_at, updated_at, profile_version
        FROM account_profiles
        WHERE principal_id = $1
        "#,
    )
    .bind(projection.principal_id)
    .fetch_one(&mut **tx)
    .await?;
    crate::profile_projection::enqueue_tx(tx, &profile).await?;
    Ok(())
}

fn event_fingerprint(projection: &RegistrationProjection) -> Result<Vec<u8>, AppError> {
    use sha2::{Digest, Sha256};

    let payload = serde_json::to_vec(projection).map_err(|error| {
        AppError::internal(
            "registration_event_invalid",
            format!("Registration event could not be serialized: {error}"),
        )
    })?;
    Ok(Sha256::digest(payload).to_vec())
}

async fn ensure_matching_replay(
    tx: &mut Transaction<'_, Postgres>,
    projection: &RegistrationProjection,
    fingerprint: &[u8],
) -> Result<(), AppError> {
    let matches: bool = sqlx::query_scalar(
        r#"
        SELECT event_type = $2
          AND principal_id = $3
          AND event_fingerprint = $4
        FROM account_inbox_events
        WHERE event_id = $1
        "#,
    )
    .bind(projection.event_id)
    .bind(&projection.event_type)
    .bind(projection.principal_id)
    .bind(fingerprint)
    .fetch_one(&mut **tx)
    .await?;
    if !matches {
        return Err(AppError::conflict(
            "registration_event_collision",
            "The registration event identifier is already bound to another payload.",
        ));
    }
    Ok(())
}

async fn initialize_preferences(
    tx: &mut Transaction<'_, Postgres>,
    projection: &RegistrationProjection,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO account_preferences (principal_id) VALUES ($1) ON CONFLICT DO NOTHING",
    )
    .bind(projection.principal_id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn initialize_notifications(
    tx: &mut Transaction<'_, Postgres>,
    projection: &RegistrationProjection,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO account_notifications (principal_id, marketing_email)
        VALUES ($1, $2)
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(projection.principal_id)
    .bind(projection.marketing_emails_accepted)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn insert_consents(
    tx: &mut Transaction<'_, Postgres>,
    projection: &RegistrationProjection,
) -> Result<(), AppError> {
    for consent in &projection.consents {
        sqlx::query(
            r#"
            INSERT INTO account_consents (
              id, principal_id, consent_type, document_version, ip_address, granted_at
            )
            VALUES ($1, $2, $3, $4, $5::inet, $6)
            ON CONFLICT (principal_id, consent_type, document_version)
              WHERE revoked_at IS NULL
            DO NOTHING
            "#,
        )
        .bind(uuid::Uuid::new_v4())
        .bind(projection.principal_id)
        .bind(&consent.consent_type)
        .bind(&consent.document_version)
        .bind(&projection.consent_ip_address)
        .bind(projection.registered_at)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}
