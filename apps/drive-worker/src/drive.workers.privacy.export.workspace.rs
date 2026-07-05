use serde_json::Value as JsonValue;
use uuid::Uuid;

use crate::db::Database;

pub async fn build(database: &Database, workspace_id: Uuid) -> anyhow::Result<JsonValue> {
    let export = sqlx::query_scalar::<_, JsonValue>(
        r#"
        SELECT jsonb_build_object(
          'exported_at', NOW(),
          'scope', 'drive.workspace',
          'workspace_id', $1,
          'workspace', (
            SELECT to_jsonb(w) || jsonb_build_object('plan', to_jsonb(p))
            FROM workspaces w
            INNER JOIN plans p ON p.id = w.plan_id
            WHERE w.id = $1
          ),
          'policy', (
            SELECT to_jsonb(workspace_policies)
            FROM workspace_policies
            WHERE workspace_id = $1
          ),
          'members', COALESCE((
            SELECT jsonb_agg(
              to_jsonb(wm)
              || jsonb_build_object('user_email', u.email, 'user_name', u.name)
              ORDER BY wm.created_at DESC
            )
            FROM workspace_memberships wm
            INNER JOIN users u ON u.id = wm.user_id
            WHERE wm.workspace_id = $1
          ), '[]'::jsonb),
          'storage_objects', COALESCE((
            SELECT jsonb_agg((to_jsonb(storage_objects) - 'object_key') ORDER BY created_at DESC)
            FROM storage_objects
            WHERE workspace_id = $1
          ), '[]'::jsonb),
          'upload_sessions', COALESCE((
            SELECT jsonb_agg((to_jsonb(upload_sessions) - 'storage_multipart_upload_id') ORDER BY created_at DESC)
            FROM upload_sessions
            WHERE workspace_id = $1
          ), '[]'::jsonb),
          'upload_parts', COALESCE((
            SELECT jsonb_agg(to_jsonb(upload_parts) ORDER BY created_at DESC)
            FROM upload_parts
            WHERE workspace_id = $1
          ), '[]'::jsonb),
          'share_links', COALESCE((
            SELECT jsonb_agg((to_jsonb(share_links) - 'token_hash') ORDER BY created_at DESC)
            FROM share_links
            WHERE workspace_id = $1
          ), '[]'::jsonb),
          'api_keys', COALESCE((
            SELECT jsonb_agg((to_jsonb(api_keys) - 'key_hash') ORDER BY created_at DESC)
            FROM api_keys
            WHERE workspace_id = $1
          ), '[]'::jsonb),
          'api_request_logs', COALESCE((
            SELECT jsonb_agg(to_jsonb(api_request_logs) ORDER BY created_at DESC)
            FROM api_request_logs
            WHERE workspace_id = $1
          ), '[]'::jsonb),
          'quota_usage', (
            SELECT to_jsonb(quota_usage)
            FROM quota_usage
            WHERE workspace_id = $1
          ),
          'audit_events', COALESCE((
            SELECT jsonb_agg(to_jsonb(audit_events) ORDER BY created_at DESC)
            FROM audit_events
            WHERE workspace_id = $1
          ), '[]'::jsonb),
          'privacy_requests', COALESCE((
            SELECT jsonb_agg((to_jsonb(privacy_requests) - 'result') ORDER BY requested_at DESC)
            FROM privacy_requests
            WHERE workspace_id = $1
          ), '[]'::jsonb)
        )
        "#,
    )
    .bind(workspace_id)
    .fetch_one(&**database)
    .await?;

    Ok(export)
}
