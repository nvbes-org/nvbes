use std::{
    net::{IpAddr, SocketAddr},
    time::Duration,
};

pub async fn pinned_https_client(
    destination: &str,
    allowed_destinations: &[&str],
    timeout: Duration,
) -> Result<(reqwest::Client, reqwest::Url), String> {
    let url = reqwest::Url::parse(destination)
        .map_err(|_| "outbound destination must be an absolute URL".to_string())?;
    if url.scheme() != "https"
        || url.username() != ""
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err(
            "outbound destination must be HTTPS without credentials or fragment".to_string(),
        );
    }
    let allowlisted = allowed_destinations.iter().any(|allowed| {
        reqwest::Url::parse(allowed).is_ok_and(|allowed_url| allowed_url.as_str() == url.as_str())
    });
    if !allowlisted {
        return Err("outbound destination is not allowlisted".to_string());
    }
    let host = url
        .host_str()
        .ok_or_else(|| "outbound destination is missing a host".to_string())?;
    let port = url.port_or_known_default().unwrap_or(443);
    let addresses = tokio::net::lookup_host((host, port))
        .await
        .map_err(|_| "outbound destination DNS resolution failed".to_string())?
        .collect::<Vec<SocketAddr>>();
    if addresses.is_empty() || addresses.iter().any(|address| is_special(address.ip())) {
        return Err(
            "outbound destination resolves to a private or special-purpose address".to_string(),
        );
    }

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(timeout)
        .https_only(true)
        .resolve_to_addrs(host, &addresses)
        .build()
        .map_err(|error| format!("outbound HTTP client initialization failed: {error}"))?;
    Ok((client, url))
}

fn is_special(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            let octets = ip.octets();
            ip.is_private()
                || ip.is_loopback()
                || ip.is_link_local()
                || ip.is_multicast()
                || ip.is_broadcast()
                || ip.is_documentation()
                || ip.is_unspecified()
                || octets[0] == 0
                || octets[0] >= 240
                || (octets[0] == 100 && (64..=127).contains(&octets[1]))
                || (octets[0] == 192 && octets[1] == 0 && octets[2] == 0)
                || (octets[0] == 192 && octets[1] == 88 && octets[2] == 99)
                || (octets[0] == 198 && matches!(octets[1], 18 | 19))
        }
        IpAddr::V6(ip) => {
            ip.to_ipv4_mapped()
                .is_some_and(|mapped| is_special(mapped.into()))
                || ip.is_loopback()
                || ip.is_unspecified()
                || ip.is_multicast()
                || ip.is_unique_local()
                || ip.is_unicast_link_local()
                || (ip.segments()[0] == 0x2001 && ip.segments()[1] == 0x0db8)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::is_special;

    #[test]
    fn private_and_special_addresses_are_blocked() {
        assert!(is_special("127.0.0.1".parse().unwrap()));
        assert!(is_special("169.254.169.254".parse().unwrap()));
        assert!(is_special("::1".parse().unwrap()));
        assert!(is_special("::ffff:127.0.0.1".parse().unwrap()));
        assert!(is_special("2001:db8::1".parse().unwrap()));
        assert!(!is_special("8.8.8.8".parse().unwrap()));
    }
}
