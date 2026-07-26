use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Deserialize, ToSchema)]
pub(crate) struct WebauthnRegisterStartRequest {
    pub(crate) label: Option<String>,
    pub(crate) kind: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub(crate) struct WebauthnRegisterFinishRequest {
    pub(crate) factor_id: Uuid,
    #[schema(value_type = Object)]
    pub(crate) reg: webauthn_rs::prelude::RegisterPublicKeyCredential,
}

#[derive(Deserialize, ToSchema)]
pub(crate) struct RecoveryCodesGenerateRequest {
    pub(crate) password: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub(crate) struct TotpSetupRequest {
    pub(crate) label: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub(crate) struct TotpConfirmRequest {
    pub(crate) factor_id: Uuid,
    pub(crate) code: String,
}

#[derive(Deserialize, ToSchema)]
pub(crate) struct EmailMfaSetupRequest {
    pub(crate) email_id: Uuid,
}
