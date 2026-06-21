use chrono::{DateTime, Datelike, Duration, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubscriptionStatus {
    Trialing,
    Active,
    Grace,
    PastDue,
    Degraded,
    Suspended,
    Canceled,
    Ended,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubscriptionInterval {
    Monthly,
    Annual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubscriptionPeriod {
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubscriptionChange {
    pub change_type: String,
    pub effective_at: DateTime<Utc>,
    pub proration_amount_minor: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanChangeRequest {
    pub old_amount_minor: i64,
    pub new_amount_minor: i64,
    pub now: DateTime<Utc>,
    pub admin_override: bool,
}

pub fn next_period(start: DateTime<Utc>, interval: SubscriptionInterval) -> SubscriptionPeriod {
    let days = match interval {
        SubscriptionInterval::Monthly => days_in_month(start),
        SubscriptionInterval::Annual => 365,
    };
    SubscriptionPeriod {
        starts_at: start,
        ends_at: start + Duration::days(days.into()),
    }
}

pub fn trial_period(start: DateTime<Utc>, trial_days: i64) -> SubscriptionPeriod {
    SubscriptionPeriod {
        starts_at: start,
        ends_at: start + Duration::days(trial_days.max(0)),
    }
}

pub fn grace_period(period: &SubscriptionPeriod, grace_days: i64) -> SubscriptionPeriod {
    SubscriptionPeriod {
        starts_at: period.ends_at,
        ends_at: period.ends_at + Duration::days(grace_days.max(0)),
    }
}

pub fn prorated_upgrade_amount(
    old_amount_minor: i64,
    new_amount_minor: i64,
    elapsed_seconds: i64,
    period_seconds: i64,
) -> i64 {
    if new_amount_minor <= old_amount_minor
        || elapsed_seconds >= period_seconds
        || period_seconds <= 0
    {
        return 0;
    }
    let remaining_seconds = period_seconds - elapsed_seconds.max(0);
    ((new_amount_minor - old_amount_minor) * remaining_seconds + period_seconds - 1)
        / period_seconds
}

pub fn decide_plan_change(
    request: PlanChangeRequest,
    period: &SubscriptionPeriod,
) -> SubscriptionChange {
    if request.new_amount_minor > request.old_amount_minor {
        return immediate_upgrade_change(
            request.now,
            request.old_amount_minor,
            request.new_amount_minor,
            period,
        );
    }

    if request.admin_override {
        return SubscriptionChange {
            change_type: "downgrade_override".to_string(),
            effective_at: request.now,
            proration_amount_minor: 0,
        };
    }

    period_end_downgrade_change(period)
}

pub fn downgrade_effective_status(current: SubscriptionStatus) -> SubscriptionStatus {
    match current {
        SubscriptionStatus::Suspended
        | SubscriptionStatus::Canceled
        | SubscriptionStatus::Ended => current,
        _ => SubscriptionStatus::Active,
    }
}

pub fn immediate_upgrade_change(
    now: DateTime<Utc>,
    old_amount_minor: i64,
    new_amount_minor: i64,
    period: &SubscriptionPeriod,
) -> SubscriptionChange {
    let elapsed_seconds = (now - period.starts_at).num_seconds();
    let period_seconds = (period.ends_at - period.starts_at).num_seconds();
    SubscriptionChange {
        change_type: "upgrade".to_string(),
        effective_at: now,
        proration_amount_minor: prorated_upgrade_amount(
            old_amount_minor,
            new_amount_minor,
            elapsed_seconds,
            period_seconds,
        ),
    }
}

pub fn period_end_downgrade_change(period: &SubscriptionPeriod) -> SubscriptionChange {
    SubscriptionChange {
        change_type: "downgrade".to_string(),
        effective_at: period.ends_at,
        proration_amount_minor: 0,
    }
}

pub fn cancel_at_period_end_keeps_access(
    status: SubscriptionStatus,
    now: DateTime<Utc>,
    period: &SubscriptionPeriod,
) -> bool {
    status == SubscriptionStatus::Canceled && now < period.ends_at
}

fn days_in_month(value: DateTime<Utc>) -> i32 {
    let next_month = if value.month() == 12 {
        chrono::NaiveDate::from_ymd_opt(value.year() + 1, 1, 1)
    } else {
        chrono::NaiveDate::from_ymd_opt(value.year(), value.month() + 1, 1)
    }
    .expect("valid next month");
    let current_month =
        chrono::NaiveDate::from_ymd_opt(value.year(), value.month(), 1).expect("valid month");
    (next_month - current_month).num_days() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upgrade_proration_uses_remaining_period() {
        assert_eq!(prorated_upgrade_amount(1_000, 2_000, 15, 30), 500);
    }

    #[test]
    fn downgrade_is_effective_at_period_end() {
        let period = SubscriptionPeriod {
            starts_at: DateTime::from_timestamp(1_735_689_600, 0).unwrap(),
            ends_at: DateTime::from_timestamp(1_738_368_000, 0).unwrap(),
        };
        let change = period_end_downgrade_change(&period);
        assert_eq!(change.effective_at, period.ends_at);
        assert_eq!(change.proration_amount_minor, 0);
    }

    #[test]
    fn cancel_at_period_end_preserves_access_until_end() {
        let period = SubscriptionPeriod {
            starts_at: DateTime::from_timestamp(1_735_689_600, 0).unwrap(),
            ends_at: DateTime::from_timestamp(1_738_368_000, 0).unwrap(),
        };
        let now = DateTime::from_timestamp(1_736_000_000, 0).unwrap();
        assert!(cancel_at_period_end_keeps_access(
            SubscriptionStatus::Canceled,
            now,
            &period
        ));
    }

    #[test]
    fn monthly_annual_trial_and_grace_periods_are_calculated() {
        let start = DateTime::from_timestamp(1_735_689_600, 0).unwrap();
        assert_eq!(
            next_period(start, SubscriptionInterval::Monthly).ends_at,
            start + Duration::days(31)
        );
        assert_eq!(
            next_period(start, SubscriptionInterval::Annual).ends_at,
            start + Duration::days(365)
        );
        assert_eq!(trial_period(start, 14).ends_at, start + Duration::days(14));
        let monthly = next_period(start, SubscriptionInterval::Monthly);
        assert_eq!(
            grace_period(&monthly, 7).ends_at,
            monthly.ends_at + Duration::days(7)
        );
    }

    #[test]
    fn plan_change_decision_upgrades_now_and_downgrades_at_period_end() {
        let period = SubscriptionPeriod {
            starts_at: DateTime::from_timestamp(1_735_689_600, 0).unwrap(),
            ends_at: DateTime::from_timestamp(1_738_368_000, 0).unwrap(),
        };
        let now = DateTime::from_timestamp(1_737_028_800, 0).unwrap();

        let upgrade = decide_plan_change(
            PlanChangeRequest {
                old_amount_minor: 1_000,
                new_amount_minor: 2_000,
                now,
                admin_override: false,
            },
            &period,
        );
        let downgrade = decide_plan_change(
            PlanChangeRequest {
                old_amount_minor: 2_000,
                new_amount_minor: 1_000,
                now,
                admin_override: false,
            },
            &period,
        );

        assert_eq!(upgrade.change_type, "upgrade");
        assert_eq!(upgrade.effective_at, now);
        assert!(upgrade.proration_amount_minor > 0);
        assert_eq!(downgrade.change_type, "downgrade");
        assert_eq!(downgrade.effective_at, period.ends_at);
    }

    #[test]
    fn admin_override_can_apply_downgrade_immediately_without_negative_proration() {
        let period = SubscriptionPeriod {
            starts_at: DateTime::from_timestamp(1_735_689_600, 0).unwrap(),
            ends_at: DateTime::from_timestamp(1_738_368_000, 0).unwrap(),
        };
        let now = DateTime::from_timestamp(1_736_000_000, 0).unwrap();
        let change = decide_plan_change(
            PlanChangeRequest {
                old_amount_minor: 2_000,
                new_amount_minor: 1_000,
                now,
                admin_override: true,
            },
            &period,
        );

        assert_eq!(change.change_type, "downgrade_override");
        assert_eq!(change.effective_at, now);
        assert_eq!(change.proration_amount_minor, 0);
    }
}
