use nvbes_product_account::{
    auth::{
        data_export::{build_account_export, store_account_export},
        users::fetch_user_record,
    },
    email::db::{completed_email_delivery, ensure_email_delivery},
};
use nvbes_redis::worker_queue::QueuedJob;
use serde_json::Value;
use uuid::Uuid;

use super::{
    email_delivery::{deliver_email, deterministic_message_id},
    email_from_address,
};
use crate::app::AppState;
use crate::worker::job_failure::JobExecutionError;

#[derive(Debug, Clone, PartialEq, Eq)]
struct DataExportJobPayload {
    user_id: Uuid,
    email: String,
}

pub(super) async fn process_data_export(
    state: &AppState,
    job: &QueuedJob,
) -> Result<bool, JobExecutionError> {
    let payload = parse_data_export_payload(&job.payload)?;
    ensure_email_delivery(
        &state.db,
        job.id,
        "data_export",
        &payload.email,
        &deterministic_message_id(job.id),
    )
    .await
    .map_err(|error| JobExecutionError::from_account(&error))?;
    if completed_email_delivery(&state.db, job.id)
        .await
        .map_err(|error| JobExecutionError::from_account(&error))?
        .is_some()
    {
        return Ok(true);
    }

    let user = fetch_user_record(&state.db, payload.user_id)
        .await
        .map_err(|error| JobExecutionError::from_account(&error))?;

    let export = build_account_export(&state.db, payload.user_id)
        .await
        .map_err(|error| JobExecutionError::from_account(&error))?;
    store_account_export(&state.redis, payload.user_id, &export)
        .await
        .map_err(|error| JobExecutionError::from_account(&error))?;

    let html_body = format!(
        "<p>Bonjour {},</p><p>Votre export de donnees personnelles est pret. Connectez-vous a votre compte nvbes et relancez le telechargement depuis la page de confidentialite. Le fichier expire automatiquement sous 24 heures.</p><p>L'equipe nvbes</p>",
        user.display_name
    );

    let subject = "Export de vos donnees - nvbes";
    let message = nvbes_email::EmailMessage {
        from: email_from_address(&state.config)?,
        to: vec![nvbes_email::EmailAddress {
            email: payload.email.clone(),
            name: Some(user.display_name.clone()),
        }],
        subject: subject.to_string(),
        html_body: Some(html_body),
        text_body: Some(format!(
            "Bonjour {}, votre export de donnees personnelles est pret. Connectez-vous a votre compte nvbes et relancez le telechargement depuis la page de confidentialite. Le fichier expire automatiquement sous 24 heures.",
            user.display_name
        )),
        headers: vec![("Reply-To".to_string(), payload.email.clone())],
    };
    let outcome = deliver_email(
        &state.db,
        state.email.as_ref(),
        job.id,
        "data_export",
        &payload.email,
        subject,
        message,
    )
    .await?;

    Ok(outcome.deduplicated)
}

fn parse_data_export_payload(payload: &Value) -> Result<DataExportJobPayload, JobExecutionError> {
    let user_id = payload
        .get("user_id")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            JobExecutionError::permanent(
                "data_export_user_id_missing",
                "Data export job is missing the user identifier",
            )
        })?
        .parse()
        .map_err(|_| {
            JobExecutionError::permanent(
                "data_export_user_id_invalid",
                "Data export job user identifier is invalid",
            )
        })?;
    let email = payload
        .get("email")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            JobExecutionError::permanent(
                "data_export_email_missing",
                "Data export job is missing the recipient",
            )
        })
        .map(String::from)?;

    Ok(DataExportJobPayload { user_id, email })
}

#[cfg(test)]
mod tests {
    use super::parse_data_export_payload;
    use serde_json::json;
    use uuid::Uuid;

    #[test]
    fn data_export_worker_payload_accepts_user_and_email() {
        let user_id = Uuid::new_v4();
        let payload = parse_data_export_payload(&json!({
            "user_id": user_id,
            "email": "privacy@example.test"
        }))
        .expect("payload should parse");

        assert_eq!(payload.user_id, user_id);
        assert_eq!(payload.email, "privacy@example.test");
    }

    #[test]
    fn data_export_worker_payload_rejects_missing_user_id() {
        let error = parse_data_export_payload(&json!({
            "email": "privacy@example.test"
        }))
        .expect_err("missing user_id should fail");

        assert_eq!(error.code(), "data_export_user_id_missing");
    }

    #[test]
    fn data_export_worker_payload_rejects_invalid_user_id() {
        let error = parse_data_export_payload(&json!({
            "user_id": "not-a-uuid",
            "email": "privacy@example.test"
        }))
        .expect_err("invalid user_id should fail");

        assert_eq!(error.code(), "data_export_user_id_invalid");
    }
}
