use axum::Router;
use nvbes_core::config::AppConfig;
use sqlx::postgres::PgPool;
use std::sync::OnceLock;
use tokio::net::TcpListener;
use tokio::sync::Mutex;

pub(crate) fn test_database_url() -> String {
    std::env::var("DATABASE_URL")
        .or_else(|_| std::env::var("NVBES_DATABASE_URL"))
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/nvbes".to_string())
}

pub(crate) fn test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

pub(crate) async fn spawn_identity_server(state: nvbes_identity_api::app::AppState) -> String {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("test server should bind");
    let addr = listener.local_addr().expect("listener addr");
    let base_url = format!("http://{addr}");
    let router: Router = nvbes_identity_api::app::build_router(state);
    tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("test server should run");
    });
    base_url
}

pub(crate) async fn identity_state(pool: &PgPool) -> nvbes_identity_api::app::AppState {
    unsafe {
        std::env::set_var("NVBES_ENV", "development");
        std::env::set_var(
            "NVBES_WORKSPACE_ROOT",
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .ancestors()
                .nth(2)
                .expect("workspace root"),
        );
    }

    let config = nvbes_identity_api::app::AppConfig {
        database_url: test_database_url(),
        environment: "development".to_string(),
        app_name: "drive-auth-e2e-identity-test".to_string(),
        ..Default::default()
    };

    nvbes_identity_api::app::AppState::bootstrap(&config, pool.clone())
        .await
        .expect("identity app state bootstrap should succeed")
}

pub(crate) async fn drive_app() -> axum::Router {
    unsafe {
        std::env::set_var("NVBES_ENV", "development");
        std::env::set_var(
            "NVBES_WORKSPACE_ROOT",
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .ancestors()
                .nth(2)
                .expect("workspace root"),
        );
    }

    let config = AppConfig {
        database_url: test_database_url(),
        environment: "development".to_string(),
        app_name: "drive-auth-e2e-drive-test".to_string(),
        trusted_proxy_cidrs: vec!["127.0.0.1/32".to_string()],
        ..Default::default()
    };
    let db = crate::db::Database::connect(&config)
        .await
        .expect("drive database should connect");
    crate::app::build_app(config, db)
        .await
        .expect("drive app should build")
}
