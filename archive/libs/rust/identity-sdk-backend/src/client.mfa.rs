use super::IdentityClient;
use crate::types::*;
use crate::SdkError;

impl IdentityClient {
    /// Liste les facteurs MFA de l'utilisateur.
    pub async fn list_mfa_factors(&self, token: &str) -> Result<MfaFactorsResult, SdkError> {
        self.send_json(
            self.http
                .get(self.endpoint("/auth/mfa/factors"))
                .bearer_auth(token),
        )
        .await
    }

    /// Configure TOTP après un step-up récent.
    pub async fn setup_totp(
        &self,
        token: &str,
        _password: &str,
        label: Option<&str>,
    ) -> Result<TotpSetupResult, SdkError> {
        self.setup_totp_after_step_up(token, label).await
    }

    pub async fn setup_totp_after_step_up(
        &self,
        token: &str,
        label: Option<&str>,
    ) -> Result<TotpSetupResult, SdkError> {
        self.send_json(
            self.http
                .post(self.endpoint("/auth/mfa/totp/setup"))
                .bearer_auth(token)
                .json(&serde_json::json!({ "label": label })),
        )
        .await
    }

    /// Confirme le code TOTP et active le facteur.
    pub async fn confirm_totp(
        &self,
        token: &str,
        factor_id: &str,
        code: &str,
    ) -> Result<TotpConfirmResult, SdkError> {
        self.send_json(
            self.http
                .post(self.endpoint("/auth/mfa/totp/confirm"))
                .bearer_auth(token)
                .json(&serde_json::json!({
                    "factor_id": factor_id,
                    "code": code,
                })),
        )
        .await
    }

    /// Démarre l'enregistrement WebAuthn.
    pub async fn start_webauthn_registration(
        &self,
        token: &str,
        label: Option<&str>,
    ) -> Result<WebauthnRegisterStartResult, SdkError> {
        self.start_webauthn_registration_with_kind(token, label, None)
            .await
    }

    pub async fn start_webauthn_registration_with_kind(
        &self,
        token: &str,
        label: Option<&str>,
        kind: Option<&str>,
    ) -> Result<WebauthnRegisterStartResult, SdkError> {
        self.send_json(
            self.http
                .post(self.endpoint("/auth/mfa/webauthn/register/start"))
                .bearer_auth(token)
                .json(&serde_json::json!({
                    "label": label,
                    "kind": kind,
                })),
        )
        .await
    }

    /// Termine l'enregistrement WebAuthn.
    pub async fn finish_webauthn_registration(
        &self,
        token: &str,
        factor_id: &str,
        credential: serde_json::Value,
    ) -> Result<TotpConfirmResult, SdkError> {
        self.send_json(
            self.http
                .post(self.endpoint("/auth/mfa/webauthn/register/finish"))
                .bearer_auth(token)
                .json(&serde_json::json!({
                    "factor_id": factor_id,
                    "reg": credential,
                })),
        )
        .await
    }

    /// Démarre l'authentification WebAuthn pour step-up ou login MFA.
    pub async fn start_webauthn_authentication(
        &self,
        token: &str,
    ) -> Result<WebauthnAuthStartResult, SdkError> {
        self.send_json(
            self.http
                .post(self.endpoint("/auth/mfa/webauthn/start"))
                .bearer_auth(token),
        )
        .await
    }

    /// Génère de nouveaux codes de récupération après step-up.
    pub async fn generate_recovery_codes(
        &self,
        token: &str,
        password: &str,
    ) -> Result<RecoveryCodesResult, SdkError> {
        self.send_json(
            self.http
                .post(self.endpoint("/auth/mfa/recovery-codes"))
                .bearer_auth(token)
                .json(&serde_json::json!({ "password": password })),
        )
        .await
    }

    /// Supprime un facteur MFA.
    pub async fn remove_mfa_factor(&self, token: &str, factor_id: &str) -> Result<(), SdkError> {
        self.send_empty(
            self.http
                .delete(self.endpoint(&format!("/auth/mfa/factors/{factor_id}")))
                .bearer_auth(token),
        )
        .await
    }
}
