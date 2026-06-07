pub(crate) fn validate_positive_integer(name: &str, value: i64) -> Result<(), String> {
    if value <= 0 {
        return Err(format!("{name} must be greater than zero"));
    }

    Ok(())
}
