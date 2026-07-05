use serde::Serialize;
use sqlx::{PgPool, Row};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::provider::ProviderCode;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BillingPortalPaymentMethodProviderView {
    #[schema(value_type = ProviderCode)]
    pub provider: String,
    pub status: String,
    pub mandate_status: String,
    pub reusable: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BillingPortalPaymentMethodView {
    pub payment_method_id: Uuid,
    pub method_type: String,
    pub display_label: Option<String>,
    pub brand: Option<String>,
    pub last4: Option<String>,
    pub exp_month: Option<i16>,
    pub exp_year: Option<i16>,
    pub funding: Option<String>,
    pub issuer_country: Option<String>,
    pub status: String,
    pub is_primary: bool,
    pub providers: Vec<BillingPortalPaymentMethodProviderView>,
}

pub async fn fetch_portal_payment_methods(
    db: &PgPool,
    workspace_id: Uuid,
) -> Result<Vec<BillingPortalPaymentMethodView>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT
          pm.id AS payment_method_id,
          pm.method_type,
          pm.display_label,
          pm.brand,
          pm.last4::text AS last4,
          pm.exp_month,
          pm.exp_year,
          pm.funding,
          pm.issuer_country::text AS issuer_country,
          pm.status,
          pm.is_primary,
          ppm.provider::text AS provider,
          ppm.status AS provider_status,
          ppm.mandate_status,
          ppm.reusable
        FROM billing_payment_methods pm
        JOIN billing_accounts a ON a.id = pm.billing_account_id
        LEFT JOIN billing_provider_payment_methods ppm ON ppm.payment_method_id = pm.id
        WHERE a.workspace_id = $1
        ORDER BY pm.is_primary DESC, pm.updated_at DESC, ppm.provider::text
        LIMIT 50
        "#,
    )
    .bind(workspace_id)
    .fetch_all(db)
    .await?;

    let mut methods: Vec<BillingPortalPaymentMethodView> = Vec::new();
    for row in rows {
        let payment_method_id: Uuid = row.get("payment_method_id");
        let index = methods
            .iter()
            .position(|method| method.payment_method_id == payment_method_id);
        let method_index = match index {
            Some(index) => index,
            None => {
                methods.push(BillingPortalPaymentMethodView {
                    payment_method_id,
                    method_type: row.get("method_type"),
                    display_label: row.get("display_label"),
                    brand: row.get("brand"),
                    last4: row.get("last4"),
                    exp_month: row.get("exp_month"),
                    exp_year: row.get("exp_year"),
                    funding: row.get("funding"),
                    issuer_country: row.get("issuer_country"),
                    status: row.get("status"),
                    is_primary: row.get("is_primary"),
                    providers: Vec::new(),
                });
                methods.len() - 1
            }
        };
        if let Some(provider) = row.get::<Option<String>, _>("provider") {
            methods[method_index]
                .providers
                .push(BillingPortalPaymentMethodProviderView {
                    provider,
                    status: row.get("provider_status"),
                    mandate_status: row.get("mandate_status"),
                    reusable: row.get("reusable"),
                });
        }
    }

    Ok(methods)
}
