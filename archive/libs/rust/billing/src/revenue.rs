use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevenueScheduleLine {
    pub recognition_date: NaiveDate,
    pub amount_minor: i64,
    pub currency: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RevenueScheduleKind {
    MonthlyRecognized,
    AnnualDeferred,
    CreditNoteReversal,
}

pub fn recognize_evenly(
    total_minor: i64,
    months: u32,
    first_month: NaiveDate,
    currency: &str,
) -> Vec<RevenueScheduleLine> {
    if months == 0 {
        return Vec::new();
    }
    let base = total_minor / i64::from(months);
    let remainder = total_minor % i64::from(months);
    (0..months)
        .map(|index| RevenueScheduleLine {
            recognition_date: first_month + chrono::Duration::days(i64::from(index) * 31),
            amount_minor: base + if index == 0 { remainder } else { 0 },
            currency: currency.to_string(),
        })
        .collect()
}

pub fn annual_deferred_schedule(
    total_minor: i64,
    first_month: NaiveDate,
    currency: &str,
) -> Vec<RevenueScheduleLine> {
    recognize_evenly(total_minor, 12, first_month, currency)
}

pub fn credit_note_reversal(
    amount_minor: i64,
    recognition_date: NaiveDate,
    currency: &str,
) -> RevenueScheduleLine {
    RevenueScheduleLine {
        recognition_date,
        amount_minor: -amount_minor,
        currency: currency.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn annual_invoice_is_recognized_monthly() {
        let schedule =
            annual_deferred_schedule(12_000, NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(), "EUR");

        assert_eq!(schedule.len(), 12);
        assert_eq!(
            schedule.iter().map(|line| line.amount_minor).sum::<i64>(),
            12_000
        );
    }

    #[test]
    fn credit_note_reversal_is_negative_revenue() {
        let line = credit_note_reversal(1_000, NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(), "EUR");
        assert_eq!(line.amount_minor, -1_000);
    }

    #[test]
    fn recognize_evenly_returns_empty_for_zero_months() {
        assert!(
            recognize_evenly(
                1_000,
                0,
                NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                "EUR"
            )
            .is_empty()
        );
    }

    #[test]
    fn recognize_evenly_spreads_remainder_on_first_month() {
        let schedule = recognize_evenly(
            10_000,
            3,
            NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            "EUR",
        );
        assert_eq!(schedule.len(), 3);
        assert_eq!(schedule[0].amount_minor, 3_334);
        assert_eq!(schedule[1].amount_minor, 3_333);
        assert_eq!(schedule[2].amount_minor, 3_333);
        assert_eq!(
            schedule[1].recognition_date,
            NaiveDate::from_ymd_opt(2026, 1, 1).unwrap() + chrono::Duration::days(31)
        );
        assert_eq!(
            schedule.iter().map(|line| line.amount_minor).sum::<i64>(),
            10_000
        );
    }
}
