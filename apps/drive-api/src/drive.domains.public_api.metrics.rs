use super::geo::ApiRequestGeo;

pub const GEO_REQUESTS_METRIC: &str = "drive_public_api_geo_requests_total";
pub const HIGH_RISK_REQUESTS_METRIC: &str = "drive_public_api_high_risk_requests_total";
pub const NETWORK_POLICY_BLOCKS_METRIC: &str = "drive_public_api_network_policy_blocks_total";

pub fn record_api_request_geo(status_code: i32, geo: &ApiRequestGeo) {
    let risk_bucket = risk_bucket(geo.risk_score);
    let labels = [
        ("status", status_bucket(status_code).to_string()),
        ("network_kind", geo.network_kind.to_string()),
        ("risk_bucket", risk_bucket.to_string()),
    ];

    metrics::counter!(GEO_REQUESTS_METRIC, &labels).increment(1);

    if geo.risk_score >= 80 {
        metrics::counter!(
            HIGH_RISK_REQUESTS_METRIC,
            &[("network_kind", geo.network_kind.to_string())]
        )
        .increment(1);
    }
}

pub fn record_network_policy_block(reason: &str, mode: &str) {
    metrics::counter!(
        NETWORK_POLICY_BLOCKS_METRIC,
        &[("reason", reason.to_string()), ("mode", mode.to_string())]
    )
    .increment(1);
}

fn status_bucket(status_code: i32) -> &'static str {
    match status_code {
        200..=299 => "2xx",
        300..=399 => "3xx",
        400..=499 => "4xx",
        500..=599 => "5xx",
        _ => "other",
    }
}

fn risk_bucket(score: i16) -> &'static str {
    match score {
        0..=29 => "low",
        30..=59 => "medium",
        60..=79 => "elevated",
        _ => "high",
    }
}

#[cfg(test)]
mod tests {
    use super::{
        GEO_REQUESTS_METRIC, HIGH_RISK_REQUESTS_METRIC, NETWORK_POLICY_BLOCKS_METRIC, risk_bucket,
        status_bucket,
    };

    #[test]
    fn buckets_status_and_geo_risk() {
        assert_eq!(status_bucket(200), "2xx");
        assert_eq!(status_bucket(403), "4xx");
        assert_eq!(status_bucket(503), "5xx");
        assert_eq!(risk_bucket(15), "low");
        assert_eq!(risk_bucket(70), "elevated");
        assert_eq!(risk_bucket(90), "high");
    }

    #[test]
    fn public_api_geo_metric_names_are_stable() {
        assert_eq!(GEO_REQUESTS_METRIC, "drive_public_api_geo_requests_total");
        assert_eq!(
            HIGH_RISK_REQUESTS_METRIC,
            "drive_public_api_high_risk_requests_total"
        );
        assert_eq!(
            NETWORK_POLICY_BLOCKS_METRIC,
            "drive_public_api_network_policy_blocks_total"
        );
    }
}
