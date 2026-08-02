#[path = "account.worker.closure.db.rs"]
mod closure_db;
#[path = "account.worker.config.rs"]
mod config;
#[path = "account.worker.db.rs"]
mod db;
#[path = "account.worker.identity.rs"]
mod identity;
#[path = "account.worker.storage.rs"]
mod storage;
#[path = "account.worker.types.rs"]
mod types;

use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "nvbes_account_worker=info".into()),
        )
        .init();
    let config = config::AccountWorkerConfig::from_env().map_err(anyhow::Error::msg)?;
    let db = PgPoolOptions::new()
        .max_connections(config.database_max_connections)
        .connect(&config.database_url)
        .await?;
    sqlx::migrate!("../account-service-next/migrations")
        .run(&db)
        .await?;
    let identity = identity::IdentityClient::new(
        &config.identity_service_base_url,
        config.identity_internal_token.clone(),
    )?;
    let cloud = identity::ClosureClient::new(
        &config.cloud_service_base_url,
        config.cloud_internal_token.clone(),
        identity::ClosureService::Cloud,
    )?;
    let billing = identity::ClosureClient::new(
        &config.billing_service_base_url,
        config.billing_internal_token.clone(),
        identity::ClosureService::Billing,
    )?;
    let avatar_storage =
        nvbes_account_service::app::build_avatar_storage(&config.avatar_storage).await;
    let storage = storage::AccountStorage::new(avatar_storage);
    tracing::info!("Account worker started");

    loop {
        tokio::select! {
            result = process_next(&db, &identity, &cloud, &billing, &storage, &config) => {
                match result {
                    Ok(true) => continue,
                    Ok(false) => tokio::time::sleep(config.poll_interval).await,
                    Err(error) => {
                        tracing::warn!(%error, "Account worker iteration failed");
                        tokio::time::sleep(config.poll_interval).await;
                    }
                }
            }
            signal = tokio::signal::ctrl_c() => {
                signal?;
                tracing::info!("Account worker stopped");
                return Ok(());
            }
        }
    }
}

async fn process_next(
    db_pool: &sqlx::PgPool,
    identity: &identity::IdentityClient,
    cloud: &identity::ClosureClient,
    billing: &identity::ClosureClient,
    storage: &storage::AccountStorage,
    config: &config::AccountWorkerConfig,
) -> anyhow::Result<bool> {
    if let Some(closure) =
        closure_db::claim(db_pool, config.claim_timeout, config.max_attempts).await?
    {
        let result = match closure.participant {
            types::ClosureParticipant::Cloud => cloud.close_account(&closure.payload).await,
            types::ClosureParticipant::Billing => billing.close_account(&closure.payload).await,
            types::ClosureParticipant::Identity => identity.close_account(&closure.payload).await,
            types::ClosureParticipant::Account => storage
                .delete_avatar(closure.avatar_object_key.as_deref())
                .await
                .map_err(|_| identity::DispatchError::storage()),
        };
        match result {
            Ok(()) => {
                if closure.participant == types::ClosureParticipant::Account {
                    closure_db::complete_account(db_pool, &closure).await?;
                } else {
                    closure_db::complete_participant(db_pool, &closure).await?;
                }
                tracing::info!(
                    event_id = %closure.event_id,
                    saga_id = %closure.saga_id,
                    principal_id = %closure.principal_id,
                    participant = closure.participant.as_str(),
                    "Account closure participant completed"
                );
            }
            Err(error) => {
                closure_db::fail(
                    db_pool,
                    &closure,
                    error.is_retryable(),
                    error.code(),
                    config.max_attempts,
                )
                .await?;
                tracing::warn!(
                    event_id = %closure.event_id,
                    saga_id = %closure.saga_id,
                    principal_id = %closure.principal_id,
                    participant = closure.participant.as_str(),
                    retryable = error.is_retryable(),
                    code = error.code(),
                    "Account closure failed"
                );
            }
        }
        return Ok(true);
    }

    let Some(event) = db::claim(db_pool, config.claim_timeout, config.max_attempts).await? else {
        return Ok(false);
    };
    match identity.dispatch_profile(&event.payload).await {
        Ok(()) => {
            db::complete(db_pool, event.event_id).await?;
            tracing::info!(event_id = %event.event_id, "OIDC profile projection delivered");
        }
        Err(error) => {
            db::fail(
                db_pool,
                event.event_id,
                error.is_retryable(),
                error.code(),
                config.max_attempts,
            )
            .await?;
            tracing::warn!(
                event_id = %event.event_id,
                retryable = error.is_retryable(),
                code = error.code(),
                "OIDC profile projection delivery failed"
            );
        }
    }
    Ok(true)
}
