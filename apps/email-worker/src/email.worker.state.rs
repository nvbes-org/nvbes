use std::{
    sync::{
        Arc,
        atomic::{AtomicI64, Ordering},
    },
    time::Duration,
};

use chrono::Utc;
use nvbes_email::EmailSender;
use sqlx::PgPool;

use crate::{
    config::{EmailWorkerConfig, ProviderConfig},
    crypto::EmailCrypto,
    webhook_verify::WebhookVerifier,
};

#[derive(Clone)]
pub struct EmailWorkerState {
    pub config: Arc<EmailWorkerConfig>,
    pub db: PgPool,
    pub crypto: Arc<EmailCrypto>,
    pub provider: Arc<dyn EmailSender>,
    pub webhook_verifier: Option<Arc<WebhookVerifier>>,
    dispatcher_heartbeat: Arc<AtomicI64>,
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
        Ok(Self {
            config: Arc::new(config),
            db,
            crypto: Arc::new(crypto),
            provider,
            webhook_verifier,
            dispatcher_heartbeat: Arc::new(AtomicI64::new(0)),
        })
    }

    pub fn record_dispatcher_heartbeat(&self) {
        self.dispatcher_heartbeat
            .store(Utc::now().timestamp(), Ordering::Release);
    }

    pub fn dispatcher_is_current(&self, maximum_age: Duration) -> bool {
        let heartbeat = self.dispatcher_heartbeat.load(Ordering::Acquire);
        heartbeat > 0 && Utc::now().timestamp() - heartbeat <= maximum_age.as_secs() as i64
    }
}
