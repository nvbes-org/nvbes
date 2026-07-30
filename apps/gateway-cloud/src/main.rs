use std::net::SocketAddr;

use nvbes_core::{config::AppConfig, http::keep_alive};

#[path = "gateway.auth.rs"]
mod auth;
#[path = "gateway.billing_client.rs"]
mod billing_client;
#[path = "gateway.http.rs"]
mod http;
#[path = "gateway.identity_client.rs"]
mod identity_client;
#[path = "gateway.pb.rs"]
mod pb;
#[path = "gateway.schema.rs"]
mod schema;
#[path = "gateway.schema.enums.rs"]
mod schema_enums;
#[path = "gateway.schema.types.rs"]
mod schema_types;
#[path = "gateway.state.rs"]
mod state;

const GATEWAY_CLOUD_PORT_ENV: &str = "NVBES_GATEWAY_CLOUD_PORT";
const BILLING_GRPC_ENDPOINT_ENV: &str = "NVBES_BILLING_GRPC_ENDPOINT";
const IDENTITY_GRPC_ENDPOINT_ENV: &str = "NVBES_IDENTITY_GRPC_ENDPOINT";
const IDENTITY_CLIENT_ID_ENV: &str = "NVBES_GATEWAY_IDENTITY_CLIENT_ID";
const IDENTITY_CLIENT_SECRET_ENV: &str = "NVBES_GATEWAY_IDENTITY_CLIENT_SECRET";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut config = AppConfig::from_env().map_err(anyhow::Error::msg)?;
    config
        .resolve_from_secret_manager()
        .await
        .map_err(anyhow::Error::msg)?;
    let port = gateway_port(config.api_port)?;
    let billing_grpc_endpoint = billing_grpc_endpoint(config.api_port)?;
    let state = state::GatewayState {
        billing_grpc_endpoint,
        identity: identity_client::IdentityGrpcClient::new(
            identity_grpc_endpoint(config.api_port)?,
            required_env(IDENTITY_CLIENT_ID_ENV)?,
            required_env(IDENTITY_CLIENT_SECRET_ENV)?,
        )?,
    };
    let app = http::router(state).fallback(http::not_found);
    let addr: SocketAddr = format!("0.0.0.0:{port}").parse()?;
    let listener = keep_alive::bind_listener_with_keepalive(addr, 4096)?;

    tracing::info!(%addr, "Starting nvbes GraphQL gateway");
    axum::serve(listener, app)
        .await
        .map_err(anyhow::Error::from)?;
    Ok(())
}

fn gateway_port(default_api_port: u16) -> anyhow::Result<u16> {
    match std::env::var(GATEWAY_CLOUD_PORT_ENV) {
        Ok(port) => port
            .parse::<u16>()
            .map_err(|error| anyhow::anyhow!("{GATEWAY_CLOUD_PORT_ENV} is invalid: {error}")),
        Err(std::env::VarError::NotPresent) => default_api_port
            .checked_add(30)
            .ok_or_else(|| anyhow::anyhow!("Default GraphQL gateway port overflowed")),
        Err(error) => Err(anyhow::anyhow!(
            "{GATEWAY_CLOUD_PORT_ENV} could not be read: {error}"
        )),
    }
}

fn billing_grpc_endpoint(default_api_port: u16) -> anyhow::Result<String> {
    match std::env::var(BILLING_GRPC_ENDPOINT_ENV) {
        Ok(endpoint) if !endpoint.trim().is_empty() => Ok(endpoint),
        Ok(_) => Err(anyhow::anyhow!(
            "{BILLING_GRPC_ENDPOINT_ENV} must not be empty"
        )),
        Err(std::env::VarError::NotPresent) => {
            let port = default_api_port
                .checked_add(21)
                .ok_or_else(|| anyhow::anyhow!("Default Billing gRPC port overflowed"))?;
            Ok(format!("http://127.0.0.1:{port}"))
        }
        Err(error) => Err(anyhow::anyhow!(
            "{BILLING_GRPC_ENDPOINT_ENV} could not be read: {error}"
        )),
    }
}

fn identity_grpc_endpoint(default_api_port: u16) -> anyhow::Result<String> {
    match std::env::var(IDENTITY_GRPC_ENDPOINT_ENV) {
        Ok(endpoint) if !endpoint.trim().is_empty() => Ok(endpoint),
        Ok(_) => Err(anyhow::anyhow!(
            "{IDENTITY_GRPC_ENDPOINT_ENV} must not be empty"
        )),
        Err(std::env::VarError::NotPresent) => {
            let port = default_api_port
                .checked_add(10)
                .ok_or_else(|| anyhow::anyhow!("Default Identity gRPC port overflowed"))?;
            Ok(format!("http://127.0.0.1:{port}"))
        }
        Err(error) => Err(anyhow::anyhow!(
            "{IDENTITY_GRPC_ENDPOINT_ENV} could not be read: {error}"
        )),
    }
}

fn required_env(name: &'static str) -> anyhow::Result<String> {
    match std::env::var(name) {
        Ok(value) if !value.trim().is_empty() => Ok(value),
        Ok(_) => Err(anyhow::anyhow!("{name} must not be empty")),
        Err(error) => Err(anyhow::anyhow!("{name} could not be read: {error}")),
    }
}

#[cfg(test)]
mod tests {
    use super::{billing_grpc_endpoint, gateway_port, identity_grpc_endpoint, required_env};

    #[test]
    fn gateway_port_defaults_after_primary_api_port() {
        assert_eq!(gateway_port(3000).unwrap(), 3030);
    }

    #[test]
    fn billing_grpc_endpoint_defaults_to_billing_grpc_offset() {
        assert_eq!(
            billing_grpc_endpoint(3000).unwrap(),
            "http://127.0.0.1:3021"
        );
    }

    #[test]
    fn identity_grpc_endpoint_defaults_to_account_grpc_offset() {
        assert_eq!(
            identity_grpc_endpoint(4000).unwrap(),
            "http://127.0.0.1:4010"
        );
    }

    #[test]
    fn required_env_rejects_missing_values() {
        assert!(required_env("NVBES_GATEWAY_CLOUD_TEST_MISSING_ENV").is_err());
    }
}
