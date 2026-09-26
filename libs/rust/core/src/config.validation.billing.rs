use serde_json::Value;

use super::AppConfig;

pub(crate) fn validate_billing_fraud_policy_overrides(config: &AppConfig) -> Result<(), String> {
    let Some(raw) = config
        .billing_fraud_policy_overrides_json
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    else {
        return Ok(());
    };
    let value: Value = serde_json::from_str(raw).map_err(|_| {
        "NVBES_BILLING_FRAUD_POLICY_OVERRIDES_JSON must be a valid JSON array".to_string()
    })?;
    let Value::Array(overrides) = value else {
        return Err("NVBES_BILLING_FRAUD_POLICY_OVERRIDES_JSON must be a JSON array".to_string());
    };
    for override_value in overrides {
        validate_override(config, &override_value)?;
    }
    Ok(())
}

fn validate_override(config: &AppConfig, value: &Value) -> Result<(), String> {
    let Value::Object(object) = value else {
        return Err("Billing fraud policy overrides must be JSON objects".to_string());
    };
    for field in ["provider", "plan_code", "country"] {
        if let Some(value) = object.get(field)
            && !value.is_string()
        {
            return Err(format!(
                "Billing fraud policy override field {field} must be a string"
            ));
        }
    }
    let min_amount = optional_i64(object.get("min_amount_minor"), "min_amount_minor")?;
    let max_amount = optional_i64(object.get("max_amount_minor"), "max_amount_minor")?;
    if matches!((min_amount, max_amount), (Some(min), Some(max)) if min > max) {
        return Err(
            "Billing fraud policy override min_amount_minor must be <= max_amount_minor"
                .to_string(),
        );
    }
    let step_up = optional_u8(object.get("step_up_threshold"), "step_up_threshold")?
        .unwrap_or(config.billing_fraud_step_up_threshold);
    let manual_review = optional_u8(
        object.get("manual_review_threshold"),
        "manual_review_threshold",
    )?
    .unwrap_or(config.billing_fraud_manual_review_threshold);
    let block = optional_u8(object.get("block_threshold"), "block_threshold")?
        .unwrap_or(config.billing_fraud_block_threshold);
    if step_up > manual_review || manual_review > block || block > 100 {
        return Err("Billing fraud policy override thresholds must satisfy step_up <= manual_review <= block <= 100".to_string());
    }
    Ok(())
}

fn optional_i64(value: Option<&Value>, field: &str) -> Result<Option<i64>, String> {
    value
        .map(|value| {
            value.as_i64().ok_or_else(|| {
                format!("Billing fraud policy override field {field} must be an integer")
            })
        })
        .transpose()
}

fn optional_u8(value: Option<&Value>, field: &str) -> Result<Option<u8>, String> {
    value
        .map(|value| {
            let raw = value.as_u64().ok_or_else(|| {
                format!("Billing fraud policy override field {field} must be an integer")
            })?;
            u8::try_from(raw).map_err(|_| {
                format!("Billing fraud policy override field {field} must be between 0 and 100")
            })
        })
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::validate_billing_fraud_policy_overrides;
    use crate::config::AppConfig;

    fn base_config(overrides: Option<&str>) -> AppConfig {
        AppConfig {
            billing_fraud_step_up_threshold: 60,
            billing_fraud_manual_review_threshold: 75,
            billing_fraud_block_threshold: 90,
            billing_fraud_policy_overrides_json: overrides.map(str::to_string),
            ..AppConfig::default()
        }
    }

    #[test]
    fn validates_ordered_fraud_policy_overrides() {
        let config = base_config(Some(
            r#"[{"provider":"stripe","plan_code":"team","manual_review_threshold":70}]"#,
        ));

        validate_billing_fraud_policy_overrides(&config)
            .expect("valid override should be accepted");
    }

    #[test]
    fn rejects_invalid_effective_fraud_policy_overrides() {
        let config = base_config(Some(
            r#"[{"step_up_threshold":95,"manual_review_threshold":75}]"#,
        ));

        let error = validate_billing_fraud_policy_overrides(&config)
            .expect_err("invalid override should be rejected");

        assert!(error.contains("step_up <= manual_review"));
    }

    #[test]
    fn empty_or_blank_overrides_are_accepted() {
        validate_billing_fraud_policy_overrides(&base_config(None)).expect("none");
        validate_billing_fraud_policy_overrides(&base_config(Some("   "))).expect("blank");
        validate_billing_fraud_policy_overrides(&base_config(Some("[]"))).expect("empty array");
    }

    #[test]
    fn fraud_policy_override_table_covers_reject_branches() {
        let cases = [
            ("not-json", "must be a valid JSON array"),
            (r#"{"provider":"stripe"}"#, "must be a JSON array"),
            (r#"[1]"#, "must be JSON objects"),
            (
                r#"[{"provider":123}]"#,
                "override field provider must be a string",
            ),
            (
                r#"[{"plan_code":false}]"#,
                "override field plan_code must be a string",
            ),
            (
                r#"[{"country":1}]"#,
                "override field country must be a string",
            ),
            (
                r#"[{"min_amount_minor":"x"}]"#,
                "min_amount_minor must be an integer",
            ),
            (
                r#"[{"max_amount_minor":1.5}]"#,
                "max_amount_minor must be an integer",
            ),
            (
                r#"[{"min_amount_minor":20,"max_amount_minor":10}]"#,
                "min_amount_minor must be <= max_amount_minor",
            ),
            (
                r#"[{"step_up_threshold":"x"}]"#,
                "step_up_threshold must be an integer",
            ),
            (
                r#"[{"block_threshold":256}]"#,
                "block_threshold must be between 0 and 100",
            ),
            (
                r#"[{"block_threshold":101}]"#,
                "step_up <= manual_review <= block <= 100",
            ),
            (
                r#"[{"block_threshold":95,"manual_review_threshold":99}]"#,
                "step_up <= manual_review <= block <= 100",
            ),
        ];
        for (raw, expected) in cases {
            let error =
                validate_billing_fraud_policy_overrides(&base_config(Some(raw))).expect_err(raw);
            assert!(
                error.contains(expected),
                "raw={raw} error={error} expected={expected}"
            );
        }
    }

    #[test]
    fn accepts_override_with_amount_bounds_and_defaults() {
        let config = base_config(Some(
            r#"[{"provider":"mollie","country":"FR","min_amount_minor":100,"max_amount_minor":5000}]"#,
        ));
        validate_billing_fraud_policy_overrides(&config).expect("amount bounds");
    }
}
