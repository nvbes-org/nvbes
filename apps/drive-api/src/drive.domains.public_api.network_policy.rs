use nvbes_region::geo::{GeoNetworkKind, GeoResolution, resolve_cached_geo_tx};
use sqlx::{PgPool, Row};
use uuid::Uuid;

pub struct PublicApiNetworkBlock {
    pub reason: &'static str,
    pub mode: PublicApiNetworkPolicyMode,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PublicApiNetworkPolicyMode {
    Enforce,
    MonitorOnly,
    Disabled,
}

impl PublicApiNetworkPolicyMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Enforce => "enforce",
            Self::MonitorOnly => "monitor_only",
            Self::Disabled => "disabled",
        }
    }

    fn from_db(value: &str) -> Self {
        match value {
            "monitor_only" => Self::MonitorOnly,
            "disabled" => Self::Disabled,
            _ => Self::Enforce,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PublicApiNetworkPolicy {
    pub mode: PublicApiNetworkPolicyMode,
    pub block_vpn: bool,
    pub block_proxy: bool,
    pub block_tor: bool,
    pub block_datacenter: bool,
    pub high_risk_score_threshold: u8,
}

impl Default for PublicApiNetworkPolicy {
    fn default() -> Self {
        Self {
            mode: PublicApiNetworkPolicyMode::Enforce,
            block_vpn: true,
            block_proxy: true,
            block_tor: true,
            block_datacenter: false,
            high_risk_score_threshold: 90,
        }
    }
}

pub async fn public_api_network_block(
    db: &PgPool,
    workspace_id: Uuid,
    ip: Option<&str>,
) -> Result<Option<PublicApiNetworkBlock>, sqlx::Error> {
    let mut tx = db.begin().await?;
    let policy = fetch_network_policy(&mut tx, workspace_id).await?;
    if matches!(policy.mode, PublicApiNetworkPolicyMode::Disabled) {
        tx.commit().await?;
        return Ok(None);
    }
    if is_allowlisted(&mut tx, workspace_id, ip).await? {
        tx.commit().await?;
        return Ok(None);
    }
    let resolution = resolve_cached_geo_tx(&mut tx, ip, None, None).await?;
    tx.commit().await?;
    Ok(block_decision(&policy, &resolution))
}

async fn fetch_network_policy(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: Uuid,
) -> Result<PublicApiNetworkPolicy, sqlx::Error> {
    let Some(row) = sqlx::query(
        r#"
        SELECT
          public_api_network_policy_mode,
          public_api_block_vpn,
          public_api_block_proxy,
          public_api_block_tor,
          public_api_block_datacenter,
          public_api_high_risk_score_threshold
        FROM workspace_policies
        WHERE workspace_id = $1
        "#,
    )
    .bind(workspace_id)
    .fetch_optional(&mut **tx)
    .await?
    else {
        return Ok(PublicApiNetworkPolicy::default());
    };

    let threshold: i16 = row.get("public_api_high_risk_score_threshold");
    Ok(PublicApiNetworkPolicy {
        mode: PublicApiNetworkPolicyMode::from_db(row.get("public_api_network_policy_mode")),
        block_vpn: row.get("public_api_block_vpn"),
        block_proxy: row.get("public_api_block_proxy"),
        block_tor: row.get("public_api_block_tor"),
        block_datacenter: row.get("public_api_block_datacenter"),
        high_risk_score_threshold: threshold.clamp(0, 100) as u8,
    })
}

async fn is_allowlisted(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: Uuid,
    ip: Option<&str>,
) -> Result<bool, sqlx::Error> {
    let Some(ip) = ip else {
        return Ok(false);
    };

    sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
          SELECT 1
          FROM public_api_network_allowlist
          WHERE workspace_id = $1
            AND $2::inet << cidr
            AND (expires_at IS NULL OR expires_at > NOW())
        )
        "#,
    )
    .bind(workspace_id)
    .bind(ip)
    .fetch_one(&mut **tx)
    .await
}

fn block_decision(
    policy: &PublicApiNetworkPolicy,
    resolution: &GeoResolution,
) -> Option<PublicApiNetworkBlock> {
    if matches!(policy.mode, PublicApiNetworkPolicyMode::Disabled) {
        return None;
    }

    let reason = if policy.block_tor && matches!(resolution.network_kind, GeoNetworkKind::Tor) {
        Some("tor")
    } else if policy.block_proxy && matches!(resolution.network_kind, GeoNetworkKind::Proxy) {
        Some("proxy")
    } else if policy.block_vpn && matches!(resolution.network_kind, GeoNetworkKind::Vpn) {
        Some("vpn")
    } else if policy.block_datacenter
        && matches!(resolution.network_kind, GeoNetworkKind::Datacenter)
    {
        Some("datacenter")
    } else if resolution.risk_score >= policy.high_risk_score_threshold {
        Some("high_geo_risk_score")
    } else {
        None
    }?;

    Some(PublicApiNetworkBlock {
        reason,
        mode: policy.mode,
    })
}

#[cfg(test)]
mod tests {
    use nvbes_region::geo::{GeoConfidence, GeoEvidenceSource, GeoNetworkKind, GeoResolution};

    use super::{PublicApiNetworkPolicy, PublicApiNetworkPolicyMode, block_decision};

    #[test]
    fn blocks_anonymous_networks_but_not_plain_datacenter() {
        let policy = PublicApiNetworkPolicy::default();
        assert_eq!(block_reason(&policy, GeoNetworkKind::Vpn, 90), Some("vpn"));
        assert_eq!(
            block_reason(&policy, GeoNetworkKind::Proxy, 85),
            Some("proxy")
        );
        assert_eq!(block_reason(&policy, GeoNetworkKind::Tor, 95), Some("tor"));
        assert_eq!(block_reason(&policy, GeoNetworkKind::Datacenter, 70), None);
    }

    #[test]
    fn blocks_extreme_score_even_when_kind_is_unknown() {
        let policy = PublicApiNetworkPolicy::default();
        assert_eq!(
            block_reason(&policy, GeoNetworkKind::Unknown, 95),
            Some("high_geo_risk_score")
        );
        assert_eq!(block_reason(&policy, GeoNetworkKind::Unknown, 50), None);
    }

    #[test]
    fn policy_can_disable_specific_network_kinds_or_monitor_only() {
        let mut policy = PublicApiNetworkPolicy {
            block_vpn: false,
            block_datacenter: true,
            ..PublicApiNetworkPolicy::default()
        };
        assert_eq!(block_reason(&policy, GeoNetworkKind::Vpn, 40), None);
        assert_eq!(
            block_reason(&policy, GeoNetworkKind::Datacenter, 40),
            Some("datacenter")
        );

        policy.mode = PublicApiNetworkPolicyMode::MonitorOnly;
        let block = block_decision(&policy, &resolution(GeoNetworkKind::Proxy, 80))
            .expect("proxy should be observed in monitor mode");
        assert_eq!(block.reason, "proxy");
        assert_eq!(block.mode, PublicApiNetworkPolicyMode::MonitorOnly);
    }

    fn block_reason(
        policy: &PublicApiNetworkPolicy,
        kind: GeoNetworkKind,
        score: u8,
    ) -> Option<&'static str> {
        block_decision(policy, &resolution(kind, score)).map(|block| block.reason)
    }

    fn resolution(kind: GeoNetworkKind, score: u8) -> GeoResolution {
        GeoResolution {
            location: None,
            confidence: GeoConfidence::Medium,
            source: GeoEvidenceSource::RemoteLookup,
            ip: None,
            private_network: false,
            network_kind: kind,
            risk_score: score,
            risk_labels: vec![kind.as_str().to_string()],
            evidence: Vec::new(),
        }
    }
}
