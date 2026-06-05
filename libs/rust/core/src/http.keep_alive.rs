use socket2::{Domain, Protocol, Socket, TcpKeepalive, Type};
use std::net::SocketAddr;
use std::time::Duration;

pub fn bind_listener_with_keepalive(
    addr: SocketAddr,
    backlog: u32,
) -> std::io::Result<tokio::net::TcpListener> {
    let domain = match addr {
        SocketAddr::V4(_) => Domain::IPV4,
        SocketAddr::V6(_) => Domain::IPV6,
    };

    let socket = Socket::new(domain, Type::STREAM, Some(Protocol::TCP))?;
    socket.set_reuse_address(true)?;
    socket.set_nonblocking(true)?;

    let keepalive_idle_secs = env_parse_u64("NVBES_TCP_KEEPALIVE_IDLE_SECS", 60);
    let keepalive_interval_secs = env_parse_u64("NVBES_TCP_KEEPALIVE_INTERVAL_SECS", 10);
    let keepalive_probes = env_parse_u32("NVBES_TCP_KEEPALIVE_PROBES", 3);

    let tcp_keepalive = TcpKeepalive::new()
        .with_time(Duration::from_secs(keepalive_idle_secs))
        .with_interval(Duration::from_secs(keepalive_interval_secs))
        .with_retries(keepalive_probes);

    socket.set_tcp_keepalive(&tcp_keepalive)?;

    tracing::info!(
        idle_secs = keepalive_idle_secs,
        interval_secs = keepalive_interval_secs,
        probes = keepalive_probes,
        "TCP keepalive configured on listener"
    );

    socket.bind(&addr.into())?;
    socket.listen(backlog as i32)?;

    tokio::net::TcpListener::from_std(socket.into())
}

fn env_parse_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn env_parse_u32(name: &str, default: u32) -> u32 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}
