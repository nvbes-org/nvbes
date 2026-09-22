use super::{CustomerType, SubscriptionStatus};

#[test]
fn subscription_status_from_string_maps_known_values() {
    assert_eq!(
        SubscriptionStatus::from("past_due".to_string()),
        SubscriptionStatus::PastDue
    );
    assert_eq!(
        SubscriptionStatus::from("unknown".to_string()),
        SubscriptionStatus::Active
    );
}

#[test]
fn customer_type_from_string_is_case_insensitive() {
    assert_eq!(CustomerType::from("B2C".to_string()), CustomerType::B2c);
    assert_eq!(CustomerType::from("other".to_string()), CustomerType::B2b);
}
