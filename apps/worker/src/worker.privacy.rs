use sqlx::PgPool;
use tracing::{error, info};

pub async fn run_privacy_tick(pool: &PgPool) {
    match nvbes_account_service::privacy_jobs::process_pending(pool).await {
        Ok(result) => {
            if result.exports_completed > 0 || result.closures_completed > 0 {
                info!(
                    exports_completed = result.exports_completed,
                    closures_completed = result.closures_completed,
                    "processed pending privacy retention jobs"
                );
            }
        }
        Err(err) => {
            error!("error processing privacy retention jobs: {err}");
        }
    }
}
