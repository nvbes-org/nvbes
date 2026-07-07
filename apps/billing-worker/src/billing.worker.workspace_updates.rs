use anyhow::Context;
use tracing::warn;
use uuid::Uuid;

use crate::worker::BillingWorkerState;

pub(crate) async fn publish_workspace_billing_updates(
    state: &BillingWorkerState,
    workspace_id: Uuid,
) -> anyhow::Result<Option<String>> {
    if let Err(error) =
        nvbes_redis::pubsub::publish_workspace_updated(&state.redis, &workspace_id.to_string())
            .await
    {
        warn!(
            workspace_id = %workspace_id,
            error = %error,
            "Failed to publish billing workspace update"
        );
    }

    let plan_code = fetch_workspace_plan_code(&state.db, workspace_id)
        .await
        .context("failed to fetch workspace plan code for billing update")?;

    if let Some(code) = plan_code.as_deref() {
        let (workspace_key, plan_payload) = workspace_plan_update_payload(workspace_id, code);
        if let Err(error) = nvbes_redis::pubsub::publish_workspace_plan_updated(
            &state.redis,
            &workspace_key,
            &plan_payload,
        )
        .await
        {
            warn!(
                workspace_id = %workspace_id,
                plan_code = %code,
                error = %error,
                "Failed to publish billing workspace plan update"
            );
        }
    }

    Ok(plan_code)
}

fn workspace_plan_update_payload(workspace_id: Uuid, plan_code: &str) -> (String, String) {
    (workspace_id.to_string(), plan_code.to_string())
}

async fn fetch_workspace_plan_code(
    db_pool: &sqlx::PgPool,
    workspace_id: Uuid,
) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT p.code FROM workspaces w JOIN plans p ON p.id = w.plan_id WHERE w.id = $1",
    )
    .bind(workspace_id)
    .fetch_optional(db_pool)
    .await
}

#[cfg(test)]
mod tests {
    use super::{fetch_workspace_plan_code, workspace_plan_update_payload};

    #[tokio::test]
    async fn plan_code_lookup_requires_database_when_called() {
        let pool = sqlx::PgPool::connect_lazy("postgres://localhost/unused").expect("pool");
        let result = fetch_workspace_plan_code(&pool, uuid::Uuid::nil()).await;

        assert!(result.is_err());
    }

    #[test]
    fn workspace_plan_update_payload_preserves_plan_code() {
        let workspace_id = uuid::Uuid::new_v4();

        let (workspace_key, plan_code) = workspace_plan_update_payload(workspace_id, "team-pro");

        assert_eq!(workspace_key, workspace_id.to_string());
        assert_eq!(plan_code, "team-pro");
    }
}
