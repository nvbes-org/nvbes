use tonic::Status;
use uuid::Uuid;

use crate::grpc::{
    pb::nvbes::cloud::v1 as cloud,
    service_status::{optional_string, parse_uuid, sql_status},
};

pub async fn ensure_principal(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    principal_id: Uuid,
    projection: Option<&cloud::PrincipalProjection>,
    field: &'static str,
) -> Result<(), Status> {
    let Some(projection) = projection else {
        return require_existing_principal(tx, principal_id, field).await;
    };

    let projected_id = if projection.principal_id.trim().is_empty() {
        principal_id
    } else {
        parse_uuid(&projection.principal_id, "principal.principal_id")?
    };
    if projected_id != principal_id {
        return Err(Status::invalid_argument(format!(
            "{field} does not match principal projection"
        )));
    }

    let email = projected_email(projection, principal_id)?;
    let display_name = optional_string(projection.display_name.clone())
        .unwrap_or_else(|| projected_display_name(projection, &email));

    sqlx::query(
        r#"
        INSERT INTO users (
          id,
          email,
          display_name,
          status,
          email_verified_at,
          identity_subject
        )
        VALUES ($1, $2, $3, 'active', NOW(), $4)
        ON CONFLICT (id)
        DO UPDATE SET
          email = EXCLUDED.email,
          display_name = EXCLUDED.display_name,
          status = 'active',
          identity_subject = COALESCE(users.identity_subject, EXCLUDED.identity_subject),
          updated_at = NOW()
        "#,
    )
    .bind(principal_id)
    .bind(email)
    .bind(display_name)
    .bind(principal_id.to_string())
    .execute(&mut **tx)
    .await
    .map_err(sql_status)?;

    Ok(())
}

async fn require_existing_principal(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    principal_id: Uuid,
    field: &'static str,
) -> Result<(), Status> {
    let exists = sqlx::query_scalar::<_, bool>("SELECT EXISTS (SELECT 1 FROM users WHERE id = $1)")
        .bind(principal_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(sql_status)?;

    if exists {
        Ok(())
    } else {
        Err(Status::failed_precondition(format!(
            "{field} is not synced in Cloud; include a principal projection or sync identity first"
        )))
    }
}

fn projected_email(
    projection: &cloud::PrincipalProjection,
    principal_id: Uuid,
) -> Result<String, Status> {
    if let Some(email) = optional_string(projection.email.clone()) {
        return Ok(email.to_lowercase());
    }

    if projection.principal_kind == "service_account" {
        return Ok(format!("service-account+{principal_id}@nvbes.machine"));
    }

    Err(Status::invalid_argument(
        "principal.email is required for unsynced human principals",
    ))
}

fn projected_display_name(projection: &cloud::PrincipalProjection, email: &str) -> String {
    if projection.principal_kind == "service_account" {
        format!("Service account {email}")
    } else {
        email.to_string()
    }
}
