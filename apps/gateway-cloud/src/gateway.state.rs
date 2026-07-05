#[derive(Clone)]
pub struct GatewayState {
    pub billing_grpc_endpoint: String,
}

#[derive(Clone, Debug)]
pub struct GatewayRequestContext {
    pub request_id: String,
    pub correlation_id: String,
    pub actor_principal_id: String,
    pub tenant_id: String,
}

impl GatewayRequestContext {
    pub fn for_workspace(
        &self,
        workspace_id: &str,
    ) -> crate::pb::nvbes::platform::v1::RequestContext {
        crate::pb::nvbes::platform::v1::RequestContext {
            request_id: self.request_id.clone(),
            correlation_id: self.correlation_id.clone(),
            actor_principal_id: self.actor_principal_id.clone(),
            tenant: Some(crate::pb::nvbes::platform::v1::TenantContext {
                tenant_id: self.tenant_id.clone(),
                workspace_id: workspace_id.to_string(),
                region_id: String::new(),
                data_residency: String::new(),
            }),
        }
    }
}
