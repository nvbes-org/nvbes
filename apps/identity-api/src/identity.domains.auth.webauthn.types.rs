use serde::{Deserialize, Serialize};
use webauthn_rs::prelude::{
    DiscoverableAuthentication, PasskeyAuthentication, PasskeyRegistration, SecurityKeyRegistration,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum StoredWebauthnRegistration {
    Passkey {
        registration: PasskeyRegistration,
    },
    SecurityKey {
        registration: SecurityKeyRegistration,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct StoredPasskeyAuthentication {
    pub authentication: PasskeyAuthentication,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct StoredDiscoverableAuthentication {
    pub authentication: DiscoverableAuthentication,
}
