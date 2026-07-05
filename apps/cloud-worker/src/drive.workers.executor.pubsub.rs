use crate::db::Database;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

pub(super) async fn run_pubsub_listener(
    database: Database,
    redis: nvbes_redis::RedisPool,
) -> anyhow::Result<()> {
    use futures_util::StreamExt;

    let url =
        std::env::var("NVBES_REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());
    let client = redis::Client::open(url)?;
    let mut pubsub = client.get_async_pubsub().await?;

    pubsub.subscribe("nvbes:pubsub:workspace:deleted").await?;
    pubsub
        .subscribe("nvbes:pubsub:workspace:plan_updated")
        .await?;
    pubsub.subscribe("nvbes:pubsub:user:suspended").await?;
    pubsub.subscribe("nvbes:pubsub:session:revoked").await?;

    tracing::info!("Subscribed to real-time events on Redis PubSub");

    let mut stream = pubsub.on_message();
    while let Some(msg) = stream.next().await {
        let channel = msg.get_channel_name();
        let payload: String = match msg.get_payload() {
            Ok(value) => value,
            Err(error) => {
                tracing::error!("Failed to get payload from PubSub message: {:?}", error);
                continue;
            }
        };
        tracing::info!(
            channel = %channel,
            payload_size_bytes = payload.len(),
            "Received real-time event via PubSub"
        );

        match channel {
            "nvbes:pubsub:workspace:deleted" => handle_workspace_deleted(&redis, &payload).await,
            "nvbes:pubsub:workspace:plan_updated" => {
                handle_workspace_plan_updated(&database, &payload).await
            }
            "nvbes:pubsub:user:suspended" => handle_user_suspended(&database, &payload).await,
            "nvbes:pubsub:session:revoked" => handle_session_revoked(&database, &payload).await,
            _ => {}
        }
    }

    Ok(())
}

async fn handle_workspace_deleted(redis: &nvbes_redis::RedisPool, payload: &str) {
    if let Ok(workspace_id) = uuid::Uuid::parse_str(payload) {
        tracing::info!(workspace_id = %workspace_id, "Handling workspace deleted event");
        if let Err(error) = nvbes_redis::worker_queue::enqueue_job(
            redis,
            nvbes_redis::worker_queue::EnqueueJobInput {
                queue: super::super::privacy::delete::JOB_PRIVACY_WORKSPACE_DELETE.to_string(),
                job_type: super::super::privacy::delete::JOB_PRIVACY_WORKSPACE_DELETE.to_string(),
                payload: serde_json::json!({ "workspace_id": workspace_id }),
                idempotency_key: Some(format!("pubsub:workspace_delete:{}", workspace_id)),
                max_attempts: 3,
                overwrite_terminal: true,
                job_id: None,
            },
        )
        .await
        {
            tracing::error!("Failed to enqueue workspace deletion job: {:?}", error);
        }
    }
}

async fn handle_workspace_plan_updated(database: &Database, payload: &str) {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(payload)
        && let (Some(workspace_id_str), Some(plan_code)) = (
            value.get("workspace_id").and_then(|item| item.as_str()),
            value.get("plan_code").and_then(|item| item.as_str()),
        )
        && let Ok(workspace_id) = uuid::Uuid::parse_str(workspace_id_str)
    {
        tracing::info!(workspace_id = %workspace_id, plan_code = %plan_code, "Handling workspace plan updated event");
        let database_clone = database.clone();
        let plan_code_owned = plan_code.to_owned();
        tokio::spawn(async move {
            match database_clone.begin().await {
                Ok(mut tx) => match plan_id_by_code_tx(&mut tx, &plan_code_owned).await {
                    Ok(Some(plan_id)) => {
                        if let Err(error) =
                            project_workspace_plan_tx(&mut tx, workspace_id, plan_id).await
                        {
                            tracing::error!("Failed to project workspace plan: {:?}", error);
                        } else if let Err(error) = tx.commit().await {
                            tracing::error!("Failed to commit transaction: {:?}", error);
                        } else {
                            tracing::info!(workspace_id = %workspace_id, plan_code = %plan_code_owned, "Workspace plan successfully projected locally");
                        }
                    }
                    Ok(None) => {
                        tracing::error!(plan_code = %plan_code_owned, "Plan code is not known locally");
                    }
                    Err(error) => {
                        tracing::error!("Failed to fetch plan id by code: {:?}", error);
                    }
                },
                Err(error) => {
                    tracing::error!("Failed to start transaction: {:?}", error);
                }
            }
        });
    }
}

async fn handle_user_suspended(database: &Database, payload: &str) {
    if let Ok(user_id) = uuid::Uuid::parse_str(payload) {
        tracing::info!(user_id = %user_id, "Handling user suspended event");
        if let Err(error) =
            sqlx::query("UPDATE users SET status = 'suspended', updated_at = NOW() WHERE id = $1")
                .bind(user_id)
                .execute(&**database)
                .await
        {
            tracing::error!("Failed to suspend user in DB: {:?}", error);
        }
    }
}

async fn plan_id_by_code_tx(
    tx: &mut Transaction<'_, Postgres>,
    code: &str,
) -> sqlx::Result<Option<Uuid>> {
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM plans WHERE code = $1")
        .bind(code)
        .fetch_optional(&mut **tx)
        .await
}

async fn project_workspace_plan_tx(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: Uuid,
    plan_id: Uuid,
) -> sqlx::Result<()> {
    sqlx::query(
        r#"
        UPDATE workspaces
        SET plan_id = $2,
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(workspace_id)
    .bind(plan_id)
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        r#"
        UPDATE workspace_policies wp
        SET max_share_link_ttl_days = p.max_share_link_ttl_days,
            default_share_link_ttl_days = LEAST(wp.default_share_link_ttl_days, p.max_share_link_ttl_days),
            updated_at = NOW()
        FROM plans p
        WHERE wp.workspace_id = $1
          AND p.id = $2
        "#,
    )
    .bind(workspace_id)
    .bind(plan_id)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

async fn handle_session_revoked(database: &Database, payload: &str) {
    let parts: Vec<&str> = payload.split(':').collect();
    if parts.len() == 2
        && let (Ok(user_id), Ok(session_id)) = (
            uuid::Uuid::parse_str(parts[0]),
            uuid::Uuid::parse_str(parts[1]),
        )
    {
        tracing::info!(user_id = %user_id, session_id = %session_id, "Handling session revoked event");
        if let Err(error) = sqlx::query(
            r#"
                INSERT INTO sessions (id, user_id, expires_at, revoked_at)
                VALUES ($1, $2, NOW(), NOW())
                ON CONFLICT (id) DO UPDATE SET revoked_at = NOW()
                "#,
        )
        .bind(session_id)
        .bind(user_id)
        .execute(&**database)
        .await
        {
            tracing::error!("Failed to revoke session in SQL DB: {:?}", error);
        }
    }
}
