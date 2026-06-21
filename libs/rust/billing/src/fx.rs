use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FxRate {
    pub base_currency: String,
    pub quote_currency: String,
    pub rate: f64,
    pub provider: String,
    pub observed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InvoiceFxSnapshot {
    pub invoice_id: String,
    pub source_amount_minor: i64,
    pub converted_amount_minor: i64,
    pub rate: FxRate,
    pub snapshotted_at: DateTime<Utc>,
}

pub fn convert_minor_units(amount_minor: i64, rate: &FxRate) -> i64 {
    ((amount_minor as f64) * rate.rate).round() as i64
}

pub fn invoice_fx_snapshot(
    invoice_id: &str,
    amount_minor: i64,
    rate: FxRate,
    snapshotted_at: DateTime<Utc>,
) -> InvoiceFxSnapshot {
    InvoiceFxSnapshot {
        invoice_id: invoice_id.to_string(),
        source_amount_minor: amount_minor,
        converted_amount_minor: convert_minor_units(amount_minor, &rate),
        rate,
        snapshotted_at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fx_snapshot_preserves_invoice_conversion() {
        let observed_at = DateTime::from_timestamp(1_782_000_000, 0).unwrap();
        let snapshotted_at = DateTime::from_timestamp(1_782_000_060, 0).unwrap();
        let snapshot = invoice_fx_snapshot(
            "inv_1",
            1_000,
            FxRate {
                base_currency: "USD".to_string(),
                quote_currency: "EUR".to_string(),
                rate: 0.91,
                provider: "ecb".to_string(),
                observed_at,
            },
            snapshotted_at,
        );

        assert_eq!(snapshot.converted_amount_minor, 910);
        assert_eq!(snapshot.rate.provider, "ecb");
        assert_eq!(snapshot.rate.observed_at, observed_at);
        assert_eq!(snapshot.snapshotted_at, snapshotted_at);
    }

    #[test]
    fn existing_fx_snapshot_does_not_change_when_later_rate_changes() {
        let observed_at = DateTime::from_timestamp(1_782_000_000, 0).unwrap();
        let snapshotted_at = DateTime::from_timestamp(1_782_000_060, 0).unwrap();
        let snapshot = invoice_fx_snapshot(
            "inv_1",
            1_000,
            FxRate {
                base_currency: "USD".to_string(),
                quote_currency: "EUR".to_string(),
                rate: 0.91,
                provider: "ecb".to_string(),
                observed_at,
            },
            snapshotted_at,
        );
        let later_rate = FxRate {
            base_currency: "USD".to_string(),
            quote_currency: "EUR".to_string(),
            rate: 0.95,
            provider: "ecb".to_string(),
            observed_at: DateTime::from_timestamp(1_782_086_400, 0).unwrap(),
        };

        assert_eq!(snapshot.converted_amount_minor, 910);
        assert_eq!(convert_minor_units(1_000, &later_rate), 950);
    }
}
