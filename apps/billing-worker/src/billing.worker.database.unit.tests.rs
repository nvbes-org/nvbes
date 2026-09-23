use super::{connect, connect_lazy, migrate};

#[tokio::test]
async fn connect_lazy_accepts_postgres_url_without_connecting() {
    let pool = connect_lazy("postgres://localhost/unused", 2).expect("lazy pool");
    pool.close().await;
}

#[tokio::test]
async fn connect_and_migrate_when_postgres_is_available() {
    if !postgres_reachable() {
        return;
    }
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@127.0.0.1:5432/nvbes_billing".into());
    let pool = connect(&url, 2).await.expect("connect");
    migrate(&pool).await.expect("migrate");
    pool.close().await;
}

fn postgres_reachable() -> bool {
    std::net::TcpStream::connect_timeout(
        &"127.0.0.1:5432".parse().unwrap(),
        std::time::Duration::from_millis(200),
    )
    .is_ok()
}
