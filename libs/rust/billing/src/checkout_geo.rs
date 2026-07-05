use nvbes_region::geo::{
    GeoLookupRequest, GeoResolution, GeoResolver, IpIntelligenceHttpClient, MaxMindGeoLiteConfig,
    MaxMindGeoLiteWebClient, RdapClient, cache_ip_intelligence_tx, cache_maxmind_geolite_web_tx,
    cached_ip_intelligence_tx, cached_remote_lookup_tx, is_private_or_special_ip,
    load_personal_geo_database_tx, parse_ip, providers_from_specs,
};
use sqlx::{Postgres, Transaction};
use std::time::Duration;

pub async fn resolve_checkout_geo(
    tx: &mut Transaction<'_, Postgres>,
    config: &nvbes_core::config::AppConfig,
    ip: Option<&str>,
    trusted_country_header: Option<&str>,
    stored_profile_country: Option<&str>,
) -> Result<GeoResolution, sqlx::Error> {
    let parsed_ip = ip.and_then(parse_ip);
    let personal_database = load_personal_geo_database_tx(tx).await?;
    let should_fetch_remote = parsed_ip
        .map(|ip| !is_private_or_special_ip(ip) && trusted_country_header.is_none())
        .unwrap_or(false);
    let cached_lookup = match parsed_ip.filter(|_| should_fetch_remote) {
        Some(ip) => cached_remote_lookup_tx(tx, ip).await?,
        None => None,
    };
    let cached_intelligence = match parsed_ip.filter(|ip| !is_private_or_special_ip(*ip)) {
        Some(ip) => cached_ip_intelligence_tx(tx, ip).await?,
        None => None,
    };
    let remote_intelligence = if cached_intelligence.is_none() {
        match parsed_ip.filter(|ip| !is_private_or_special_ip(*ip)) {
            Some(ip) => fetch_and_cache_ip_intelligence(tx, config, ip).await?,
            None => None,
        }
    } else {
        None
    };
    let maxmind_lookup = if should_fetch_remote && cached_lookup.is_none() {
        match parsed_ip {
            Some(ip) => fetch_and_cache_maxmind_geolite(tx, config, ip).await?,
            None => None,
        }
    } else {
        None
    };
    let remote_lookup =
        if should_fetch_remote && cached_lookup.is_none() && maxmind_lookup.is_none() {
            match tokio::time::timeout(
                Duration::from_secs(3),
                RdapClient::new(nvbes_core::security::pinned_http_client())
                    .lookup(parsed_ip.expect("checked above")),
            )
            .await
            {
                Ok(Ok(lookup)) => lookup,
                Ok(Err(_)) | Err(_) => None,
            }
        } else {
            None
        };
    let maxmind_lookup = maxmind_lookup
        .as_ref()
        .map(|lookup| lookup.as_rdap_lookup());
    let remote_lookup = cached_lookup
        .as_ref()
        .or(maxmind_lookup.as_ref())
        .or(remote_lookup.as_ref());

    Ok(
        GeoResolver::new(personal_database).resolve(GeoLookupRequest {
            ip: parsed_ip,
            trusted_country_header,
            remote_lookup,
            network_intelligence: cached_intelligence
                .as_ref()
                .or(remote_intelligence.as_ref()),
            stored_profile_country,
            ..GeoLookupRequest::default()
        }),
    )
}

async fn fetch_and_cache_maxmind_geolite(
    tx: &mut Transaction<'_, Postgres>,
    config: &nvbes_core::config::AppConfig,
    ip: std::net::IpAddr,
) -> Result<Option<nvbes_region::geo::MaxMindGeoLiteWebLookup>, sqlx::Error> {
    if !config.maxmind_geolite_web_enabled {
        return Ok(None);
    }
    let Some(account_id) = config.maxmind_account_id.clone() else {
        return Ok(None);
    };
    let Some(license_key) = config.maxmind_license_key.clone() else {
        return Ok(None);
    };
    let client = match MaxMindGeoLiteWebClient::new(
        nvbes_core::security::pinned_http_client(),
        MaxMindGeoLiteConfig {
            account_id,
            license_key,
            eula_accepted: config.maxmind_geolite_eula_accepted,
        },
    ) {
        Ok(client) => client,
        Err(error) => {
            tracing::warn!(%error, "invalid MaxMind GeoLite web configuration");
            return Ok(None);
        }
    };
    let lookup = match tokio::time::timeout(
        Duration::from_secs(config.maxmind_web_timeout_secs),
        client.lookup(ip),
    )
    .await
    {
        Ok(Ok(lookup)) => lookup,
        Ok(Err(error)) => {
            tracing::warn!(%error, "MaxMind GeoLite web lookup failed");
            None
        }
        Err(_) => {
            tracing::warn!("MaxMind GeoLite web lookup timed out");
            None
        }
    };

    if let Some(lookup) = &lookup {
        let expires_at =
            chrono::Utc::now() + chrono::Duration::hours(config.maxmind_web_cache_ttl_hours);
        cache_maxmind_geolite_web_tx(tx, lookup, expires_at).await?;
    }
    Ok(lookup)
}

async fn fetch_and_cache_ip_intelligence(
    tx: &mut Transaction<'_, Postgres>,
    config: &nvbes_core::config::AppConfig,
    ip: std::net::IpAddr,
) -> Result<Option<nvbes_region::geo::IpIntelligenceLookup>, sqlx::Error> {
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
