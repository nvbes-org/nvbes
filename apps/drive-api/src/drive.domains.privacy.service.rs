use serde_json::{Value, json};
use sqlx::PgPool;
use std::time::Duration;
use uuid::Uuid;

use crate::{
    domains::auth::types::AuthContext, domains::authz::WorkspaceAccess, http::error::AppError,
};
use nvbes_tenancy::RlsContext;

use super::db;
pub use super::types::*;

const JOB_PRIVACY_ACCOUNT_EXPORT: &str = "privacy.account_export";
const JOB_PRIVACY_ACCOUNT_DELETE: &str = "privacy.account_delete";
const JOB_PRIVACY_WORKSPACE_EXPORT: &str = "privacy.workspace_export";
const JOB_PRIVACY_WORKSPACE_DELETE: &str = "privacy.workspace_delete";

pub async fn get_request(
    db: &PgPool,
    storage: &dyn nvbes_storage::ObjectStore,
    auth: &AuthContext,
    request_id: Uuid,
) -> Result<PrivacyRequestStatusResponse, AppError> {
    let request =
        db::fetch_privacy_request(db, auth.user_id, auth.principal_id, request_id).await?;

    let mut request = request.ok_or_else(|| {
        AppError::not_found("privacy_request_not_found", "Privacy request not found.")
    })?;

    attach_export_download(storage, &mut request).await?;

    Ok(request)
}

pub async fn request_account_export(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    auth: &AuthContext,
) -> Result<PrivacyRequestResponse, AppError> {
    create_privacy_request(
        db,
        redis,
        PrivacyRequestDraft {
            request_type: "account_export",
            job_type: JOB_PRIVACY_ACCOUNT_EXPORT,
            subject_user_id: Some(auth.user_id),
            workspace_id: None,
            requested_by: auth.user_id,
            requested_by_principal_id: auth.principal_id,
            rls_context: RlsContext {
                principal_id: Some(auth.principal_id),
                user_id: Some(auth.user_id),
                tenant_id: auth.tenant_id,
                workspace_id: auth.workspace_id,
            },
            payload: privacy_request_payload(
                Some(auth.user_id),
                None,
                auth.user_id,
                auth.principal_id,
            ),
            message: "Account data export has been queued.",
        },
    )
    .await
}

pub async fn request_account_delete(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    auth: &AuthContext,
) -> Result<PrivacyRequestResponse, AppError> {
    ensure_account_delete_allowed(db, auth.user_id).await?;

    create_privacy_request(
        db,
        redis,
        PrivacyRequestDraft {
            request_type: "account_delete",
            job_type: JOB_PRIVACY_ACCOUNT_DELETE,
            subject_user_id: Some(auth.user_id),
            workspace_id: None,
            requested_by: auth.user_id,
            requested_by_principal_id: auth.principal_id,
            rls_context: RlsContext {
                principal_id: Some(auth.principal_id),
                user_id: Some(auth.user_id),
                tenant_id: auth.tenant_id,
                workspace_id: auth.workspace_id,
            },
            payload: privacy_request_payload(
                Some(auth.user_id),
                None,
                auth.user_id,
                auth.principal_id,
            ),
            message: "Account deletion has been queued.",
        },
    )
    .await
}

pub async fn request_workspace_export(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    access: &WorkspaceAccess,
) -> Result<PrivacyRequestResponse, AppError> {
    create_privacy_request(
        db,
        redis,
        PrivacyRequestDraft {
            request_type: "workspace_export",
            job_type: JOB_PRIVACY_WORKSPACE_EXPORT,
            subject_user_id: None,
            workspace_id: Some(access.workspace_id),
            requested_by: access.auth.user_id,
            requested_by_principal_id: access.auth.principal_id,
            rls_context: RlsContext {
                principal_id: Some(access.auth.principal_id),
                user_id: Some(access.auth.user_id),
                tenant_id: access.tenant_id.or(access.auth.tenant_id),
                workspace_id: Some(access.workspace_id),
            },
            payload: privacy_request_payload(
                None,
                Some(access.workspace_id),
                access.auth.user_id,
                access.auth.principal_id,
            ),
            message: "Workspace data export has been queued.",
        },
    )
    .await
}

pub async fn request_workspace_delete(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    access: &WorkspaceAccess,
) -> Result<PrivacyRequestResponse, AppError> {
    ensure_workspace_delete_allowed(db, access.workspace_id).await?;

    create_privacy_request(
        db,
        redis,
        PrivacyRequestDraft {
            request_type: "workspace_delete",
            job_type: JOB_PRIVACY_WORKSPACE_DELETE,
            subject_user_id: None,
            workspace_id: Some(access.workspace_id),
            requested_by: access.auth.user_id,
            requested_by_principal_id: access.auth.principal_id,
            rls_context: RlsContext {
                principal_id: Some(access.auth.principal_id),
                user_id: Some(access.auth.user_id),
                tenant_id: access.tenant_id.or(access.auth.tenant_id),
                workspace_id: Some(access.workspace_id),
            },
            payload: privacy_request_payload(
                None,
                Some(access.workspace_id),
                access.auth.user_id,
                access.auth.principal_id,
            ),
            message: "Workspace deletion has been queued.",
        },
    )
    .await
}

async fn create_privacy_request(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    draft: PrivacyRequestDraft,
) -> Result<PrivacyRequestResponse, AppError> {
    let mut tx = db.begin().await?;
    nvbes_tenancy::set_transaction_rls_context(&mut tx, draft.rls_context).await?;
    let idempotency_key = privacy_idempotency_key(&draft);

    let res = db::insert_privacy_request_tx(&mut tx, &draft).await?;
    let worker_job_id = res.worker_job_id.unwrap_or_else(uuid::Uuid::new_v4);
    db::update_privacy_request_job_tx(&mut tx, res.id, worker_job_id).await?;
    tx.commit().await?;

    let payload = merge_request_payload(draft.payload, res.id);
    db::insert_worker_job_tx(
        redis,
        draft.job_type,
        draft.job_type,
        &idempotency_key,
        payload,
        worker_job_id,
    )
    .await?;

    Ok(PrivacyRequestResponse {
        request_id: res.id,
        request_type: draft.request_type.to_string(),
        status: res.status,
        worker_job_id,
        requested_by_principal_id: res.requested_by_principal_id,
        message: draft.message.to_string(),
        requested_at: res.requested_at,
        guardrails: vec![
            "Identity is verified through the active authenticated session.".to_string(),
            "Audit logs, billing records and security logs are retained when legally required.".to_string(),
            "Deletion jobs anonymize retained references instead of breaking audit integrity.".to_string(),
            "Backup purges must respect documented retention and replay completed deletion requests.".to_string(),
        ],
    })
}

async fn ensure_account_delete_allowed(db: &PgPool, user_id: Uuid) -> Result<(), AppError> {
    let owned_workspace_count = db::count_owned_workspaces(db, user_id).await?;

    if owned_workspace_count > 0 {
        return Err(AppError::conflict(
            "account_owns_workspaces",
            "Delete or transfer owned workspaces before deleting this account.",
        ));
    }

    Ok(())
}

async fn ensure_workspace_delete_allowed(db: &PgPool, workspace_id: Uuid) -> Result<(), AppError> {
    let legal_hold = db::check_legal_hold(db, workspace_id).await?;

    if legal_hold {
        return Err(AppError::conflict(
            "workspace_legal_hold",
            "Workspace deletion is blocked by an active legal hold.",
        ));
    }

    Ok(())
}

fn privacy_idempotency_key(draft: &PrivacyRequestDraft) -> String {
    let subject = draft
        .subject_user_id
        .or(draft.workspace_id)
        .map(|id| id.to_string())
        .unwrap_or_else(|| "unknown".to_owned());

    format!("privacy:{}:{subject}", draft.request_type)
}

fn merge_request_payload(payload: serde_json::Value, request_id: Uuid) -> serde_json::Value {
    let mut payload = payload;
    if let Some(object) = payload.as_object_mut() {
        object.insert(
            "privacy_request_id".to_owned(),
            serde_json::json!(request_id),
        );
    }
    payload
}

async fn attach_export_download(
    storage: &dyn nvbes_storage::ObjectStore,
    request: &mut PrivacyRequestStatusResponse,
) -> Result<(), AppError> {
    if request.status != "completed" {
        return Ok(());
    }

    let Some(result) = request.result.as_mut() else {
        return Ok(());
    };
    let Some(delivery) = result.get_mut("delivery").and_then(Value::as_object_mut) else {
        return Ok(());
    };
    let Some(object_key) = delivery
        .get("object_key")
        .and_then(Value::as_str)
        .map(str::to_owned)
    else {
        return Ok(());
    };

    let expires_in = delivery
        .get("download_expires_in_seconds")
        .and_then(Value::as_u64)
        .unwrap_or(900)
        .clamp(60, 3600);
    let signed = storage
        .presign_download(&object_key, Duration::from_secs(expires_in))
        .await
        .map_err(|error| AppError::internal("privacy_export_presign_failed", &error.to_string()))?;

    delivery.remove("object_key");
    delivery.insert(
        "download".to_owned(),
        json!({
            "url": signed.url,
            "method": signed.method,
            "expires_in_seconds": signed.expires_in.as_secs(),
        }),
    );

    Ok(())
}

fn privacy_request_payload(
    subject_user_id: Option<Uuid>,
    workspace_id: Option<Uuid>,
    requested_by: Uuid,
    requested_by_principal_id: Uuid,
) -> serde_json::Value {
    let mut payload = serde_json::Map::new();
    if let Some(subject_user_id) = subject_user_id {
        payload.insert("subject_user_id".to_owned(), json!(subject_user_id));
    }
    if let Some(workspace_id) = workspace_id {
        payload.insert("workspace_id".to_owned(), json!(workspace_id));
    }
    payload.insert("requested_by".to_owned(), json!(requested_by));
    payload.insert(
        "requested_by_principal_id".to_owned(),
        json!(requested_by_principal_id),
    );
    serde_json::Value::Object(payload)
}

#[cfg(test)]
mod tests {
    use super::{PrivacyRequestResponse, privacy_request_payload};
    use uuid::Uuid;

    #[test]
    fn privacy_request_payload_includes_requested_by_principal_id() {
        let payload = privacy_request_payload(
            Some(Uuid::nil()),
            Some(Uuid::new_v4()),
            Uuid::new_v4(),
            Uuid::new_v4(),
        );

        let object = payload.as_object().expect("object payload");
        assert!(object.contains_key("requested_by_principal_id"));
        assert!(object.contains_key("requested_by"));
    }

    #[test]
    fn privacy_request_response_includes_requested_by_principal_id() {
        let response = PrivacyRequestResponse {
            request_id: Uuid::new_v4(),
            request_type: "workspace_export".to_string(),
            status: "queued".to_string(),
            worker_job_id: Uuid::new_v4(),
            requested_by_principal_id: Uuid::new_v4(),
            message: "Queued".to_string(),
            guardrails: vec!["guardrail".to_string()],
            requested_at: chrono::Utc::now(),
        };

        let json = serde_json::to_value(response).expect("serializable response");
        assert!(json.get("requested_by_principal_id").is_some());
    }
}
