use nvbes_billing::payments::{PaymentStatus, can_fallback_to_another_provider};

pub fn payment_can_fallback(status: PaymentStatus) -> bool {
    can_fallback_to_another_provider(status)
}
