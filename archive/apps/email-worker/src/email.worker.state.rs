use std::sync::{Arc, Mutex};

use nvbes_email::EmailSender;
use sqlx::PgPool;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::{
    config::{EmailWorkerConfig, ProviderConfig},
    crypto::EmailCrypto,
    queue::{self, DispatchQueue},
    webhook_verify::WebhookVerifier,
};

#[derive(Clone)]
pub struct EmailWorkerState {
    pub config: Arc<EmailWorkerConfig>,
    pub db: PgPool,
    pub crypto: Arc<EmailCrypto>,
    pub provider: Arc<dyn EmailSender>,
    pub dispatch_queue: Arc<dyn DispatchQueue>,
    pub metrics: Arc<metrics_exporter_prometheus::PrometheusHandle>,
    pub webhook_verifier: Option<Arc<WebhookVerifier>>,
    local_dispatch_receiver: Arc<Mutex<Option<mpsc::Receiver<Uuid>>>>,
}

impl EmailWorkerState {
    pub fn new(config: EmailWorkerConfig, db: PgPool) -> anyhow::Result<Self> {
        let crypto = EmailCrypto::new(config.data_encryption_key, config.recipient_hmac_key);
        let provider: Arc<dyn EmailSender> = match &config.provider {
            ProviderConfig::Mock => Arc::new(nvbes_email::MockEmailSender::new()),
            ProviderConfig::Smtp(provider) => {
                Arc::new(nvbes_email::SmtpEmailSender::new(provider.clone())?)
            }
            ProviderConfig::TestCapture(directory) => {
                Arc::new(nvbes_email::TestCaptureEmailSender::new(directory)?)
            }
            ProviderConfig::Scaleway(provider) => {
                Arc::new(nvbes_email_scaleway::ScalewayEmailClient::new(
                    provider.secret_key.clone(),
                    provider.project_id.clone(),
                    &provider.region,
                ))
            }
        };
        let webhook_verifier = config
            .webhook
            .as_ref()
            .map(WebhookVerifier::new)
            .transpose()?
            .map(Arc::new);
        let queue = queue::runtime(&config.dispatch);
        Ok(Self {
            config: Arc::new(config),
            db,
            crypto: Arc::new(crypto),
            provider,
            dispatch_queue: queue.publisher,
            metrics: crate::email_metrics::install(),
            webhook_verifier,
            local_dispatch_receiver: Arc::new(Mutex::new(queue.local_receiver)),
        })
    }

    pub fn take_local_dispatch_receiver(&self) -> Option<mpsc::Receiver<Uuid>> {
        self.local_dispatch_receiver
            .lock()
            .expect("local email queue lock must not be poisoned")
            .take()
    }
}

#[cfg(all(test, feature = "database-tests"))]
#[path = "email.worker.state.tests.rs"]
mod tests;
