use uuid::Uuid;

pub trait EnterpriseAuditWriter: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    fn record_enterprise_event(
        &self,
        tenant_id: Uuid,
        actor_principal_id: Uuid,
        action: &'static str,
        target_id: Option<Uuid>,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send;
}
