use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubscriptionMetricInput {
    pub tenant_id: String,
    pub current_mrr_minor: i64,
    pub previous_mrr_minor: i64,
    pub was_trial: bool,
    pub converted_from_trial: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TenantMarginInput {
    pub tenant_id: String,
    pub revenue_minor: i64,
    pub cost_minor: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FinanceKpis {
    pub mrr_minor: i64,
    pub arr_minor: i64,
    pub churned_count: u32,
    pub expansion_minor: i64,
    pub contraction_minor: i64,
    pub trial_conversion_rate_bps: i64,
    pub gross_margin_bps: i64,
    pub negative_margin_tenant_ids: Vec<String>,
}

pub fn calculate_finance_kpis(
    subscriptions: &[SubscriptionMetricInput],
    margins: &[TenantMarginInput],
) -> FinanceKpis {
    let mrr_minor = subscriptions
        .iter()
        .map(|subscription| subscription.current_mrr_minor)
        .sum::<i64>();
    let trial_count = subscriptions
        .iter()
        .filter(|subscription| subscription.was_trial)
        .count() as i64;
    let trial_conversions = subscriptions
        .iter()
        .filter(|subscription| subscription.converted_from_trial)
        .count() as i64;
    let revenue_minor = margins
        .iter()
        .map(|margin| margin.revenue_minor)
        .sum::<i64>();
    let cost_minor = margins.iter().map(|margin| margin.cost_minor).sum::<i64>();

    FinanceKpis {
        mrr_minor,
        arr_minor: mrr_minor.saturating_mul(12),
        churned_count: subscriptions
            .iter()
            .filter(|subscription| {
                subscription.previous_mrr_minor > 0 && subscription.current_mrr_minor == 0
            })
            .count() as u32,
        expansion_minor: subscriptions
            .iter()
            .map(|subscription| {
                (subscription.current_mrr_minor - subscription.previous_mrr_minor).max(0)
            })
            .sum(),
        contraction_minor: subscriptions
            .iter()
            .map(|subscription| {
                (subscription.previous_mrr_minor - subscription.current_mrr_minor).max(0)
            })
            .sum(),
        trial_conversion_rate_bps: ratio_bps(trial_conversions, trial_count),
        gross_margin_bps: ratio_bps(revenue_minor - cost_minor, revenue_minor),
        negative_margin_tenant_ids: margins
            .iter()
            .filter(|margin| margin.cost_minor > margin.revenue_minor)
            .map(|margin| margin.tenant_id.clone())
            .collect(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FinanceAnomaly {
    UsageSpike { current: i64, baseline: i64 },
    FailedPaymentCluster { failed_count: u32 },
    LedgerImbalance,
    ProviderWebhookLag { lag_seconds: i64 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FinanceAnomalyInput {
    pub usage_current: i64,
    pub usage_baseline: i64,
    pub failed_payment_count: u32,
    pub ledger_balances: bool,
    pub provider_webhook_lag_seconds: i64,
}

pub fn detect_finance_anomalies(input: FinanceAnomalyInput) -> Vec<FinanceAnomaly> {
    let mut anomalies = Vec::new();
    if input.usage_baseline > 0 && input.usage_current >= input.usage_baseline.saturating_mul(3) {
        anomalies.push(FinanceAnomaly::UsageSpike {
            current: input.usage_current,
            baseline: input.usage_baseline,
        });
    }
    if input.failed_payment_count >= 5 {
        anomalies.push(FinanceAnomaly::FailedPaymentCluster {
            failed_count: input.failed_payment_count,
        });
    }
    if !input.ledger_balances {
        anomalies.push(FinanceAnomaly::LedgerImbalance);
    }
    if input.provider_webhook_lag_seconds >= 900 {
        anomalies.push(FinanceAnomaly::ProviderWebhookLag {
            lag_seconds: input.provider_webhook_lag_seconds,
        });
    }
    anomalies
}

fn ratio_bps(numerator: i64, denominator: i64) -> i64 {
    if denominator <= 0 {
        return 0;
    }
    numerator.saturating_mul(10_000) / denominator
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finance_kpis_come_from_internal_subscription_metrics() {
        let kpis = calculate_finance_kpis(
            &[
                SubscriptionMetricInput {
                    tenant_id: "tenant-a".to_string(),
                    current_mrr_minor: 10_000,
                    previous_mrr_minor: 8_000,
                    was_trial: true,
                    converted_from_trial: true,
                },
                SubscriptionMetricInput {
                    tenant_id: "tenant-b".to_string(),
                    current_mrr_minor: 0,
                    previous_mrr_minor: 4_000,
                    was_trial: true,
                    converted_from_trial: false,
                },
            ],
            &[TenantMarginInput {
                tenant_id: "tenant-a".to_string(),
                revenue_minor: 10_000,
                cost_minor: 3_000,
            }],
        );

        assert_eq!(kpis.mrr_minor, 10_000);
        assert_eq!(kpis.arr_minor, 120_000);
        assert_eq!(kpis.churned_count, 1);
        assert_eq!(kpis.expansion_minor, 2_000);
        assert_eq!(kpis.contraction_minor, 4_000);
        assert_eq!(kpis.trial_conversion_rate_bps, 5_000);
        assert_eq!(kpis.gross_margin_bps, 7_000);
    }

    #[test]
    fn negative_margin_tenants_are_identified() {
        let kpis = calculate_finance_kpis(
            &[],
            &[TenantMarginInput {
                tenant_id: "tenant-loss".to_string(),
                revenue_minor: 1_000,
                cost_minor: 1_500,
            }],
        );

        assert_eq!(kpis.negative_margin_tenant_ids, ["tenant-loss"]);
    }

    #[test]
    fn finance_anomalies_detect_usage_payments_ledger_and_webhook_lag() {
        let anomalies = detect_finance_anomalies(FinanceAnomalyInput {
            usage_current: 3_000,
            usage_baseline: 1_000,
            failed_payment_count: 5,
            ledger_balances: false,
            provider_webhook_lag_seconds: 900,
        });

        assert_eq!(anomalies.len(), 4);
        assert!(matches!(anomalies[0], FinanceAnomaly::UsageSpike { .. }));
        assert!(matches!(
            anomalies[1],
            FinanceAnomaly::FailedPaymentCluster { .. }
        ));
        assert_eq!(anomalies[2], FinanceAnomaly::LedgerImbalance);
        assert!(matches!(
            anomalies[3],
            FinanceAnomaly::ProviderWebhookLag { .. }
        ));
    }

    #[test]
    fn finance_anomalies_skip_below_thresholds() {
        let anomalies = detect_finance_anomalies(FinanceAnomalyInput {
            usage_current: 2_999,
            usage_baseline: 1_000,
            failed_payment_count: 4,
            ledger_balances: true,
            provider_webhook_lag_seconds: 899,
        });
        assert!(anomalies.is_empty());

        let anomalies = detect_finance_anomalies(FinanceAnomalyInput {
            usage_current: 10_000,
            usage_baseline: 0,
            failed_payment_count: 0,
            ledger_balances: true,
            provider_webhook_lag_seconds: 0,
        });
        assert!(anomalies.is_empty());
    }

    #[test]
    fn ratio_bps_is_zero_when_denominator_non_positive() {
        let kpis = calculate_finance_kpis(
            &[SubscriptionMetricInput {
                tenant_id: "tenant-a".to_string(),
                current_mrr_minor: 1_000,
                previous_mrr_minor: 1_000,
                was_trial: false,
                converted_from_trial: false,
            }],
            &[TenantMarginInput {
                tenant_id: "tenant-a".to_string(),
                revenue_minor: 0,
                cost_minor: 100,
            }],
        );
        assert_eq!(kpis.trial_conversion_rate_bps, 0);
        assert_eq!(kpis.gross_margin_bps, 0);
        assert_eq!(kpis.churned_count, 0);
        assert_eq!(kpis.expansion_minor, 0);
        assert_eq!(kpis.contraction_minor, 0);
    }

    #[test]
    fn churn_ignores_subscriptions_without_previous_mrr() {
        let kpis = calculate_finance_kpis(
            &[
                SubscriptionMetricInput {
                    tenant_id: "new".to_string(),
                    current_mrr_minor: 0,
                    previous_mrr_minor: 0,
                    was_trial: false,
                    converted_from_trial: false,
                },
                SubscriptionMetricInput {
                    tenant_id: "growing".to_string(),
                    current_mrr_minor: 2_000,
                    previous_mrr_minor: 1_000,
                    was_trial: false,
                    converted_from_trial: false,
                },
            ],
            &[],
        );
        assert_eq!(kpis.churned_count, 0);
        assert_eq!(kpis.expansion_minor, 1_000);
    }
}
