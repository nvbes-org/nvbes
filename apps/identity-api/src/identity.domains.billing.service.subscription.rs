use nvbes_billing::subscriptions::{
    PlanChangeRequest, SubscriptionChange, SubscriptionPeriod, SubscriptionStatus,
    decide_plan_change, downgrade_effective_status,
};

pub fn schedule_downgrade(current_status: SubscriptionStatus) -> SubscriptionStatus {
    downgrade_effective_status(current_status)
}

pub fn schedule_plan_change(
    request: PlanChangeRequest,
    period: &SubscriptionPeriod,
) -> SubscriptionChange {
    decide_plan_change(request, period)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::DateTime;

    #[test]
    fn identity_subscription_service_schedules_downgrade_at_period_end() {
        let period = SubscriptionPeriod {
            starts_at: DateTime::from_timestamp(1_735_689_600, 0).unwrap(),
            ends_at: DateTime::from_timestamp(1_738_368_000, 0).unwrap(),
        };
        let change = schedule_plan_change(
            PlanChangeRequest {
                old_amount_minor: 2_000,
                new_amount_minor: 1_000,
                now: DateTime::from_timestamp(1_736_000_000, 0).unwrap(),
                admin_override: false,
            },
            &period,
        );

        assert_eq!(change.change_type, "downgrade");
        assert_eq!(change.effective_at, period.ends_at);
    }
}
