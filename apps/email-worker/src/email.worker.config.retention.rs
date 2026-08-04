#[derive(Debug, Clone)]
pub struct RetentionConfig {
    pub payload_days: i32,
    pub ledger_days: i32,
}

pub fn parse(payload_days: &str, ledger_days: &str) -> anyhow::Result<RetentionConfig> {
    let payload_days = days("NVBES_EMAIL_PAYLOAD_RETENTION_DAYS", payload_days)?;
    let ledger_days = days("NVBES_EMAIL_LEDGER_RETENTION_DAYS", ledger_days)?;
    if payload_days > 90 {
        anyhow::bail!("NVBES_EMAIL_PAYLOAD_RETENTION_DAYS must not exceed 90 days");
    }
    if ledger_days < payload_days || ledger_days > 3_650 {
        anyhow::bail!(
            "NVBES_EMAIL_LEDGER_RETENTION_DAYS must be between payload retention and 3650 days"
        );
    }
    Ok(RetentionConfig {
        payload_days,
        ledger_days,
    })
}

fn days(variable: &str, value: &str) -> anyhow::Result<i32> {
    let value = value
        .parse::<i32>()
        .map_err(|_| anyhow::anyhow!("{variable} must be a positive number of days"))?;
    if value <= 0 {
        anyhow::bail!("{variable} must be a positive number of days");
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn retention_keeps_secret_payloads_shorter_than_the_ledger() {
        assert!(parse("30", "400").is_ok());
        assert!(parse("91", "400").is_err());
        assert!(parse("30", "29").is_err());
    }
}
