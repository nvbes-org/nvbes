use crate::{
    domains::oauth::service::IntrospectionResponse,
    grpc_pb::nvbes::identity::internal::v1::IntrospectAccessTokenResponse,
};

pub fn introspection_response(
    value: IntrospectionResponse,
) -> Result<IntrospectAccessTokenResponse, serde_json::Error> {
    Ok(IntrospectAccessTokenResponse {
        active: value.active,
        scope: value.scope,
        client_id: value.client_id,
        principal_type: value.principal_type,
        token_type: value.token_type,
        audience: value.audience,
        sub: value.sub,
        role: value.role,
        tenant_id: value.tenant_id.map(|id| id.to_string()),
        organization_id: value.organization_id.map(|id| id.to_string()),
        workspace_id: value.workspace_id.map(|id| id.to_string()),
        username: value.username,
        email: value.email,
        email_verified: value.email_verified,
        display_name: value.display_name,
        acr: value.acr,
        amr: value.amr,
        auth_time: value.auth_time,
        jti: value.jti,
        sid: value.sid,
        exp: value.exp,
        iat: value.iat,
        nbf: value.nbf,
        authorization_details_json: value
            .authorization_details
            .into_iter()
            .map(|detail| serde_json::to_string(&detail))
            .collect::<Result<Vec<_>, _>>()?,
        cnf_json: value
            .cnf
            .map(|confirmation| serde_json::to_string(&confirmation))
            .transpose()?,
        act_json: value
            .act
            .map(|actor| serde_json::to_string(&actor))
            .transpose()?,
        actor_principal_type: value.actor_principal_type,
        actor_role: value.actor_role,
        actor_workspace_id: value.actor_workspace_id.map(|id| id.to_string()),
        actor_organization_id: value.actor_organization_id.map(|id| id.to_string()),
        actor_tenant_id: value.actor_tenant_id.map(|id| id.to_string()),
        network_valid: value.network_valid,
    })
}

#[cfg(test)]
mod tests {
    use crate::domains::oauth::service::IntrospectionResponse;

    #[test]
    fn inactive_response_remains_inactive() {
        let response = super::introspection_response(IntrospectionResponse::inactive()).unwrap();

        assert!(!response.active);
        assert!(response.sub.is_none());
        assert!(response.authorization_details_json.is_empty());
    }
}
