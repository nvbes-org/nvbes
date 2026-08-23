use chrono::{DateTime, NaiveDate, Utc};
use uuid::Uuid;

use crate::{billing_grpc::parse_uuid, error::AppError};

pub(crate) struct BackofficeBillingOperationsSnapshot {
    pub(crate) provider_event_failure_count: i64,
    pub(crate) provider_event_backlog_count: i64,
    pub(crate) export_pending_count: i64,
    pub(crate) export_failed_count: i64,
    pub(crate) reconciliation_pending_count: i64,
    pub(crate) reconciliation_failed_count: i64,
    pub(crate) unresolved_reconciliation_difference_count: i64,
    pub(crate) recent_provider_failures: Vec<BackofficeRecentProviderFailure>,
    pub(crate) recent_export_runs: Vec<BackofficeRecentExportRun>,
    pub(crate) recent_reconciliation_differences: Vec<BackofficeRecentReconciliationDifference>,
}

pub(crate) struct BackofficeRecentProviderFailure {
    pub(crate) id: Uuid,
    pub(crate) tenant_id: Option<Uuid>,
    pub(crate) tenant_name: Option<String>,
    pub(crate) provider: String,
    pub(crate) provider_event_id: String,
    pub(crate) event_type: String,
    pub(crate) status: String,
    pub(crate) received_at: DateTime<Utc>,
}

pub(crate) struct BackofficeRecentExportRun {
    pub(crate) id: Uuid,
    pub(crate) export_type: String,
    pub(crate) status: String,
    pub(crate) period_start: Option<NaiveDate>,
    pub(crate) period_end: Option<NaiveDate>,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
}

pub(crate) struct BackofficeRecentReconciliationDifference {
    pub(crate) id: Uuid,
    pub(crate) tenant_id: Option<Uuid>,
    pub(crate) tenant_name: Option<String>,
    pub(crate) difference_type: String,
    pub(crate) severity: String,
    pub(crate) created_at: DateTime<Utc>,
}

pub(crate) fn operations_snapshot(
    value: crate::grpc_pb::nvbes::billing::v1::AdminOperationsCenterSnapshot,
) -> Result<BackofficeBillingOperationsSnapshot, AppError> {
    Ok(BackofficeBillingOperationsSnapshot {
        provider_event_failure_count: value.provider_event_failure_count,
        provider_event_backlog_count: value.provider_event_backlog_count,
        export_pending_count: value.export_pending_count,
        export_failed_count: value.export_failed_count,
        reconciliation_pending_count: value.reconciliation_pending_count,
        reconciliation_failed_count: value.reconciliation_failed_count,
        unresolved_reconciliation_difference_count: value
            .unresolved_reconciliation_difference_count,
        recent_provider_failures: value
            .recent_provider_failures
            .into_iter()
            .map(|item| {
                Ok(BackofficeRecentProviderFailure {
                    id: parse_uuid(&item.id)?,
                    tenant_id: parse_optional_uuid(&item.tenant_id)?,
                    tenant_name: empty_to_none(item.tenant_name),
                    provider: item.provider,
                    provider_event_id: item.provider_event_id,
                    event_type: item.event_type,
                    status: item.status,
                    received_at: parse_datetime(&item.received_at)?,
                })
            })
            .collect::<Result<Vec<_>, AppError>>()?,
        recent_export_runs: value
            .recent_export_runs
            .into_iter()
            .map(|item| {
                Ok(BackofficeRecentExportRun {
                    id: parse_uuid(&item.id)?,
                    export_type: item.export_type,
                    status: item.status,
                    period_start: parse_optional_date(&item.period_start)?,
                    period_end: parse_optional_date(&item.period_end)?,
                    created_at: parse_datetime(&item.created_at)?,
                    updated_at: parse_datetime(&item.updated_at)?,
                })
            })
            .collect::<Result<Vec<_>, AppError>>()?,
        recent_reconciliation_differences: value
            .recent_reconciliation_differences
            .into_iter()
            .map(|item| {
                Ok(BackofficeRecentReconciliationDifference {
                    id: parse_uuid(&item.id)?,
                    tenant_id: parse_optional_uuid(&item.tenant_id)?,
                    tenant_name: empty_to_none(item.tenant_name),
                    difference_type: item.difference_type,
                    severity: item.severity,
                    created_at: parse_datetime(&item.created_at)?,
                })
            })
            .collect::<Result<Vec<_>, AppError>>()?,
    })
}

fn empty_to_none(value: String) -> Option<String> {
    let value = value.trim().to_string();
    if value.is_empty() { None } else { Some(value) }
}

fn parse_optional_uuid(value: &str) -> Result<Option<Uuid>, AppError> {
    empty_to_none(value.to_string()).map_or(Ok(None), |value| parse_uuid(&value).map(Some))
}

fn parse_datetime(value: &str) -> Result<DateTime<Utc>, AppError> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|error| AppError::internal("billing_grpc_invalid_datetime", error.to_string()))
}

fn parse_optional_date(value: &str) -> Result<Option<NaiveDate>, AppError> {
    empty_to_none(value.to_string()).map_or(Ok(None), |value| {
        NaiveDate::parse_from_str(&value, "%Y-%m-%d")
            .map(Some)
            .map_err(|error| AppError::internal("billing_grpc_invalid_date", error.to_string()))
    })
}
