use sqlx::PgPool;
use tracing::{error, info};

pub async fn run_outbox_tick(pool: &PgPool) {
    // 1. Dispatch pending Account outbox events
    match nvbes_account_service::outbox::publish_pending(pool, 50).await {
        Ok(published) => {
            if published > 0 {
                info!(published, "published pending account outbox events");
            }
        }
        Err(err) => {
            error!("error publishing account outbox events: {err}");
        }
    }

    // 2. Dispatch pending Billing outbox events
    match nvbes_billing_service::outbox::publish_pending_outbox_events(
        pool,
        None,
        "https://nvbes.test",
        50,
    )
    .await
    {
        Ok(published) => {
            if published > 0 {
                info!(published, "published pending billing outbox events");
            }
        }
        Err(err) => {
            error!("error publishing billing outbox events: {err}");
        }
    }
}
