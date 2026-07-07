#![allow(dead_code)]

#[path = "identity.domains.oauth.service.types.auth.rs"]
mod auth;
#[path = "identity.domains.oauth.service.types.client.rs"]
mod client;
#[path = "identity.domains.oauth.service.types.device.rs"]
mod device;
#[path = "identity.domains.oauth.service.types.introspection.rs"]
mod introspection;
#[path = "identity.domains.oauth.service.types.par.rs"]
mod par;
#[path = "identity.domains.oauth.service.types.tokens.rs"]
mod tokens;

pub use auth::{
    AuthCodeRecord, ClientAssertionAuthentication, ClientAuthentication,
    CreateAuthorizationCodeInput, ExchangeCodeInput, TokenExchangeInput,
};
pub use client::{
    CreateOAuthClientInput, CreateOAuthClientPolicyInput, CreateOAuthClientResult,
    DeleteOAuthClientPolicyResult, OAuthClientPoliciesResult, OAuthClientPolicyView,
    OAuthClientView, OAuthClientsResult, RevokeOAuthClientResult, UpdateOAuthClientPolicyInput,
};
pub use device::{
    DeviceApprovalInput, DeviceAuthorizationInput, DeviceAuthorizationView,
    DeviceVerificationInput, DeviceVerificationView, ExchangeDeviceCodeInput,
};
pub use introspection::IntrospectionResponse;
pub use par::ParResponse;
pub use tokens::{AuthorizationCodeView, TokenView};
