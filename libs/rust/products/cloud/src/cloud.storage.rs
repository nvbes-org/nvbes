use nvbes_core::config::AppConfig;
use nvbes_storage::ObjectStore;
use std::sync::Arc;

pub async fn build_storage(config: &AppConfig) -> Arc<dyn ObjectStore> {
    if config.storage_enabled {
        let endpoint = config
            .storage_endpoint
            .as_deref()
            .expect("STORAGE_ENDPOINT required when STORAGE_ENABLED is true");
        let access_key = config
            .storage_access_key
            .as_deref()
            .expect("STORAGE_ACCESS_KEY required when STORAGE_ENABLED is true");
        let secret_key = config
            .storage_secret_key
            .as_deref()
            .expect("STORAGE_SECRET_KEY required when STORAGE_ENABLED is true");

        tracing::info!(
            bucket = %config.storage_bucket,
            endpoint,
            region = %config.storage_region,
            "Storage: S3 (real)"
        );

        Arc::new(
            nvbes_storage::S3ObjectStore::new(
                config.storage_bucket.clone(),
                endpoint,
                &config.storage_region,
                access_key,
                secret_key,
            )
            .await,
        )
    } else {
        if config.environment != "development" {
            panic!(
                "Mock storage is forbidden outside development. Enable STORAGE_ENABLED and configure S3-compatible credentials."
            );
        }
        tracing::info!("Storage: Mock (development mode)");
        Arc::new(nvbes_storage::MockObjectStore::new())
    }
}
