use std::net::SocketAddr;

#[derive(Debug, Clone)]
pub enum DispatchMode {
    InMemory,
    Scaleway(DispatchQueueConfig),
}

#[derive(Debug, Clone)]
pub struct DispatchQueueConfig {
    pub endpoint: String,
    pub queue_url: String,
    pub region: String,
    pub access_key: String,
    pub secret_key: String,
}

#[derive(Debug, Clone)]
pub struct BillingWorkerConfig {
    pub environment: String,
    pub database_url: String,
    pub http_bind_addr: SocketAddr,
    pub app_url: String,
    pub email_grpc_endpoint: Option<String>,
    pub email_token: Option<String>,
    pub billing_grpc_endpoint: Option<String>,
    pub billing_grpc_token: Option<String>,
    pub metrics_token: Option<String>,
    pub dispatch_mode: DispatchMode,
    pub otlp_endpoint: Option<String>,
    pub otlp_authorization_header: Option<String>,
}

impl BillingWorkerConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let environment =
            std::env::var("NVBES_ENVIRONMENT").unwrap_or_else(|_| "development".to_string());

        let database_url = std::env::var("NVBES_BILLING_DATABASE_URL")
            .or_else(|_| std::env::var("DATABASE_URL"))
            .map_err(|_| anyhow::anyhow!("NVBES_BILLING_DATABASE_URL is required"))?;

        let http_bind_addr: SocketAddr = std::env::var("NVBES_BILLING_HTTP_BIND_ADDR")
            .or_else(|_| std::env::var("NVBES_BILLING_BIND_ADDR"))
            .unwrap_or_else(|_| "0.0.0.0:8080".to_string())
            .parse()
            .map_err(|e| anyhow::anyhow!("invalid NVBES_BILLING_HTTP_BIND_ADDR: {e}"))?;

        let app_url =
            std::env::var("NVBES_APP_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

        let email_grpc_endpoint = std::env::var("NVBES_EMAIL_GRPC_ENDPOINT").ok();
        let email_token = std::env::var("NVBES_EMAIL_PRODUCER_TOKEN")
            .or_else(|_| std::env::var("NVBES_EMAIL_TOKEN"))
            .ok();
        let billing_grpc_endpoint = std::env::var("NVBES_BILLING_GRPC_ENDPOINT").ok();
        let billing_grpc_token = std::env::var("NVBES_BILLING_GRPC_AUTH_TOKEN").ok();
        let metrics_token = std::env::var("NVBES_BILLING_METRICS_TOKEN").ok();
        let otlp_endpoint = std::env::var("NVBES_OTLP_ENDPOINT").ok();
        let otlp_authorization_header = std::env::var("NVBES_OTLP_AUTHORIZATION_HEADER").ok();

        // Strict V1 test-mode check - fail closed if any live key leaked into env
        for key in ["STRIPE_SECRET_KEY", "NVBES_STRIPE_SECRET_KEY"] {
            if std::env::var(key)
                .is_ok_and(|val| val.starts_with("sk_live_") || val.starts_with("rk_live_"))
            {
                anyhow::bail!(
                    "CRITICAL: Live Stripe credentials are fundamentally prohibited in V1"
                );
            }
        }

        let dispatch_mode = match (
            std::env::var("NVBES_BILLING_QUEUE_ENDPOINT"),
            std::env::var("NVBES_BILLING_QUEUE_URL"),
            std::env::var("NVBES_BILLING_QUEUE_REGION"),
            std::env::var("NVBES_BILLING_QUEUE_ACCESS_KEY"),
            std::env::var("NVBES_BILLING_QUEUE_SECRET_KEY"),
        ) {
            (Ok(endpoint), Ok(queue_url), Ok(region), Ok(access_key), Ok(secret_key)) => {
                DispatchMode::Scaleway(DispatchQueueConfig {
                    endpoint,
                    queue_url,
                    region,
                    access_key,
                    secret_key,
                })
            }
            _ => DispatchMode::InMemory,
        };

        Ok(Self {
            environment,
            database_url,
            http_bind_addr,
            app_url,
            email_grpc_endpoint,
            email_token,
            billing_grpc_endpoint,
            billing_grpc_token,
            metrics_token,
            dispatch_mode,
            otlp_endpoint,
            otlp_authorization_header,
        })
    }
}

#[cfg(test)]
#[path = "billing.worker.config.tests.rs"]
mod tests;
