use tonic::{
    Request,
    metadata::{Ascii, MetadataValue},
    transport::{Channel, ClientTlsConfig, Endpoint},
};

const ENTERPRISE_GRPC_ENDPOINT_ENV: &str = "NVBES_ENTERPRISE_GRPC_ENDPOINT";
const ENTERPRISE_GRPC_AUTH_TOKEN_ENV: &str = "NVBES_ENTERPRISE_GRPC_AUTH_TOKEN";
const MIN_AUTH_TOKEN_LENGTH: usize = 32;

#[derive(Clone)]
pub struct EnterpriseGrpcConfig {
    endpoint: Endpoint,
    authorization: MetadataValue<Ascii>,
}

impl EnterpriseGrpcConfig {
    pub fn from_env(environment: &str) -> anyhow::Result<Option<Self>> {
        Self::from_values(
            environment,
            std::env::var(ENTERPRISE_GRPC_ENDPOINT_ENV).ok(),
            std::env::var(ENTERPRISE_GRPC_AUTH_TOKEN_ENV).ok(),
        )
    }

    fn from_values(
        environment: &str,
        endpoint: Option<String>,
        auth_token: Option<String>,
    ) -> anyhow::Result<Option<Self>> {
        let Some(endpoint) = normalized(endpoint) else {
            return Ok(None);
        };
        let auth_token = normalized(auth_token).ok_or_else(|| {
            anyhow::anyhow!(
                "{ENTERPRISE_GRPC_AUTH_TOKEN_ENV} is required when {ENTERPRISE_GRPC_ENDPOINT_ENV} is configured"
            )
        })?;
        if auth_token.len() < MIN_AUTH_TOKEN_LENGTH {
            anyhow::bail!(
                "{ENTERPRISE_GRPC_AUTH_TOKEN_ENV} must contain at least {MIN_AUTH_TOKEN_LENGTH} characters"
            );
        }

        let endpoint = Endpoint::from_shared(endpoint)?;
        let is_https = endpoint.uri().scheme_str() == Some("https");
        if environment != "development" && !is_https {
            anyhow::bail!("{ENTERPRISE_GRPC_ENDPOINT_ENV} must use https outside development");
        }
        let endpoint = if is_https {
            endpoint.tls_config(ClientTlsConfig::new().with_webpki_roots())?
        } else {
            endpoint
        };
        let authorization = format!("Bearer {auth_token}").parse().map_err(|_| {
            anyhow::anyhow!("{ENTERPRISE_GRPC_AUTH_TOKEN_ENV} is not metadata-safe")
        })?;

        Ok(Some(Self {
            endpoint,
            authorization,
        }))
    }

    pub async fn connect(&self) -> anyhow::Result<Channel> {
        Ok(self.endpoint.connect().await?)
    }

    pub fn authorize<T>(&self, message: T) -> Request<T> {
        let mut request = Request::new(message);
        request
            .metadata_mut()
            .insert("authorization", self.authorization.clone());
        request
    }
}

fn normalized(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::EnterpriseGrpcConfig;

    const VALID_TOKEN: &str = "account-worker-enterprise-token-32";

    #[test]
    fn production_requires_https() {
        let error = EnterpriseGrpcConfig::from_values(
            "production",
            Some("http://enterprise:3031".to_string()),
            Some(VALID_TOKEN.to_string()),
        )
        .err()
        .expect("plaintext endpoint should fail");

        assert!(error.to_string().contains("must use https"));
    }

    #[test]
    fn configured_endpoint_requires_a_strong_auth_token() {
        let error = EnterpriseGrpcConfig::from_values(
            "development",
            Some("http://127.0.0.1:3031".to_string()),
            Some("short".to_string()),
        )
        .err()
        .expect("short token should fail");

        assert!(error.to_string().contains("at least 32"));
    }

    #[test]
    fn development_allows_authenticated_loopback_http() {
        let config = EnterpriseGrpcConfig::from_values(
            "development",
            Some("http://127.0.0.1:3031".to_string()),
            Some(VALID_TOKEN.to_string()),
        )
        .expect("development configuration should parse");

        assert!(config.is_some());
    }
}
