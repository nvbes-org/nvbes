use std::sync::Arc;

use nvbes_billing::BillingClient;
use nvbes_email::EmailClient;
use sqlx::PgPool;
use tokio::sync::{Mutex, mpsc};

use crate::{
    config::BillingWorkerConfig,
    queue::{DispatchQueue, runtime},
};

#[derive(Clone)]
pub struct BillingWorkerState {
    pub config: Arc<BillingWorkerConfig>,
    pub db: PgPool,
    pub queue: Arc<dyn DispatchQueue>,
    pub email_client: Option<Arc<EmailClient>>,
    pub billing_client: Option<Arc<BillingClient>>,
    local_dispatch_rx: Arc<Mutex<Option<mpsc::Receiver<String>>>>,
}

impl BillingWorkerState {
    pub async fn new(config: BillingWorkerConfig, db: PgPool) -> anyhow::Result<Self> {
        let queue_runtime = runtime(&config.dispatch_mode);

        let email_client = if let (Some(endpoint), Some(token)) =
            (&config.email_grpc_endpoint, &config.email_token)
        {
            let email_cfg = nvbes_email::EmailClientConfig::from_values(
                &config.environment,
                endpoint.clone(),
                token.clone(),
                std::time::Duration::from_secs(5),
            )?;
            nvbes_email::EmailClient::connect(email_cfg)
                .await
                .ok()
                .map(Arc::new)
        } else {
            None
        };

        let billing_client = if let (Some(endpoint), Some(token)) =
            (&config.billing_grpc_endpoint, &config.billing_grpc_token)
        {
            let billing_cfg = nvbes_billing::BillingClientConfig::from_values(
                &config.environment,
                endpoint.clone(),
                token.clone(),
                std::time::Duration::from_secs(10),
            )
            .ok();
            if let Some(cfg) = billing_cfg {
                BillingClient::connect(cfg).await.ok().map(Arc::new)
            } else {
                None
            }
        } else {
            None
        };

        Ok(Self {
            config: Arc::new(config),
            db,
            queue: queue_runtime.publisher,
            email_client,
            billing_client,
            local_dispatch_rx: Arc::new(Mutex::new(queue_runtime.local_receiver)),
        })
    }

    pub async fn enqueue_event(&self, event_id: &str) -> anyhow::Result<()> {
        self.queue.enqueue(event_id).await
    }

    pub async fn take_local_dispatch_receiver(&self) -> Option<mpsc::Receiver<String>> {
        self.local_dispatch_rx.lock().await.take()
    }
}
