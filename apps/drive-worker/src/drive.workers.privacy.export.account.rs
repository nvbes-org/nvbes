use serde_json::Value as JsonValue;
use uuid::Uuid;

use crate::db::Database;

pub async fn build(database: &Database, subject_user_id: Uuid) -> anyhow::Result<JsonValue> {
    let export = sqlx::query_scalar::<_, JsonValue>(
        r#"
        SELECT jsonb_build_object(
          'exported_at', NOW(),
          'scope', 'drive.account',
          'subject_user_id', $1,
          'user', (
            SELECT to_jsonb(users)
            FROM users
            WHERE id = $1
          ),
          'sessions', COALESCE((
            SELECT jsonb_agg(to_jsonb(sessions) ORDER BY created_at DESC)
            FROM sessions
            WHERE user_id = $1
          ), '[]'::jsonb),
          'mfa_factors', COALESCE((
            SELECT jsonb_agg(to_jsonb(mfa_factors) ORDER BY created_at DESC)
            FROM mfa_factors
            WHERE user_id = $1
          ), '[]'::jsonb),
          'workspace_memberships', COALESCE((
            SELECT jsonb_agg(
              to_jsonb(wm)
              || jsonb_build_object('workspace_name', w.name, 'workspace_type', w.workspace_type::text)
              ORDER BY wm.created_at DESC
            )
            FROM workspace_memberships wm
            INNER JOIN workspaces w ON w.id = wm.workspace_id
            WHERE wm.user_id = $1
          ), '[]'::jsonb),
          'owned_workspaces', COALESCE((
            SELECT jsonb_agg(to_jsonb(workspaces) ORDER BY created_at DESC)
            FROM workspaces
            WHERE owner_user_id = $1 OR owner_principal_id = $1
          ), '[]'::jsonb),
          'created_storage_objects', COALESCE((
            SELECT jsonb_agg((to_jsonb(storage_objects) - 'object_key') ORDER BY created_at DESC)
            FROM storage_objects
            WHERE created_by = $1
          ), '[]'::jsonb),
          'created_upload_sessions', COALESCE((
            SELECT jsonb_agg((to_jsonb(upload_sessions) - 'storage_multipart_upload_id') ORDER BY created_at DESC)
            FROM upload_sessions
            WHERE created_by = $1
          ), '[]'::jsonb),
          'created_share_links', COALESCE((
            SELECT jsonb_agg((to_jsonb(share_links) - 'token_hash') ORDER BY created_at DESC)
            FROM share_links
            WHERE created_by = $1
          ), '[]'::jsonb),
          'created_api_keys', COALESCE((
            SELECT jsonb_agg((to_jsonb(api_keys) - 'key_hash') ORDER BY created_at DESC)
            FROM api_keys
            WHERE created_by = $1
          ), '[]'::jsonb),
          'api_request_logs_for_created_keys', COALESCE((
            SELECT jsonb_agg(to_jsonb(api_request_logs) ORDER BY created_at DESC)
            FROM api_request_logs
            WHERE api_key_id IN (
              SELECT id FROM api_keys WHERE created_by = $1
            )
          ), '[]'::jsonb),
          'audit_events', COALESCE((
            SELECT jsonb_agg(to_jsonb(audit_events) ORDER BY created_at DESC)
            FROM audit_events
            WHERE actor_user_id = $1 OR actor_principal_id = $1
          ), '[]'::jsonb),
          'privacy_requests', COALESCE((
            SELECT jsonb_agg((to_jsonb(privacy_requests) - 'result') ORDER BY requested_at DESC)
            FROM privacy_requests
            WHERE subject_user_id = $1 OR requested_by = $1 OR requested_by_principal_id = $1
          ), '[]'::jsonb)
        )
        "#,
    )
    .bind(subject_user_id)
    .fetch_one(&**database)
    .await?;

    Ok(export)
}
