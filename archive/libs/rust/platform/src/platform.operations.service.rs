use crate::{
    cockpit_auth::{OperatorSession, Permission},
    operations_db::{lock_case, save_case},
    operations_error::OperationsError,
    operations_model::{Action, Case, CaseCategory, CaseStatus, Command, Cost, Receipt},
    operations_validation::{transition, validate},
};
use chrono::Utc;
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

pub async fn execute(
    db: &PgPool,
    actor: &OperatorSession,
    command: Command,
) -> Result<Receipt, OperationsError> {
    let permission = if matches!(command.action, Action::RecordCost { .. }) {
        Permission::WriteCosts
    } else {
        Permission::WriteCases
    };
    if !actor.permits(permission) {
        return Err(OperationsError::Forbidden);
    }
    validate(&command)?;
    let request = serde_json::to_value(&command)?;
    let mut tx = db.begin().await?;
    // Serialize commands from this authenticated actor across replicas and restarts.
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(&actor.operator_id)
        .execute(&mut *tx)
        .await?;
    let previous: Option<(Value, Value)> = sqlx::query_as(
        "SELECT request, receipt FROM operations_audit WHERE actor=$1 AND idempotency_key=$2",
    )
    .bind(&actor.operator_id)
    .bind(command.idempotency_key)
    .fetch_optional(&mut *tx)
    .await?;
    if let Some((original, receipt)) = previous {
        if original != request {
            return Err(OperationsError::Conflict);
        }
        return Ok(serde_json::from_value(receipt)?);
    }
    let now = Utc::now();
    let mut receipt = Receipt {
        id: Uuid::new_v4(),
        actor: actor.operator_id.clone(),
        correlation_id: command.correlation_id,
        case_id: command.action.case_id(),
        version: None,
        cost_id: None,
        executed_at: now,
        status: "executed".into(),
    };
    match command.action {
        Action::OpenCase {
            category,
            owner,
            subject_id,
            source,
            summary,
            related_case_id,
        } => {
            if category == CaseCategory::Appeal && related_case_id.is_none() {
                return Err(OperationsError::Invalid("appeal requires an original case"));
            }
            if let Some(id) = related_case_id {
                let original: Option<Value> =
                    sqlx::query_scalar("SELECT document FROM operations_cases WHERE id=$1")
                        .bind(id)
                        .fetch_optional(&mut *tx)
                        .await?;
                let original: Case =
                    serde_json::from_value(original.ok_or(OperationsError::NotFound)?)?;
                if original.subject_id != subject_id {
                    return Err(OperationsError::Invalid(
                        "related case belongs to another subject",
                    ));
                }
            }
            let case = Case {
                id: Uuid::new_v4(),
                version: 1,
                category,
                owner,
                subject_id,
                source,
                summary,
                status: CaseStatus::Open,
                related_case_id,
                created_at: now,
                updated_at: now,
            };
            save_case(&mut tx, &case).await?;
            receipt.case_id = Some(case.id);
            receipt.version = Some(1);
        }
        Action::Transition {
            case_id,
            expected_version,
            status,
            ..
        } => {
            let mut case = lock_case(&mut tx, case_id, expected_version).await?;
            transition(case.status, status)?;
            case.status = status;
            case.version += 1;
            case.updated_at = now;
            save_case(&mut tx, &case).await?;
            receipt.version = Some(case.version);
        }
        Action::AddNote {
            case_id,
            expected_version,
            ..
        }
        | Action::RecordObservation {
            case_id,
            expected_version,
            ..
        } => {
            let mut case = lock_case(&mut tx, case_id, expected_version).await?;
            if case.status == CaseStatus::Closed {
                return Err(OperationsError::Invalid(
                    "reopen an appeal before adding notes",
                ));
            }
            case.version += 1;
            case.updated_at = now;
            save_case(&mut tx, &case).await?;
            receipt.version = Some(case.version);
        }
        Action::RecordCost {
            month,
            provider,
            category,
            actual_cents,
            forecast_cents,
            evidence,
            replaces,
        } => {
            let cost = Cost {
                id: Uuid::new_v4(),
                month,
                provider,
                category,
                actual_cents,
                forecast_cents,
                evidence,
                replaces,
            };
            if let Some(id) = replaces {
                let original: Option<Value> = sqlx::query_scalar(
                    "SELECT document FROM operations_costs WHERE id=$1 FOR UPDATE",
                )
                .bind(id)
                .fetch_optional(&mut *tx)
                .await?;
                let original: Cost =
                    serde_json::from_value(original.ok_or(OperationsError::NotFound)?)?;
                let replaced: bool = sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM operations_costs WHERE document->>'replaces'=$1)",
                )
                .bind(id.to_string())
                .fetch_one(&mut *tx)
                .await?;
                if replaced {
                    return Err(OperationsError::Conflict);
                }
                if original.month != month
                    || original.provider != cost.provider
                    || original.category != cost.category
                {
                    return Err(OperationsError::Invalid(
                        "correction must preserve month, provider and category",
                    ));
                }
            }
            sqlx::query("INSERT INTO operations_costs(id,month,document) VALUES ($1,$2,$3)")
                .bind(cost.id)
                .bind(month)
                .bind(serde_json::to_value(&cost)?)
                .execute(&mut *tx)
                .await?;
            receipt.cost_id = Some(cost.id);
        }
    }
    sqlx::query("INSERT INTO operations_audit(actor,idempotency_key,case_id,request,receipt) VALUES ($1,$2,$3,$4,$5)")
        .bind(&actor.operator_id).bind(command.idempotency_key).bind(receipt.case_id)
        .bind(request).bind(serde_json::to_value(&receipt)?).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(receipt)
}
