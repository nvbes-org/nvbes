use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Money {
    pub amount_minor: i64,
    pub currency: String,
}

impl Money {
    pub fn eur(amount_minor: i64) -> Self {
        Self {
            amount_minor,
            currency: "EUR".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PriceVersion {
    pub id: Uuid,
    pub plan_version_id: Option<Uuid>,
    pub meter_code: Option<String>,
    pub amount: Money,
    pub interval_unit: BillingInterval,
    pub active: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct PriceSelection<'a> {
    pub plan_version_id: Uuid,
    pub tenant_price_version_id: Option<Uuid>,
    pub prices: &'a [PriceVersion],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BillingInterval {
    Month,
    Year,
    OneTime,
    Usage,
}

pub fn plan_monthly_price_cents(plan_code: &str) -> i64 {
    match plan_code {
        "solo_pro" => 1_500,
        "team" => 3_900,
        "team_plus" => 7_900,
        _ => 0,
    }
}

pub fn active_plan_price(selection: PriceSelection<'_>) -> Option<&PriceVersion> {
    selection
        .tenant_price_version_id
        .and_then(|id| selection.prices.iter().find(|price| price.id == id))
        .or_else(|| {
            selection.prices.iter().find(|price| {
                price.plan_version_id == Some(selection.plan_version_id)
                    && price.interval_unit == BillingInterval::Month
                    && price.active
            })
        })
}

pub fn preserve_historical_price(
    invoice_price: &PriceVersion,
    current_price: &PriceVersion,
) -> i64 {
    let _ = current_price;
    invoice_price.amount.amount_minor
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_monthly_price_cents_keeps_legacy_catalog_values() {
        assert_eq!(plan_monthly_price_cents("solo_pro"), 1_500);
        assert_eq!(plan_monthly_price_cents("team"), 3_900);
        assert_eq!(plan_monthly_price_cents("team_plus"), 7_900);
        assert_eq!(plan_monthly_price_cents("trial"), 0);
    }

    #[test]
    fn preserve_historical_price_ignores_current_price_changes() {
        let invoice_price = PriceVersion {
            id: Uuid::nil(),
            plan_version_id: None,
            meter_code: None,
            amount: Money::eur(3_900),
            interval_unit: BillingInterval::Month,
            active: false,
        };
        let current_price = PriceVersion {
            amount: Money::eur(4_900),
            active: true,
            ..invoice_price.clone()
        };

        assert_eq!(
            preserve_historical_price(&invoice_price, &current_price),
            3_900
        );
    }

    #[test]
    fn tenant_can_stay_on_older_price_version() {
        let plan_version_id = Uuid::new_v4();
        let old_price_id = Uuid::new_v4();
        let prices = vec![
            PriceVersion {
                id: old_price_id,
                plan_version_id: Some(plan_version_id),
                meter_code: None,
                amount: Money::eur(3_900),
                interval_unit: BillingInterval::Month,
                active: false,
            },
            PriceVersion {
                id: Uuid::new_v4(),
                plan_version_id: Some(plan_version_id),
                meter_code: None,
                amount: Money::eur(4_900),
                interval_unit: BillingInterval::Month,
                active: true,
            },
        ];

        let selected = active_plan_price(PriceSelection {
            plan_version_id,
            tenant_price_version_id: Some(old_price_id),
            prices: &prices,
        })
        .expect("tenant-specific price should be selected");

        assert_eq!(selected.amount.amount_minor, 3_900);
    }
}
