use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaxInput {
    pub seller_country: String,
    pub customer_country: String,
    pub customer_vat_id: Option<String>,
    pub customer_is_business: bool,
    pub amount_minor: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaxResult {
    pub tax_minor: i64,
    pub rate_basis_points: i64,
    pub reverse_charge: bool,
    pub evidence_required: bool,
    pub decision_reason: String,
}

pub fn calculate_eu_tax(input: &TaxInput, rate_basis_points: i64) -> TaxResult {
    let reverse_charge = input.customer_is_business
        && input.customer_vat_id.is_some()
        && input.seller_country != input.customer_country;
    let tax_minor = if reverse_charge {
        0
    } else {
        input.amount_minor * rate_basis_points / 10_000
    };

    TaxResult {
        tax_minor,
        rate_basis_points,
        reverse_charge,
        evidence_required: true,
        decision_reason: if reverse_charge {
            "eu_b2b_reverse_charge".to_string()
        } else {
            "eu_customer_country_vat".to_string()
        },
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaxEvidenceSource {
    BillingAddress,
    VatId,
    IpCountry,
    ProviderPaymentCountry,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaxEvidence {
    pub source: TaxEvidenceSource,
    pub country: String,
    pub collected_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceTaxSnapshot {
    pub invoice_id: String,
    pub seller_country: String,
    pub customer_country: String,
    pub customer_vat_id: Option<String>,
    pub customer_is_business: bool,
    pub amount_minor: i64,
    pub tax_minor: i64,
    pub rate_basis_points: i64,
    pub reverse_charge: bool,
    pub decision_reason: String,
    pub evidence: Vec<TaxEvidence>,
    pub calculated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegionPolicy {
    pub country: String,
    pub allowed_payment_methods: Vec<String>,
    pub invoice_retention_years: u16,
    pub requires_tax_evidence: bool,
    pub einvoicing_profile_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EInvoicingProfile {
    pub code: String,
    pub country: String,
    pub format: String,
    pub enabled: bool,
}

pub fn invoice_tax_snapshot(
    invoice_id: &str,
    input: &TaxInput,
    rate_basis_points: i64,
    evidence: Vec<TaxEvidence>,
    calculated_at: DateTime<Utc>,
) -> InvoiceTaxSnapshot {
    let result = calculate_eu_tax(input, rate_basis_points);
    InvoiceTaxSnapshot {
        invoice_id: invoice_id.to_string(),
        seller_country: input.seller_country.clone(),
        customer_country: input.customer_country.clone(),
        customer_vat_id: input.customer_vat_id.clone(),
        customer_is_business: input.customer_is_business,
        amount_minor: input.amount_minor,
        tax_minor: result.tax_minor,
        rate_basis_points: result.rate_basis_points,
        reverse_charge: result.reverse_charge,
        decision_reason: result.decision_reason,
        evidence,
        calculated_at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn b2b_cross_border_reverse_charge_has_zero_tax() {
        let result = calculate_eu_tax(
            &TaxInput {
                seller_country: "FR".to_string(),
                customer_country: "DE".to_string(),
                customer_vat_id: Some("DE123".to_string()),
                customer_is_business: true,
                amount_minor: 10_000,
            },
            2_000,
        );
        assert!(result.reverse_charge);
        assert_eq!(result.tax_minor, 0);
        assert_eq!(result.decision_reason, "eu_b2b_reverse_charge");
    }

    #[test]
    fn b2c_customer_country_vat_is_applied() {
        let result = calculate_eu_tax(
            &TaxInput {
                seller_country: "FR".to_string(),
                customer_country: "FR".to_string(),
                customer_vat_id: None,
                customer_is_business: false,
                amount_minor: 10_000,
            },
            2_000,
        );

        assert!(!result.reverse_charge);
        assert_eq!(result.tax_minor, 2_000);
        assert_eq!(result.decision_reason, "eu_customer_country_vat");
    }

    #[test]
    fn invoice_tax_snapshot_keeps_historical_rate_and_evidence() {
        let calculated_at = DateTime::from_timestamp(1_782_000_000, 0).unwrap();
        let input = TaxInput {
            seller_country: "FR".to_string(),
            customer_country: "BE".to_string(),
            customer_vat_id: None,
            customer_is_business: false,
            amount_minor: 10_000,
        };

        let snapshot = invoice_tax_snapshot(
            "invoice-1",
            &input,
            2_100,
            vec![TaxEvidence {
                source: TaxEvidenceSource::BillingAddress,
                country: "BE".to_string(),
                collected_at: calculated_at,
            }],
            calculated_at,
        );
        let later_result = calculate_eu_tax(&input, 2_000);

        assert_eq!(snapshot.rate_basis_points, 2_100);
        assert_eq!(snapshot.tax_minor, 2_100);
        assert_eq!(later_result.tax_minor, 2_000);
        assert_eq!(
            snapshot.evidence[0].source,
            TaxEvidenceSource::BillingAddress
        );
    }

    #[test]
    fn einvoicing_profile_can_be_modelled_without_activation() {
        let policy = RegionPolicy {
            country: "FR".to_string(),
            allowed_payment_methods: vec!["card".to_string(), "sepa_debit".to_string()],
            invoice_retention_years: 10,
            requires_tax_evidence: true,
            einvoicing_profile_code: Some("fr-factur-x".to_string()),
        };
        let profile = EInvoicingProfile {
            code: "fr-factur-x".to_string(),
            country: "FR".to_string(),
            format: "Factur-X".to_string(),
            enabled: false,
        };

        assert_eq!(
            policy.einvoicing_profile_code.as_deref(),
            Some(profile.code.as_str())
        );
        assert!(!profile.enabled);
    }
}
