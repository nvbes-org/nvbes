use uuid::Uuid;

pub trait DeveloperClientSecretRevoker: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    fn revoke_secret_version(
        &self,
        tenant_id: Uuid,
        client_id: &str,
        version_id: Uuid,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send;
}
