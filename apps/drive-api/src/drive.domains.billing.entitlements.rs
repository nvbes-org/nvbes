use crate::http::error::AppError;
use nvbes_billing::types::InvoiceEstimateView;
use sqlx::PgPool;

pub async fn persist_invoice_estimate(
    db: &PgPool,
    estimate: &InvoiceEstimateView,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO invoice_estimates (
          workspace_id,
          billing_period_start,
          billing_period_end,
          estimated_amount_cents,
          currency
        )
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(estimate.workspace_id)
    .bind(estimate.billing_period_start)
    .bind(estimate.billing_period_end)
    .bind(estimate.estimated_amount_cents)
    .bind(&estimate.currency)
    .execute(db)
    .await?;

    Ok(())
}
