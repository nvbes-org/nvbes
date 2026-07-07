use crate::domains::{
    developer::grpc::DeveloperCredentialSummary,
    enterprise::types::EnterpriseDeveloperCredentialSummary,
};

pub(super) fn enterprise_developer_credential(
    credential: DeveloperCredentialSummary,
) -> EnterpriseDeveloperCredentialSummary {
    EnterpriseDeveloperCredentialSummary {
        id: credential.id,
        client_id: credential.client_id,
        name: credential.name,
        status: credential.status,
        secret_last4: credential.secret_last4,
        owner_email: credential.owner_email,
        scopes: credential.scopes,
        last_used_at: credential.last_used_at,
        created_at: credential.created_at,
        expires_at: credential.expires_at,
    }
}
