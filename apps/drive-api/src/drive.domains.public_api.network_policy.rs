use nvbes_region::geo::{GeoNetworkKind, GeoResolution, resolve_cached_geo_tx};
use sqlx::PgPool;

pub struct PublicApiNetworkBlock {
    pub reason: &'static str,
}

pub async fn public_api_network_block(
    db: &PgPool,
    ip: Option<&str>,
) -> Result<Option<PublicApiNetworkBlock>, sqlx::Error> {
    let mut tx = db.begin().await?;
    let resolution = resolve_cached_geo_tx(&mut tx, ip, None, None).await?;
    tx.commit().await?;
    Ok(block_decision(&resolution))
}

fn block_decision(resolution: &GeoResolution) -> Option<PublicApiNetworkBlock> {
    let reason = if matches!(resolution.network_kind, GeoNetworkKind::Tor) {
        Some("tor")
    } else if matches!(resolution.network_kind, GeoNetworkKind::Proxy) {
        Some("proxy")
    } else if matches!(resolution.network_kind, GeoNetworkKind::Vpn) {
        Some("vpn")
    } else if resolution.risk_score >= 90 {
        Some("high_geo_risk_score")
    } else {
        None
    }?;

    Some(PublicApiNetworkBlock { reason })
}

#[cfg(test)]
mod tests {
    use nvbes_region::geo::{GeoConfidence, GeoEvidenceSource, GeoNetworkKind, GeoResolution};

    use super::block_decision;

    #[test]
    fn blocks_anonymous_networks_but_not_plain_datacenter() {
        assert_eq!(block_reason(GeoNetworkKind::Vpn, 90), Some("vpn"));
        assert_eq!(block_reason(GeoNetworkKind::Proxy, 85), Some("proxy"));
        assert_eq!(block_reason(GeoNetworkKind::Tor, 95), Some("tor"));
        assert_eq!(block_reason(GeoNetworkKind::Datacenter, 70), None);
    }

    #[test]
    fn blocks_extreme_score_even_when_kind_is_unknown() {
        assert_eq!(
            block_reason(GeoNetworkKind::Unknown, 95),
            Some("high_geo_risk_score")
        );
        assert_eq!(block_reason(GeoNetworkKind::Unknown, 50), None);
    }

    fn block_reason(kind: GeoNetworkKind, score: u8) -> Option<&'static str> {
        block_decision(&GeoResolution {
            location: None,
            confidence: GeoConfidence::Medium,
            source: GeoEvidenceSource::RemoteLookup,
            ip: None,
            private_network: false,
            network_kind: kind,
            risk_score: score,
            risk_labels: vec![kind.as_str().to_string()],
            evidence: Vec::new(),
        })
        .map(|block| block.reason)
    }
}
