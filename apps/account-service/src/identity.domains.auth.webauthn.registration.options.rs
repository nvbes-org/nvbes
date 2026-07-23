use serde_json::json;

use crate::http::error::AppError;

pub(crate) fn normalize_registration_kind(kind: Option<String>) -> Result<&'static str, AppError> {
    match kind.as_deref().unwrap_or("passkey") {
        "passkey" => Ok("passkey"),
        "security_key" => Ok("security_key"),
        _ => Err(AppError::bad_request(
            "webauthn_registration_kind_invalid",
            "WebAuthn registration kind is invalid.",
        )),
    }
}

pub(crate) fn shape_registration_options(
    mut options: serde_json::Value,
    kind: &'static str,
) -> serde_json::Value {
    let Some(public_key) = options.get_mut("publicKey") else {
        return options;
    };
    if let Some(public_key_object) = public_key.as_object_mut() {
        public_key_object.insert(
            "authenticatorSelection".to_string(),
            match kind {
                "security_key" => json!({
                    "authenticatorAttachment": "cross-platform",
                    "requireResidentKey": false,
                    "residentKey": "discouraged",
                    "userVerification": "required",
                }),
                _ => json!({
                    "authenticatorAttachment": "platform",
                    "requireResidentKey": false,
                    "residentKey": "preferred",
                    "userVerification": "required",
                }),
            },
        );
        public_key_object.insert(
            "hints".to_string(),
            json!(["security-key", "client-device", "hybrid"]),
        );
    }

    options
}
