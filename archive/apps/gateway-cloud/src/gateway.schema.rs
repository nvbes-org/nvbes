use async_graphql::{Context, EmptySubscription, InputObject, Object, Result, Schema};

use crate::{
    billing_client::billing_client,
    pb::nvbes::billing::v1::{
        CreateCheckoutRequest, CreatePortalRequest, GetBillingOverviewRequest,
        GetBillingPortalRequest,
    },
    schema_types::{
        BillingOverview, BillingPortal, CheckoutSession, PortalSession, ProductEntitlements,
    },
    state::{GatewayRequestContext, GatewayState},
};

pub type GatewaySchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

pub fn schema(state: GatewayState) -> GatewaySchema {
    Schema::build(QueryRoot, MutationRoot, EmptySubscription)
        .data(state)
        .finish()
}

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn billing_overview(
        &self,
        ctx: &Context<'_>,
        workspace_id: String,
    ) -> Result<BillingOverview> {
        let state = ctx.data::<GatewayState>()?;
        let request_context = ctx
            .data::<GatewayRequestContext>()?
            .for_workspace(&workspace_id);
        let mut client = billing_client(&state.billing_grpc_endpoint).await?;
        let response = client
            .get_billing_overview(GetBillingOverviewRequest {
                context: Some(request_context),
                workspace_id,
            })
            .await?
            .into_inner();
        BillingOverview::from_grpc(response)
    }

    async fn billing_portal(
        &self,
        ctx: &Context<'_>,
        workspace_id: String,
    ) -> Result<BillingPortal> {
        let state = ctx.data::<GatewayState>()?;
        let request_context = ctx
            .data::<GatewayRequestContext>()?
            .for_workspace(&workspace_id);
        let mut client = billing_client(&state.billing_grpc_endpoint).await?;
        let response = client
            .get_billing_portal(GetBillingPortalRequest {
                context: Some(request_context),
                workspace_id,
            })
            .await?
            .into_inner();
        BillingPortal::from_grpc(response)
    }

    async fn billing_entitlements(
        &self,
        ctx: &Context<'_>,
        workspace_id: String,
    ) -> Result<ProductEntitlements> {
        let overview = self.billing_overview(ctx, workspace_id).await?;
        Ok(overview.entitlements)
    }
}

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    async fn create_billing_checkout(
        &self,
        ctx: &Context<'_>,
        input: CreateBillingCheckoutInput,
    ) -> Result<CheckoutSession> {
        let state = ctx.data::<GatewayState>()?;
        let request_context = ctx
            .data::<GatewayRequestContext>()?
            .for_workspace(&input.workspace_id);
        let mut client = billing_client(&state.billing_grpc_endpoint).await?;
        let response = client
            .create_checkout(CreateCheckoutRequest {
                context: Some(request_context),
                workspace_id: input.workspace_id,
                plan_code: input.plan_code,
                success_url: input.success_url.unwrap_or_default(),
                cancel_url: input.cancel_url.unwrap_or_default(),
            })
            .await?
            .into_inner();
        CheckoutSession::from_grpc(response)
    }

    async fn create_billing_portal_session(
        &self,
        ctx: &Context<'_>,
        input: CreateBillingPortalSessionInput,
    ) -> Result<PortalSession> {
        let state = ctx.data::<GatewayState>()?;
        let request_context = ctx
            .data::<GatewayRequestContext>()?
            .for_workspace(&input.workspace_id);
        let mut client = billing_client(&state.billing_grpc_endpoint).await?;
        let response = client
            .create_portal(CreatePortalRequest {
                context: Some(request_context),
                workspace_id: input.workspace_id,
                return_url: input.return_url.unwrap_or_default(),
            })
            .await?
            .into_inner();
        PortalSession::from_grpc(response)
    }
}

#[derive(InputObject)]
pub struct CreateBillingCheckoutInput {
    pub workspace_id: String,
    pub plan_code: String,
    pub success_url: Option<String>,
    pub cancel_url: Option<String>,
}

#[derive(InputObject)]
pub struct CreateBillingPortalSessionInput {
    pub workspace_id: String,
    pub return_url: Option<String>,
}
