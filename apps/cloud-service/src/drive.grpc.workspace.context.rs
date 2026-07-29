use sqlx::{Postgres, Transaction};
use tonic::Status;
use uuid::Uuid;

use crate::grpc::{
    pb::nvbes::platform::v1::RequestContext,
    service_status::{optional_uuid, parse_uuid, sql_status, validate_context},
};

#[derive(Debug, Clone, Copy)]
pub(crate) struct ValidatedRequestContext {
    pub actor_principal_id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub workspace_id: Option<Uuid>,
}

pub(crate) fn validated_request_context(
    context: Option<&RequestContext>,
    expected_workspace_id: Option<Uuid>,
) -> Result<ValidatedRequestContext, Status> {
    let context = validate_context(context, expected_workspace_id)?;
    let actor_principal_id = parse_uuid(&context.actor_principal_id, "context.actor_principal_id")?;
    let tenant = context
        .tenant
        .as_ref()
        .ok_or_else(|| Status::invalid_argument("tenant context is required"))?;
    let tenant_id = optional_uuid(&tenant.tenant_id, "context.tenant_id")?;
    let context_workspace_id = optional_uuid(&tenant.workspace_id, "context.workspace_id")?
        .filter(|workspace_id| !workspace_id.is_nil());
    let workspace_id = match (expected_workspace_id, context_workspace_id) {
        (Some(expected), Some(actual)) if expected != actual => {
            return Err(Status::permission_denied(
                "request context workspace does not match the Cloud workspace",
            ));
        }
        (Some(expected), _) => Some(expected),
        (None, actual) => actual,
    };

    Ok(ValidatedRequestContext {
        actor_principal_id,
        tenant_id,
        workspace_id,
    })
}

pub(crate) async fn begin_request_transaction<'a>(
    db: &'a sqlx::PgPool,
    context: Option<&RequestContext>,
    expected_workspace_id: Option<Uuid>,
) -> Result<(Transaction<'a, Postgres>, ValidatedRequestContext), Status> {
    let mut context = validated_request_context(context, expected_workspace_id)?;
    let mut tx = db.begin().await.map_err(sql_status)?;
    set_request_context(&mut tx, context).await?;

    if let Some(workspace_id) = expected_workspace_id {
        let stored_tenant_id =
            sqlx::query_scalar::<_, Option<Uuid>>("SELECT tenant_id FROM workspaces WHERE id = $1")
                .bind(workspace_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(sql_status)?
                .flatten();
        match (context.tenant_id, stored_tenant_id) {
            (Some(request_tenant), Some(stored_tenant)) if request_tenant != stored_tenant => {
                return Err(Status::permission_denied(
                    "request context tenant does not match the Cloud workspace",
                ));
            }
            (None, Some(stored_tenant)) => {
                context.tenant_id = Some(stored_tenant);
                set_request_context(&mut tx, context).await?;
            }
            _ => {}
        }
    }

    Ok((tx, context))
}

pub(crate) fn required_context_workspace_id(
    context: Option<&RequestContext>,
) -> Result<Uuid, Status> {
    validated_request_context(context, None)?
        .workspace_id
        .ok_or_else(|| Status::invalid_argument("context.workspace_id is required"))
}

pub(crate) async fn set_request_context(
    tx: &mut Transaction<'_, Postgres>,
    context: ValidatedRequestContext,
) -> Result<(), Status> {
    nvbes_tenancy::set_transaction_rls_context(
        tx,
        nvbes_tenancy::RlsContext {
            principal_id: Some(context.actor_principal_id),
            user_id: Some(context.actor_principal_id),
            tenant_id: context.tenant_id,
            workspace_id: context.workspace_id,
        },
    )
    .await
    .map_err(sql_status)
}
