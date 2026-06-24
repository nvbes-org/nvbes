use nvbes_region::geo::{
    GeoLookupRequest, GeoResolution, GeoResolver, IpIntelligenceHttpClient, RdapClient,
    cache_ip_intelligence_tx, cached_ip_intelligence_tx, cached_remote_lookup_tx,
    is_private_or_special_ip, load_personal_geo_database_tx, parse_ip, providers_from_specs,
};
use sqlx::{Postgres, Transaction};
use std::time::Duration;

pub(super) async fn resolve_checkout_geo(
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
    let remote_lookup = if should_fetch_remote && cached_lookup.is_none() {
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
    let remote_lookup = cached_lookup.as_ref().or(remote_lookup.as_ref());

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

pub(super) fn checkout_geo_risk(
    resolution: &GeoResolution,
    stored_country: Option<&str>,
) -> CheckoutGeoRisk {
    let resolved_country = resolution
        .location
        .as_ref()
        .map(|location| location.country_code.as_str());
    let mut score = 5.0;
    let mut factors = Vec::new();

    if resolution.private_network {
        score += 10.0;
        factors.push("private_or_special_ip".to_string());
    }
    if matches!(resolution.confidence.as_str(), "none" | "low") {
        score += 15.0;
        factors.push("low_geo_confidence".to_string());
    }
    if resolution.source.as_str() == "fallback" {
        score += 10.0;
        factors.push("geo_unresolved".to_string());
    }
    if resolution.risk_score >= 80 {
        score += 25.0;
        factors.push("high_risk_network".to_string());
    } else if resolution.risk_score >= 60 {
        score += 12.0;
        factors.push("elevated_risk_network".to_string());
    }
    if let (Some(resolved), Some(stored)) = (resolved_country, stored_country)
        && resolved != stored
    {
        score += 25.0;
        factors.push("geo_country_mismatch".to_string());
    }

    CheckoutGeoRisk {
        score: f64::min(score, 100.0),
        factors,
    }
}

pub(super) struct CheckoutGeoRisk {
    pub score: f64,
    pub factors: Vec<String>,
}
