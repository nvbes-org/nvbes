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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegionalPriceSelection {
    pub country_code: Option<String>,
    pub pricing_region: Option<String>,
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

pub fn regional_price_selection(country_code: Option<&str>) -> RegionalPriceSelection {
    let country_code = country_code.and_then(normalize_billing_country);
    let pricing_region = country_code
        .as_deref()
        .and_then(country_pricing_region)
        .map(str::to_string);

    RegionalPriceSelection {
        country_code,
        pricing_region,
    }
}

pub fn normalize_billing_country(country_code: &str) -> Option<String> {
    let normalized = country_code.trim().to_ascii_uppercase();
    (normalized.len() == 2 && normalized.chars().all(|ch| ch.is_ascii_alphabetic()))
        .then_some(normalized)
}

fn country_pricing_region(country_code: &str) -> Option<&'static str> {
    match country_code {
        "US" | "CA" => Some("north_america"),
        "AR" | "BO" | "BR" | "CL" | "CO" | "CR" | "EC" | "MX" | "PE" | "UY" => Some("latam"),
        "AT" | "BE" | "CH" | "DE" | "DK" | "ES" | "FI" | "FR" | "GB" | "IE" | "IT" | "LU"
        | "NL" | "NO" | "PT" | "SE" => Some("western_europe"),
        "BG" | "CZ" | "EE" | "GR" | "HR" | "HU" | "LT" | "LV" | "PL" | "RO" | "SI" | "SK" => {
            Some("eastern_europe")
        }
        "AE" | "BH" | "EG" | "IL" | "JO" | "KW" | "MA" | "QA" | "SA" | "TN" | "TR" => Some("mena"),
        "BD" | "IN" | "LK" | "NP" | "PK" => Some("south_asia"),
        "ID" | "MY" | "PH" | "TH" | "VN" => Some("southeast_asia"),
        "AU" | "HK" | "JP" | "KR" | "NZ" | "SG" | "TW" => Some("apac"),
        "GH" | "KE" | "NG" | "ZA" => Some("africa"),
        _ => None,
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
    fn regional_price_selection_normalizes_country_and_region() {
        assert_eq!(
            regional_price_selection(Some(" fr ")),
            RegionalPriceSelection {
                country_code: Some("FR".to_string()),
                pricing_region: Some("western_europe".to_string()),
            }
        );
    }

    #[test]
    fn regional_price_selection_rejects_invalid_country() {
        assert_eq!(
            regional_price_selection(Some("FRA")),
            RegionalPriceSelection {
                country_code: None,
                pricing_region: None,
            }
        );
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

    #[test]
    fn active_plan_price_falls_back_to_active_monthly_catalog_price() {
        let plan_version_id = Uuid::new_v4();
        let prices = vec![
            PriceVersion {
                id: Uuid::new_v4(),
                plan_version_id: Some(plan_version_id),
                meter_code: None,
                amount: Money::eur(3_900),
                interval_unit: BillingInterval::Year,
                active: true,
            },
            PriceVersion {
                id: Uuid::new_v4(),
                plan_version_id: Some(plan_version_id),
                meter_code: None,
                amount: Money::eur(2_900),
                interval_unit: BillingInterval::Month,
                active: false,
            },
            PriceVersion {
                id: Uuid::new_v4(),
                plan_version_id: Some(plan_version_id),
                meter_code: None,
                amount: Money::eur(3_500),
                interval_unit: BillingInterval::Month,
                active: true,
            },
        ];

        let selected = active_plan_price(PriceSelection {
            plan_version_id,
            tenant_price_version_id: None,
            prices: &prices,
        })
        .expect("active monthly price should be selected");
        assert_eq!(selected.amount.amount_minor, 3_500);

        assert!(
            active_plan_price(PriceSelection {
                plan_version_id: Uuid::new_v4(),
                tenant_price_version_id: None,
                prices: &prices,
            })
            .is_none()
        );
    }
}
