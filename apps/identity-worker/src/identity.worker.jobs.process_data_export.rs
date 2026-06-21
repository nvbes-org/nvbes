use anyhow::Context;
use serde_json::Value;
use uuid::Uuid;

use crate::app::AppState;

#[derive(Debug, Clone, PartialEq, Eq)]
struct DataExportJobPayload {
    user_id: Uuid,
    email: String,
}

pub(super) async fn process_data_export(state: &AppState, payload: &Value) -> anyhow::Result<()> {
    let payload = parse_data_export_payload(payload)?;

    let user = crate::domains::auth::db::fetch_user_record(&state.db, payload.user_id)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to fetch user: {e:?}"))?;

    let export =
        crate::domains::auth::data_export::build_account_export(&state.db, payload.user_id)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to build data export: {e:?}"))?;
    crate::domains::auth::data_export::store_account_export(&state.redis, payload.user_id, &export)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to store data export: {e:?}"))?;

    let html_body = format!(
        "<p>Bonjour {},</p><p>Votre export de donnees personnelles est pret. Connectez-vous a votre compte nvbes et relancez le telechargement depuis la page de confidentialite. Le fichier expire automatiquement sous 24 heures.</p><p>L'equipe nvbes</p>",
        user.display_name
    );

    let from_email = state
        .config
        .email_from_email
        .clone()
        .ok_or_else(|| anyhow::anyhow!("NVBES_EMAIL_FROM_EMAIL must be set"))?;

    let msg = nvbes_email::EmailMessage {
        from: nvbes_email::EmailAddress {
            email: from_email,
            name: Some(
                state
                    .config
                    .email_from_name
                    .clone()
                    .unwrap_or_else(|| "nvbes".to_string()),
            ),
        },
        to: vec![nvbes_email::EmailAddress {
            email: payload.email.clone(),
            name: Some(user.display_name.clone()),
        }],
        subject: "Export de vos donnees - nvbes".to_string(),
        html_body: Some(html_body),
        text_body: Some(format!(
            "Bonjour {}, votre export de donnees personnelles est pret. Connectez-vous a votre compte nvbes et relancez le telechargement depuis la page de confidentialite. Le fichier expire automatiquement sous 24 heures.",
            user.display_name
        )),
        headers: vec![("Reply-To".to_string(), payload.email.clone())],
    };

    let result = state
        .email
        .send_message(&msg)
        .await
        .context("Failed to send export email")?;

    let mut tx = state.db.begin().await?;
    crate::email::db::record_email_sent_event_tx(
        &mut tx,
        &payload.email,
        &result.provider_email_id,
        "Export de vos donnees - nvbes",
    )
    .await
    .map_err(|e| anyhow::anyhow!("Failed to record sent event: {e:?}"))?;
    tx.commit().await?;

    Ok(())
}

fn parse_data_export_payload(payload: &Value) -> anyhow::Result<DataExportJobPayload> {
    let user_id = payload
        .get("user_id")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("Missing user_id"))?
        .parse()
        .map_err(|e| anyhow::anyhow!("Invalid user_id: {e}"))?;
    let email = payload
        .get("email")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("Missing email"))
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

        assert!(error.to_string().contains("Missing user_id"));
    }

    #[test]
    fn data_export_worker_payload_rejects_invalid_user_id() {
        let error = parse_data_export_payload(&json!({
            "user_id": "not-a-uuid",
            "email": "privacy@example.test"
        }))
        .expect_err("invalid user_id should fail");

        assert!(error.to_string().contains("Invalid user_id"));
    }
}
