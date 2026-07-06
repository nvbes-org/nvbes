use uuid::Uuid;

use crate::billing_admin_types::{BackofficeAccess, SearchResult};
use crate::error::AppError;

pub(crate) async fn search_billing_admin(
    billing_grpc_endpoint: &str,
    access: BackofficeAccess,
    workspace_id: Uuid,
    query: String,
) -> Result<Vec<SearchResult>, AppError> {
    let results = crate::billing_grpc::search_admin_billing(
        billing_grpc_endpoint,
        access,
        workspace_id,
        query,
        25,
    )
    .await?;
    results
        .results
        .into_iter()
        .map(|row| {
            Ok(SearchResult {
                kind: row.kind,
                id: parse_uuid(&row.id)?,
                label: row.label,
                status: row.status,
            })
        })
        .collect()
}

fn parse_uuid(value: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(value)
        .map_err(|_| AppError::internal("billing_grpc_decode", "search result id"))
}
