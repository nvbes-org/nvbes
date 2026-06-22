use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskFriction {
    None,
    VerifyEmail,
    RequirePaymentMethod,
    ManualReview,
    TrialCaps,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskSignal {
    DisposableEmail,
    TrialCost,
    WorkspaceCount,
    IpVelocity,
    DomainMismatch,
    PaymentFailures,
    UnusualEgress,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RiskDecision {
    pub score: u32,
    pub friction: RiskFriction,
    pub reasons: Vec<RiskSignal>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BillingContact {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegalAddress {
    pub country: String,
    pub line1: String,
    pub postal_code: String,
    pub city: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KycProfile {
    pub company_name: String,
    pub company_domain: String,
    pub vat_id: Option<String>,
    pub billing_contact: BillingContact,
    pub legal_address: LegalAddress,
    pub proof_reference: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KycProfileValidation {
    pub complete: bool,
    pub missing_fields: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CryptoLedgerDirection {
    Credit,
    Debit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CryptoLedgerEntry {
    pub asset_code: String,
    pub amount_atomic: i128,
    pub direction: CryptoLedgerDirection,
    pub source_type: String,
    pub source_id: String,
}

pub fn trial_risk_friction(score: u32) -> RiskFriction {
    match score {
        0..=24 => RiskFriction::None,
        25..=49 => RiskFriction::VerifyEmail,
        50..=74 => RiskFriction::TrialCaps,
        75..=89 => RiskFriction::RequirePaymentMethod,
        _ => RiskFriction::ManualReview,
    }
}

pub fn score_trial_risk(signals: &[RiskSignal], active_paid_customer: bool) -> RiskDecision {
    if active_paid_customer {
        return RiskDecision {
            score: 0,
            friction: RiskFriction::None,
            reasons: Vec::new(),
        };
    }

    let score = signals
        .iter()
        .map(|signal| match signal {
            RiskSignal::DisposableEmail => 25,
            RiskSignal::TrialCost => 20,
            RiskSignal::WorkspaceCount => 15,
            RiskSignal::IpVelocity => 20,
            RiskSignal::DomainMismatch => 10,
            RiskSignal::PaymentFailures => 25,
            RiskSignal::UnusualEgress => 30,
        })
        .sum::<u32>()
        .min(100);

    RiskDecision {
        score,
        friction: trial_risk_friction(score),
        reasons: signals.to_vec(),
    }
}

pub fn validate_kyc_profile(profile: &KycProfile) -> KycProfileValidation {
    let mut missing_fields = Vec::new();
    if profile.company_name.trim().is_empty() {
        missing_fields.push("company_name".to_string());
    }
    if profile.company_domain.trim().is_empty() {
        missing_fields.push("company_domain".to_string());
    }
    if profile.billing_contact.email.trim().is_empty() {
        missing_fields.push("billing_contact.email".to_string());
    }
    if profile.legal_address.country.trim().len() != 2 {
        missing_fields.push("legal_address.country".to_string());
    }
    if profile.legal_address.line1.trim().is_empty() {
        missing_fields.push("legal_address.line1".to_string());
    }
    if profile.legal_address.city.trim().is_empty() {
        missing_fields.push("legal_address.city".to_string());
    }

    KycProfileValidation {
        complete: missing_fields.is_empty(),
        missing_fields,
    }
}

pub fn kyc_requires_manual_review(profile: &KycProfile) -> bool {
    let validation = validate_kyc_profile(profile);
    !validation.complete || profile.proof_reference.is_some()
}

pub fn crypto_ledger_can_create_payment(entry: &CryptoLedgerEntry) -> bool {
    let _ = entry;
    false
}

pub fn crypto_ledger_can_reference_wallet(entry: &CryptoLedgerEntry) -> bool {
    let _ = entry;
    false
}

pub fn crypto_ledger_entry_is_reporting_only(entry: &CryptoLedgerEntry) -> bool {
    entry.amount_atomic != 0
        && !entry.asset_code.trim().is_empty()
        && !entry.source_id.trim().is_empty()
        && !crypto_ledger_can_create_payment(entry)
        && !crypto_ledger_can_reference_wallet(entry)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trial_abuse_applies_caps() {
        let decision = score_trial_risk(
            &[RiskSignal::DisposableEmail, RiskSignal::UnusualEgress],
            false,
        );
        assert_eq!(decision.friction, RiskFriction::TrialCaps);
    }

    #[test]
    fn active_paid_customer_is_not_frictioned_without_reason() {
        let decision = score_trial_risk(&[RiskSignal::UnusualEgress], true);
        assert_eq!(decision.friction, RiskFriction::None);
        assert!(decision.reasons.is_empty());
    }

    #[test]
    fn crypto_ledger_entry_is_not_a_payment_source() {
        assert!(!crypto_ledger_can_create_payment(&CryptoLedgerEntry {
            asset_code: "BTC".to_string(),
            amount_atomic: 1,
            direction: CryptoLedgerDirection::Credit,
            source_type: "internal_credit".to_string(),
            source_id: "credit-1".to_string(),
        }));
    }

    #[test]
    fn lightweight_b2b_kyc_profile_is_proportionate() {
        let profile = KycProfile {
            company_name: "Acme SAS".to_string(),
            company_domain: "acme.example".to_string(),
            vat_id: Some("FR123".to_string()),
            billing_contact: BillingContact {
                name: "Ada".to_string(),
                email: "billing@acme.example".to_string(),
            },
            legal_address: LegalAddress {
                country: "FR".to_string(),
                line1: "1 Rue Example".to_string(),
                postal_code: "75001".to_string(),
                city: "Paris".to_string(),
            },
            proof_reference: None,
        };

        let validation = validate_kyc_profile(&profile);
        assert!(validation.complete);
        assert!(!kyc_requires_manual_review(&profile));
    }

    #[test]
    fn incomplete_kyc_profile_requires_manual_review_without_blocking_paid_customers() {
        let profile = KycProfile {
            company_name: "".to_string(),
            company_domain: "acme.example".to_string(),
            vat_id: None,
            billing_contact: BillingContact {
                name: "Ada".to_string(),
                email: "".to_string(),
            },
            legal_address: LegalAddress {
                country: "F".to_string(),
                line1: "".to_string(),
                postal_code: "".to_string(),
                city: "Paris".to_string(),
            },
            proof_reference: None,
        };

        let validation = validate_kyc_profile(&profile);
        let decision = score_trial_risk(&[RiskSignal::DomainMismatch], true);

        assert!(!validation.complete);
        assert!(
            validation
                .missing_fields
                .contains(&"company_name".to_string())
        );
        assert!(kyc_requires_manual_review(&profile));
        assert_eq!(decision.friction, RiskFriction::None);
    }

    #[test]
    fn crypto_ledger_entry_is_reporting_only_without_wallet_or_payment() {
        let entry = CryptoLedgerEntry {
            asset_code: "BTC".to_string(),
            amount_atomic: 100,
            direction: CryptoLedgerDirection::Debit,
            source_type: "internal_credit_reversal".to_string(),
            source_id: "credit-1".to_string(),
        };

        assert!(crypto_ledger_entry_is_reporting_only(&entry));
        assert!(!crypto_ledger_can_create_payment(&entry));
        assert!(!crypto_ledger_can_reference_wallet(&entry));
    }
}
