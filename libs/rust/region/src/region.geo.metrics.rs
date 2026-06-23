use std::time::Duration;

use crate::geo::types::GeoResolution;

pub fn record_geo_resolution(resolution: &GeoResolution) {
    let labels = [
        ("source", resolution.source.as_str().to_string()),
        ("confidence", resolution.confidence.as_str().to_string()),
        ("private_network", resolution.private_network.to_string()),
        (
            "country",
            resolution
                .location
                .as_ref()
                .map(|location| location.country_code.clone())
                .unwrap_or_else(|| "unknown".to_string()),
        ),
    ];

    metrics::counter!("geo_resolutions_total", &labels).increment(1);
}

pub fn record_geo_cache_lookup(outcome: &str, duration: Duration) {
    let labels = [("outcome", outcome.to_string())];

    metrics::counter!("geo_cache_lookups_total", &labels).increment(1);
    metrics::histogram!("geo_cache_lookup_duration_seconds", &labels)
        .record(duration.as_secs_f64());
}

pub fn record_rdap_lookup(registry: &str, outcome: &str, duration: Duration) {
    let labels = [
        ("registry", registry.to_string()),
        ("outcome", outcome.to_string()),
    ];

    metrics::counter!("geo_rdap_lookups_total", &labels).increment(1);
    metrics::histogram!("geo_rdap_lookup_duration_seconds", &labels).record(duration.as_secs_f64());
}
