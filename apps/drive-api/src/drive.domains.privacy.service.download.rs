use std::time::Duration;

use serde_json::{Value, json};

use crate::http::error::AppError;

use super::PrivacyRequestStatusResponse;

pub(super) async fn attach_export_download(
    storage: &dyn nvbes_storage::ObjectStore,
    request: &mut PrivacyRequestStatusResponse,
) -> Result<(), AppError> {
    if request.status != "completed" {
        return Ok(());
    }

    let Some(result) = request.result.as_mut() else {
        return Ok(());
    };
    let Some(delivery) = result.get_mut("delivery").and_then(Value::as_object_mut) else {
        return Ok(());
    };
    let Some(object_key) = delivery
        .get("object_key")
        .and_then(Value::as_str)
        .map(str::to_owned)
    else {
        return Ok(());
    };

    let expires_in = delivery
        .get("download_expires_in_seconds")
        .and_then(Value::as_u64)
        .unwrap_or(900)
        .clamp(60, 3600);
    let signed = storage
        .presign_download(&object_key, Duration::from_secs(expires_in))
        .await
        .map_err(|error| AppError::internal("privacy_export_presign_failed", &error.to_string()))?;

    delivery.remove("object_key");
    delivery.insert(
        "download".to_owned(),
        json!({
            "url": signed.url,
            "method": signed.method,
            "expires_in_seconds": signed.expires_in.as_secs(),
        }),
    );

    Ok(())
}
