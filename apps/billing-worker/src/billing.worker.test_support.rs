use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;

/// True when a PostgreSQL endpoint from the coverage/security test env answers TCP.
///
/// Prefer `DATABASE_URL` / `NVBES_SECURITY_TEST_DATABASE_URL` over a hardcoded
/// `:5432` probe so local docker wrappers (port 15432) contribute branch evidence.
pub fn postgres_reachable() -> bool {
    let candidate = std::env::var("DATABASE_URL")
        .or_else(|_| std::env::var("NVBES_SECURITY_TEST_DATABASE_URL"))
        .or_else(|_| std::env::var("NVBES_BILLING_DATABASE_URL"))
        .unwrap_or_else(|_| "postgres://127.0.0.1:5432/postgres".into());
    let Some(addr) = postgres_socket_addr(&candidate) else {
        return false;
    };
    TcpStream::connect_timeout(&addr, Duration::from_millis(200)).is_ok()
}

fn postgres_socket_addr(database_url: &str) -> Option<SocketAddr> {
    let without_scheme = database_url.split("://").nth(1)?;
    let authority = without_scheme.split('/').next()?;
    let host_port = authority.rsplit('@').next()?;
    let (host, port) = match host_port.rsplit_once(':') {
        Some((host, port)) => (
            host.trim_start_matches('[').trim_end_matches(']'),
            port.parse().ok()?,
        ),
        None => (host_port, 5432_u16),
    };
    (host, port).to_socket_addrs().ok()?.next()
}

#[cfg(test)]
mod tests {
    use super::postgres_socket_addr;

    #[test]
    fn parses_loopback_url_with_explicit_port() {
        let addr = postgres_socket_addr(
            "postgres://postgres:postgres@127.0.0.1:15432/nvbes_coverage_test",
        )
        .expect("parse");
        assert_eq!(addr.port(), 15432);
    }

    #[test]
    fn defaults_missing_port_to_5432() {
        let addr = postgres_socket_addr("postgres://postgres@127.0.0.1/nvbes").expect("parse");
        assert_eq!(addr.port(), 5432);
    }
}
