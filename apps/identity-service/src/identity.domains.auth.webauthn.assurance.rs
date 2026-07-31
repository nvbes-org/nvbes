use nvbes_core::auth::Aal;
use serde_json::Value;
use webauthn_rs::prelude::AuthenticationResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebauthnAssurance {
    SyncedPasskey,
    DeviceBoundPasskey,
    HardwareSecurityKey,
}

impl WebauthnAssurance {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SyncedPasskey => "synced_passkey",
            Self::DeviceBoundPasskey => "device_bound_passkey",
            Self::HardwareSecurityKey => "hardware_security_key",
        }
    }

    pub fn aal(self) -> Aal {
        match self {
            Self::SyncedPasskey => Aal::Aal2,
            Self::DeviceBoundPasskey | Self::HardwareSecurityKey => Aal::Aal3,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebauthnCredentialSignals {
    pub assurance: WebauthnAssurance,
    pub backup_eligible: bool,
    pub backup_state: bool,
    pub sign_count: i64,
    pub attestation_format: Option<String>,
}

impl WebauthnCredentialSignals {
    pub fn from_registered_credential(credential: &Value, declared_kind: &str) -> Self {
        let credential = credential.get("cred").unwrap_or(credential);
        let backup_eligible = bool_field(credential, "backup_eligible");
        Self {
            assurance: classify(declared_kind, backup_eligible),
            backup_eligible,
            backup_state: bool_field(credential, "backup_state"),
            sign_count: unsigned_field(credential, "counter"),
            attestation_format: credential
                .pointer("/attestation/attestation_format")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
        }
    }

    pub fn from_authentication(
        result: &AuthenticationResult,
        declared_kind: &str,
        attestation_format: Option<String>,
    ) -> Self {
        Self {
            assurance: classify(declared_kind, result.backup_eligible()),
            backup_eligible: result.backup_eligible(),
            backup_state: result.backup_state(),
            sign_count: i64::from(result.counter()),
            attestation_format,
        }
    }
}

fn classify(declared_kind: &str, backup_eligible: bool) -> WebauthnAssurance {
    if backup_eligible {
        WebauthnAssurance::SyncedPasskey
    } else if declared_kind == "security_key" {
        WebauthnAssurance::HardwareSecurityKey
    } else {
        WebauthnAssurance::DeviceBoundPasskey
    }
}

fn bool_field(value: &Value, field: &str) -> bool {
    value.get(field).and_then(Value::as_bool).unwrap_or(false)
}

fn unsigned_field(value: &Value, field: &str) -> i64 {
    value
        .get(field)
        .and_then(Value::as_u64)
        .and_then(|value| i64::try_from(value).ok())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn backup_eligible_credentials_are_classified_as_synced() {
        let signals = WebauthnCredentialSignals::from_registered_credential(
            &json!({
                "cred": {
                    "backup_eligible": true,
                    "backup_state": true,
                    "counter": 0,
                    "attestation": { "attestation_format": "None" }
                }
            }),
            "passkey",
        );

        assert_eq!(signals.assurance, WebauthnAssurance::SyncedPasskey);
        assert!(signals.backup_state);
        assert_eq!(signals.attestation_format.as_deref(), Some("None"));
    }

    #[test]
    fn non_backup_cross_platform_credentials_are_hardware_keys() {
        let signals = WebauthnCredentialSignals::from_registered_credential(
            &json!({ "backup_eligible": false, "counter": 7 }),
            "security_key",
        );

        assert_eq!(signals.assurance, WebauthnAssurance::HardwareSecurityKey);
        assert_eq!(signals.sign_count, 7);
        assert_eq!(signals.assurance.aal(), Aal::Aal3);
    }
}
