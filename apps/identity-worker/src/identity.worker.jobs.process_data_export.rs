use anyhow::Context;
use serde_json::Value;

use crate::app::AppState;

pub(super) async fn process_data_export(state: &AppState, payload: &Value) -> anyhow::Result<()> {
    let user_id: uuid::Uuid = payload
        .get("user_id")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("Missing user_id"))?
        .parse()
        .map_err(|e| anyhow::anyhow!("Invalid user_id: {e}"))?;
    let email: String = payload
        .get("email")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("Missing email"))
        .map(String::from)?;

    let user = crate::domains::auth::db::fetch_user_record(&state.db, user_id)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to fetch user: {e:?}"))?;

    let export = crate::domains::auth::data_export::build_account_export(&state.db, user_id)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to build data export: {e:?}"))?;
    crate::domains::auth::data_export::store_account_export(&state.redis, user_id, &export)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to store data export: {e:?}"))?;

    let html_body = format!(
        "<p>Bonjour {},</p><p>Votre export de donnees personnelles est pret. Connectez-vous a votre compte nvbes et relancez le telechargement depuis la page de confidentialite. Le fichier expire automatiquement sous 24 heures.</p><p>L'equipe nvbes</p>",
        user.display_name
    );

    let from_email = state
        .config
        .scw_tem_from_email
        .clone()
        .ok_or_else(|| anyhow::anyhow!("SCW_TEM_FROM_EMAIL must be set"))?;

    let msg = nvbes_email::EmailMessage {
        from: nvbes_email::EmailAddress {
            email: from_email,
            name: Some(
                state
                    .config
                    .scw_tem_from_name
                    .clone()
                    .unwrap_or_else(|| "nvbes".to_string()),
            ),
        },
        to: vec![nvbes_email::EmailAddress {
            email: email.clone(),
            name: Some(user.display_name.clone()),
        }],
        subject: "Export de vos donnees - nvbes".to_string(),
        html_body: Some(html_body),
        text_body: Some(format!(
            "Bonjour {}, votre export de donnees personnelles est pret. Connectez-vous a votre compte nvbes et relancez le telechargement depuis la page de confidentialite. Le fichier expire automatiquement sous 24 heures.",
            user.display_name
        )),
        headers: vec![("Reply-To".to_string(), email.clone())],
    };

    let result = state
        .email
        .send_message(&msg)
        .await
        .context("Failed to send export email")?;

    let mut tx = state.db.begin().await?;
    crate::email::db::record_email_sent_event_tx(
        &mut tx,
        &email,
        &result.provider_email_id,
        "Export de vos donnees - nvbes",
    )
    .await
    .map_err(|e| anyhow::anyhow!("Failed to record sent event: {e:?}"))?;
    tx.commit().await?;

    Ok(())
}
