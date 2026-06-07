use crate::http::error::AppError;
use sqlx::Row;
use uuid::Uuid;

pub async fn resolve_network_valid(
    db: &sqlx::PgPool,
    tenant_id: Option<Uuid>,
    client_ip: Option<&str>,
) -> Result<Option<bool>, AppError> {
    let Some(tenant_id) = tenant_id else {
        return Ok(None);
    };

    let allowed_ips_row = sqlx::query("SELECT allowed_ips FROM tenants WHERE id = $1 LIMIT 1")
        .bind(tenant_id)
        .fetch_optional(db)
        .await?;

    let Some(row) = allowed_ips_row else {
        return Ok(None);
    };
    let allowed_ips: Option<Vec<String>> = row.get("allowed_ips");
    let Some(ips) = allowed_ips else {
        return Ok(None);
    };
    if ips.is_empty() {
        return Ok(None);
    }

    let mut valid = false;
    if let Some(ip_str) = client_ip {
        if let Ok(client_addr) = ip_str.parse::<std::net::IpAddr>() {
            for cidr in ips {
                if let Ok(net) = cidr.parse::<ipnet::IpNet>() {
                    if net.contains(&client_addr) {
                        valid = true;
                        break;
                    }
                } else if let Ok(allowed_ip) = cidr.parse::<std::net::IpAddr>() {
                    if allowed_ip == client_addr {
                        valid = true;
                        break;
                    }
                }
            }
        }
    }

    Ok(Some(valid))
}
