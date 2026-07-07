use nvbes_core::config::AppConfig;
use nvbes_region::geo::{
    IpIntelligenceHttpClient, IpIntelligenceLookup, cache_ip_intelligence_tx,
    cached_ip_intelligence_tx, is_private_or_special_ip, parse_ip, providers_from_specs,
};
use sqlx::{Postgres, Transaction};
use std::net::IpAddr;
use std::time::Duration;

pub(crate) async fn ensure_ip_intelligence_tx(
    tx: &mut Transaction<'_, Postgres>,
    config: &AppConfig,
    ip: Option<&str>,
) -> Result<Option<IpIntelligenceLookup>, sqlx::Error> {
    let Some(ip) = ip.and_then(parse_ip) else {
        return Ok(None);
    };
    if is_private_or_special_ip(ip) {
        return Ok(None);
    }
    if let Some(lookup) = cached_ip_intelligence_tx(tx, ip).await? {
        return Ok(Some(lookup));
    }
    fetch_and_cache_ip_intelligence(tx, config, ip).await
}

async fn fetch_and_cache_ip_intelligence(
    tx: &mut Transaction<'_, Postgres>,
    config: &AppConfig,
    ip: IpAddr,
) -> Result<Option<IpIntelligenceLookup>, sqlx::Error> {
    if config.ip_intelligence_provider_specs.is_empty() {
        return Ok(None);
    }
    let providers = match providers_from_specs(&config.ip_intelligence_provider_specs) {
        Ok(providers) => providers,
        Err(error) => {
            tracing::warn!(%error, "invalid ip intelligence provider configuration");
            return Ok(None);
        }
    };

    let client =
        IpIntelligenceHttpClient::new(nvbes_core::security::pinned_http_client(), providers);
    let lookup = match tokio::time::timeout(
        Duration::from_secs(config.ip_intelligence_timeout_secs),
        client.lookup(ip),
    )
    .await
    {
        Ok(Ok(lookup)) => lookup,
        Ok(Err(error)) => {
            tracing::warn!(%error, "ip intelligence lookup failed");
            None
        }
        Err(_) => {
            tracing::warn!("ip intelligence lookup timed out");
            None
        }
    };

    if let Some(lookup) = &lookup {
        let expires_at =
            chrono::Utc::now() + chrono::Duration::hours(config.ip_intelligence_cache_ttl_hours);
        cache_ip_intelligence_tx(tx, lookup, Some(expires_at)).await?;
    }
    Ok(lookup)
}
