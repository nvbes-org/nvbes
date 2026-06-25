use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::app::AppState;
use crate::backoffice_authorization::{
    BackofficePermission, require_idempotency_key, require_operator_role_grant, require_permission,
    require_strong_confirmation_for_value,
};
use crate::backoffice_dual_control::require_dual_control;
use crate::billing_admin_access::{actor_principal_id, authorize_backoffice};
use crate::error::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BillingRunbook {
    PspOutage,
    WebhookLag,
    DuplicatePayment,
    TaxConfigError,
    LedgerImbalance,
    FailedExport,
}

#[derive(Debug, Serialize)]
pub struct BillingRunbookView {
    pub id: &'static str,
    pub title: &'static str,
    pub severity: &'static str,
    pub steps: &'static [&'static str],
}

#[derive(Debug, Deserialize)]
struct RunbookExecutionRequest {
    confirm_code: String,
    reason: String,
}

#[derive(Debug, Serialize)]
struct RunbookExecutionResult {
    workspace_id: Uuid,
    tenant_id: Uuid,
    runbook_id: String,
    audit_action: &'static str,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/admin/billing/runbooks", get(list_billing_runbooks_route))
        .route(
            "/workspaces/{workspaceId}/billing/admin/runbooks/{runbookId}/execute",
            post(execute_runbook_route),
        )
}

async fn list_billing_runbooks_route() -> Json<Vec<BillingRunbookView>> {
    Json(list_billing_runbooks())
}

async fn execute_runbook_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((workspace_id, runbook_id)): Path<(Uuid, String)>,
    Json(request): Json<RunbookExecutionRequest>,
) -> Result<Json<RunbookExecutionResult>, AppError> {
    require_runbook_mutation(&headers, &request.confirm_code, &runbook_id)?;
    require_operator_role_grant(&state.db, &headers).await?;
    let access = authorize_backoffice(&state.db, &headers, workspace_id).await?;
    let _actor_id = actor_principal_id(&headers)?;
    Ok(Json(
        execute_runbook(
            &state.db,
            access.tenant_id,
            access.actor_principal_id,
            workspace_id,
            &runbook_id,
            request,
        )
        .await?,
    ))
}

fn list_billing_runbooks() -> Vec<BillingRunbookView> {
    vec![
        runbook_view(
            BillingRunbook::PspOutage,
            "PSP outage",
            "critical",
            &[
                "Basculer le routage provider si une route de secours est active.",
                "Geler les retries automatiques non idempotents.",
                "Suivre la reconciliation provider jusqu'au retour nominal.",
            ],
        ),
        runbook_view(
            BillingRunbook::WebhookLag,
            "Webhook lag",
            "high",
            &[
                "Comparer le dernier event provider recu avec le dernier event traite.",
                "Rejouer uniquement les events idempotents en retard.",
                "Verifier que les projections invoices/payments reviennent a jour.",
            ],
        ),
        runbook_view(
            BillingRunbook::DuplicatePayment,
            "Duplicate payment",
            "critical",
            &[
                "Identifier les paiements avec meme invoice, provider et montant.",
                "Bloquer toute compensation automatique avant revue humaine.",
                "Creer un refund intent ou une note de credit selon le statut ledger.",
            ],
        ),
        runbook_view(
            BillingRunbook::TaxConfigError,
            "Tax config error",
            "high",
            &[
                "Suspendre les emissions de factures impactees.",
                "Comparer le profil fiscal actif avec la juridiction workspace.",
                "Corriger la configuration puis regenerer les documents touches.",
            ],
        ),
        runbook_view(
            BillingRunbook::LedgerImbalance,
            "Ledger imbalance",
            "critical",
            &[
                "Isoler la devise et la periode comptable en ecart.",
                "Verifier les ecritures manuelles et imports provider recents.",
                "Poster une compensation uniquement apres validation finance.",
            ],
        ),
        runbook_view(
            BillingRunbook::FailedExport,
            "Failed export",
            "medium",
            &[
                "Relancer l'export apres verification des filtres.",
                "Comparer le volume exporte avec le snapshot source.",
                "Escalader si le meme export echoue deux fois consecutives.",
            ],
        ),
    ]
}

fn runbook_view(
    runbook: BillingRunbook,
    title: &'static str,
    severity: &'static str,
    steps: &'static [&'static str],
) -> BillingRunbookView {
    BillingRunbookView {
        id: runbook_slug(runbook),
        title,
        severity,
        steps,
    }
}

fn runbook_slug(runbook: BillingRunbook) -> &'static str {
    match runbook {
        BillingRunbook::PspOutage => "psp-outage",
        BillingRunbook::WebhookLag => "webhook-lag",
        BillingRunbook::DuplicatePayment => "duplicate-payment",
        BillingRunbook::TaxConfigError => "tax-config-error",
        BillingRunbook::LedgerImbalance => "ledger-imbalance",
        BillingRunbook::FailedExport => "failed-export",
    }
}

async fn execute_runbook(
    db: &PgPool,
    tenant_id: Uuid,
    actor_id: Uuid,
    workspace_id: Uuid,
    runbook_id: &str,
    request: RunbookExecutionRequest,
) -> Result<RunbookExecutionResult, AppError> {
    validate_runbook_id(runbook_id)?;
    validate_runbook_reason(&request.reason)?;

    sqlx::query(
        "INSERT INTO audit_events (
           tenant_id, workspace_id, actor_principal_id, action, target_type, target_id, metadata, event_hash
         ) VALUES (
           $1, $2, $3, 'internal_admin.runbook.executed', 'runbook', $2,
           jsonb_build_object(
             'runbook_id', $4,
             'reason', $5,
             'object_links', jsonb_build_object(
               'runbook_id', $4,
               'workspace_id', $2::text
             ),
             'target_links', jsonb_build_object(
               'workspace_id', $2::text,
               'target_type', 'runbook'
             ),
             'changes', jsonb_build_array(jsonb_build_object(
               'field', 'runbook.execution',
               'before', null,
               'after', 'recorded'
             ))
           ),
           gen_random_uuid()::text
         )",
    )
    .bind(tenant_id)
    .bind(workspace_id)
    .bind(actor_id)
    .bind(runbook_id)
    .bind(request.reason.trim())
    .execute(db)
    .await?;

    Ok(RunbookExecutionResult {
        workspace_id,
        tenant_id,
        runbook_id: runbook_id.to_string(),
        audit_action: "internal_admin.runbook.executed",
    })
}

fn validate_runbook_id(runbook_id: &str) -> Result<(), AppError> {
    let exists = list_billing_runbooks()
        .iter()
        .any(|runbook| runbook.id == runbook_id);
    if !exists {
        return Err(AppError::not_found(
            "runbook_not_found",
            "Billing runbook not found.",
        ));
    }
    Ok(())
}

fn validate_runbook_reason(reason: &str) -> Result<(), AppError> {
    if reason.trim().len() < 12 {
        return Err(AppError::bad_request(
            "audit_reason_required",
            "Runbook executions require a detailed audit reason.",
        ));
    }
    Ok(())
}

fn require_runbook_mutation(
    headers: &HeaderMap,
    confirm_code: &str,
    runbook_id: &str,
) -> Result<(), AppError> {
    require_idempotency_key(headers)?;
    require_permission(headers, BackofficePermission::BillingMutate)?;
    require_strong_confirmation_for_value(confirm_code, "EXECUTE RUNBOOK", runbook_id)?;
    require_dual_control(headers)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderMap;
    use uuid::Uuid;

    #[test]
    fn runbook_slugs_cover_critical_incidents() {
        assert_eq!(runbook_slug(BillingRunbook::PspOutage), "psp-outage");
        assert_eq!(
            runbook_slug(BillingRunbook::LedgerImbalance),
            "ledger-imbalance"
        );
    }

    #[test]
    fn runbook_execution_requires_known_runbook_and_reason() {
        assert!(validate_runbook_id("psp-outage").is_ok());
        assert!(validate_runbook_id("unknown").is_err());
        assert!(validate_runbook_reason("short").is_err());
        assert!(validate_runbook_reason("incident OPS-123 approved").is_ok());
    }

    #[test]
    fn runbook_mutation_requires_strong_confirmation_and_dual_control() {
        let actor_id = Uuid::new_v4();
        let approver_id = Uuid::new_v4();
        let headers_without_approver = mutation_headers(actor_id, None);
        assert!(
            require_runbook_mutation(
                &headers_without_approver,
                "EXECUTE RUNBOOK PSPOUTAG",
                "psp-outage"
            )
            .is_err()
        );

        let headers = mutation_headers(actor_id, Some(approver_id));
        assert!(require_runbook_mutation(&headers, "EXECUTE RUNBOOK", "psp-outage").is_err());
        assert!(
            require_runbook_mutation(&headers, "EXECUTE RUNBOOK PSPOUTAG", "psp-outage").is_ok()
        );
    }

    fn mutation_headers(actor_id: Uuid, approver_id: Option<Uuid>) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert("idempotency-key", "test-runbook-mutation".parse().unwrap());
        headers.insert("x-nvbes-backoffice-role", "finance_admin".parse().unwrap());
        headers.insert(
            "x-nvbes-actor-principal-id",
            actor_id.to_string().parse().unwrap(),
        );
        if let Some(approver_id) = approver_id {
            headers.insert(
                "x-nvbes-second-approver-principal-id",
                approver_id.to_string().parse().unwrap(),
            );
            headers.insert(
                "x-nvbes-second-approver-role",
                "platform_admin".parse().unwrap(),
            );
        }
        headers
    }
}
