use uuid::Uuid;

use crate::billing_admin_types::BackofficeAccess;
use crate::error::AppError;

pub(crate) struct FinanceExport {
    pub(crate) export_run_id: Uuid,
    pub(crate) filename: String,
    pub(crate) content_type: String,
    pub(crate) body: Vec<u8>,
    pub(crate) row_count: u64,
}

pub(crate) async fn build_finance_export(
    billing_grpc_endpoint: &str,
    access: BackofficeAccess,
    workspace_id: Uuid,
    export_type: String,
) -> Result<FinanceExport, AppError> {
    let export = crate::billing_grpc::build_admin_finance_export(
        billing_grpc_endpoint,
        access,
        workspace_id,
        export_type,
    )
    .await?;
    Ok(FinanceExport {
        export_run_id: parse_uuid(&export.export_run_id)?,
        filename: export.filename,
        content_type: export.content_type,
        body: export.body,
        row_count: export.row_count,
    })
}

fn parse_uuid(value: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(value).map_err(|_| AppError::internal("billing_grpc_decode", "export run id"))
}
