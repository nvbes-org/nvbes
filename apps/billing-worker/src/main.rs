use tokio::sync::watch;

#[path = "billing.worker.config.rs"]
mod config;
#[path = "billing.worker.database.rs"]
mod database;
#[path = "billing.worker.dispatcher.rs"]
mod dispatcher;
#[path = "billing.worker.email.rs"]
mod email;
#[path = "billing.worker.health.rs"]
mod health;
#[path = "billing.worker.metrics.rs"]
mod metrics;
#[path = "billing.worker.queue.rs"]
mod queue;
#[path = "billing.worker.queue.trigger.rs"]
mod queue_trigger;
#[path = "billing.worker.state.rs"]
mod state;
#[path = "billing.worker.synthetic.rs"]
mod synthetic;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let command: Vec<String> = std::env::args().skip(1).collect();

    if matches!(command.as_slice(), [action] if action == "deployment-bootstrap") {
        return run_deployment_bootstrap().await;
    }

    if matches!(command.as_slice(), [action] if action == "migrate") {
        let config = config::BillingWorkerConfig::from_env()?;
        let pool = database::connect(&config.database_url, 2).await?;
        database::migrate(&pool).await?;
        println!("billing database migrations applied");
        return Ok(());
    }

    if matches!(command.as_slice(), [action] if action == "synthetic-smoke") {
        let config = config::BillingWorkerConfig::from_env()?;
        let pool = database::connect(&config.database_url, 2).await?;
        let result = synthetic::run(&pool, &config).await?;
        println!("{}", serde_json::to_string(&result)?);
        return Ok(());
    }

    let config = config::BillingWorkerConfig::from_env()?;
    if matches!(command.as_slice(), [action] if action == "validate-runtime") {
        database::connect_lazy(&config.database_url, 2)?;
        println!("billing worker runtime configuration is valid");
        return Ok(());
    }

    if !command.is_empty() && command[0] != "serve" {
        anyhow::bail!(
            "usage: nvbes-billing-worker [serve|migrate|validate-runtime|synthetic-smoke|deployment-bootstrap]"
        );
    }

    let _metrics_handle = metrics::install();
    nvbes_observability::install_safe_panic_hook();
    nvbes_observability::init_tracing_with_config(
        nvbes_observability::TracingConfig {
            environment: &config.environment,
            otlp_endpoint: config.otlp_endpoint.as_deref(),
            otlp_authorization_header: config.otlp_authorization_header.as_deref(),
            protocol: nvbes_observability::OtlpProtocol::Grpc,
        },
        "billing-worker",
    );

    let db = database::connect(&config.database_url, 5).await?;
    let state = state::BillingWorkerState::new(config.clone(), db.clone()).await?;

    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let local_receiver = state.take_local_dispatch_receiver().await;
    let local_dispatcher = local_receiver.map(|receiver| {
        tokio::spawn(dispatcher::run_local(
            state.clone(),
            receiver,
            shutdown_rx.clone(),
        ))
    });

    let app = axum::Router::new()
        .merge(health::router(state.clone()))
        .merge(metrics::router(state.clone()))
        .merge(queue_trigger::router(state.clone()));

    let listener = tokio::net::TcpListener::bind(state.config.http_bind_addr).await?;
    tracing::info!(
        bind_addr = %state.config.http_bind_addr,
        environment = %state.config.environment,
        dispatch_mode = ?state.config.dispatch_mode,
        "starting nvbes billing worker"
    );

    let server =
        axum::serve(listener, app).with_graceful_shutdown(server_shutdown(shutdown_rx.clone()));

    tokio::select! {
        result = server => result?,
        _ = process_shutdown_signal() => {},
    }

    let _ = shutdown_tx.send(true);
    if let Some(dispatcher) = local_dispatcher {
        let _ = dispatcher.await;
    }

    db.close().await;
    Ok(())
}

async fn run_deployment_bootstrap() -> anyhow::Result<()> {
    let bind_addr: std::net::SocketAddr = std::env::var("NVBES_BILLING_HTTP_BIND_ADDR")
        .or_else(|_| std::env::var("NVBES_BILLING_BIND_ADDR"))
        .unwrap_or_else(|_| "0.0.0.0:8080".to_string())
        .parse()
        .map_err(|error| anyhow::anyhow!("NVBES_BILLING_HTTP_BIND_ADDR is invalid: {error}"))?;
    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    let router = axum::Router::new().route(
        "/health/live",
        axum::routing::get(|| async { axum::http::StatusCode::NO_CONTENT }),
    );

    axum::serve(listener, router)
        .with_graceful_shutdown(process_shutdown_signal())
        .await?;
    Ok(())
}

async fn server_shutdown(mut shutdown: watch::Receiver<bool>) {
    while !*shutdown.borrow() {
        if shutdown.changed().await.is_err() {
            break;
        }
    }
}

async fn process_shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let mut terminate = signal(SignalKind::terminate()).expect("SIGTERM handler must register");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {},
            _ = terminate.recv() => {},
        }
    }
    #[cfg(not(unix))]
    let _ = tokio::signal::ctrl_c().await;
}
