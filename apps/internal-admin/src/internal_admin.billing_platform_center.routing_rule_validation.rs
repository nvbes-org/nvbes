use crate::billing_platform_center_routing_mutations::CreateRoutingRuleInput;
use crate::billing_platform_center_validation::validate_reason;
use crate::error::AppError;

pub(crate) fn validate_create_routing_rule(input: &CreateRoutingRuleInput) -> Result<(), AppError> {
    validate_provider(&input.provider)?;
    validate_reason(&input.reason)?;
    validate_code_len(input.country.as_deref(), 2, "invalid_routing_country")?;
    validate_code_len(input.currency.as_deref(), 3, "invalid_routing_currency")?;
    validate_amount_bounds(input.min_amount_minor, input.max_amount_minor)
}

pub(crate) fn normalize_country(country: Option<&str>) -> Option<String> {
    country.map(str::to_ascii_uppercase)
}

pub(crate) fn normalize_currency(currency: Option<&str>) -> Option<String> {
    currency.map(str::to_ascii_uppercase)
}

fn validate_provider(provider: &str) -> Result<(), AppError> {
    if matches!(provider, "stripe" | "mollie") {
        return Ok(());
    }
    Err(AppError::bad_request(
        "invalid_routing_provider",
        "Routing provider is not supported.",
    ))
}

fn validate_code_len(
    value: Option<&str>,
    expected: usize,
    code: &'static str,
) -> Result<(), AppError> {
    if value.is_none_or(|value| value.len() == expected) {
        return Ok(());
    }
    Err(AppError::bad_request(
        code,
        "Routing filter code is invalid.",
    ))
}

fn validate_amount_bounds(
    min_amount: Option<i64>,
    max_amount: Option<i64>,
) -> Result<(), AppError> {
    if min_amount.is_some_and(|amount| amount < 0) || max_amount.is_some_and(|amount| amount < 0) {
        return Err(AppError::bad_request(
            "invalid_routing_amount",
            "Routing amount bounds must be non-negative.",
        ));
    }
    if let (Some(min_amount), Some(max_amount)) = (min_amount, max_amount)
        && min_amount > max_amount
    {
        return Err(AppError::bad_request(
            "invalid_routing_amount_range",
            "Routing minimum amount cannot exceed maximum amount.",
        ));
    }
    Ok(())
}
