use reqwest::Url;
use std::net::IpAddr;

pub(crate) fn validate_database_url(value: &str, strict_mode: bool) -> Result<(), String> {
    let url = Url::parse(value).map_err(|_| "DATABASE_URL must be a valid URL".to_string())?;

    if !strict_mode {
        return Ok(());
    }

    let sslmode = url.query_pairs().find_map(|(key, value)| {
        if key == "sslmode" {
            Some(value.into_owned())
        } else {
            None
        }
    });

    match sslmode.as_deref() {
        Some("require") | Some("verify-full") => Ok(()),
        Some(other) => Err(format!(
            "DATABASE_URL sslmode must be require or verify-full outside development, got '{other}'"
        )),
        None => Err(
            "DATABASE_URL must specify sslmode=require or sslmode=verify-full outside development"
                .to_string(),
        ),
    }
}

pub(crate) fn validate_public_url(
    name: &str,
    value: &str,
    strict_mode: bool,
    require_https: bool,
) -> Result<(), String> {
    let url = Url::parse(value).map_err(|_| format!("{name} must be a valid URL"))?;

    if !strict_mode {
        return Ok(());
    }
    if require_https && url.scheme() != "https" {
        return Err(format!("{name} must use HTTPS outside development"));
    }

    let host = url
        .host_str()
        .ok_or_else(|| format!("{name} must include a host"))?;

    if host.eq_ignore_ascii_case("localhost") || host.ends_with(".localhost") {
        return Err(format!(
            "{name} cannot point to localhost outside development"
        ));
    }

    if let Ok(ip_addr) = host.parse::<IpAddr>() {
        let is_loopback = match ip_addr {
            IpAddr::V4(ipv4) => ipv4.is_loopback(),
            IpAddr::V6(ipv6) => ipv6.is_loopback(),
        };

        if is_loopback {
            return Err(format!(
                "{name} cannot point to loopback addresses outside development"
            ));
        }
    }

    Ok(())
}

pub(crate) fn validate_jwt_secret(value: &str, strict_mode: bool) -> Result<(), String> {
    if !strict_mode {
        return Ok(());
    }
    if value == "default-secret-change-me" {
        return Err(
            "NVBES_JWT_SECRET cannot use the placeholder secret outside development".to_string(),
        );
    }
    if value.len() < 32 {
        return Err(
            "NVBES_JWT_SECRET must be at least 32 characters long outside development".to_string(),
        );
    }

    Ok(())
}

pub(crate) fn validate_profiling_endpoint(value: &str) -> Result<(), String> {
    let url = Url::parse(value)
        .map_err(|_| "NVBES_PROFILING_ENDPOINT must be a valid URL".to_string())?;

    match url.scheme() {
        "http" | "https" => {}
        _ => return Err("NVBES_PROFILING_ENDPOINT must use HTTP or HTTPS".to_string()),
    }

    url.host_str()
        .ok_or_else(|| "NVBES_PROFILING_ENDPOINT must include a host".to_string())?;

    Ok(())
}

pub(crate) fn validate_webauthn_rp_id(value: &str, strict_mode: bool) -> Result<(), String> {
    let value = value.trim();

    if value.is_empty() {
        return Err("NVBES_WEBAUTHN_RP_ID must not be empty".to_string());
    }
    if !strict_mode {
        return Ok(());
    }
    if value.eq_ignore_ascii_case("localhost") {
        return Err("NVBES_WEBAUTHN_RP_ID cannot be localhost outside development".to_string());
    }

    if let Ok(ip_addr) = value.parse::<IpAddr>() {
        let is_loopback = match ip_addr {
            IpAddr::V4(ipv4) => ipv4.is_loopback(),
            IpAddr::V6(ipv6) => ipv6.is_loopback(),
        };

        if is_loopback {
            return Err(
                "NVBES_WEBAUTHN_RP_ID cannot point to loopback addresses outside development"
                    .to_string(),
            );
        }
    }

    Ok(())
}

#[cfg(test)]
#[path = "config.validation.urls.tests.rs"]
mod tests;
