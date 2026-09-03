use std::collections::BTreeSet;

pub const ACCOUNT_AUDIENCE: &str = "nvbes-account-service";
pub const BILLING_AUDIENCE: &str = "nvbes-billing-service";

const ACCOUNT_SCOPES: [&str; 4] = [
    "account:read",
    "account:write",
    "account:export",
    "account:close",
];
const BILLING_SCOPES: [&str; 2] = ["billing:read", "billing:checkout"];
const ALLOWED_AMR: [&str; 3] = ["pwd", "totp", "webauthn"];

pub fn validate_scopes(audience: &str, scope: &str) -> anyhow::Result<()> {
    if scope.len() > 1024
        || scope.split(' ').any(|item| {
            item.is_empty()
                || item.bytes().any(|byte| {
                    !byte.is_ascii_alphanumeric() && !matches!(byte, b':' | b'-' | b'_' | b'.')
                })
        })
    {
        anyhow::bail!("token scope syntax is invalid");
    }
    let requested = scope.split_whitespace().collect::<BTreeSet<_>>();
    if requested.is_empty() || requested.len() != scope.split_whitespace().count() {
        anyhow::bail!("token scopes must be non-empty and unique");
    }
    let allowed = allowed_scopes(audience)?;
    if requested.iter().any(|item| !allowed.contains(item)) {
        anyhow::bail!("token scope is not allowed for its audience");
    }
    Ok(())
}

pub fn validate_amr(amr: &[String]) -> anyhow::Result<()> {
    if amr.is_empty()
        || amr
            .iter()
            .any(|method| !ALLOWED_AMR.contains(&method.as_str()))
    {
        anyhow::bail!("token amr is invalid");
    }
    if amr.iter().collect::<BTreeSet<_>>().len() != amr.len() || !amr.iter().any(|m| m == "pwd") {
        anyhow::bail!("token amr must include one password authentication");
    }
    Ok(())
}

pub fn step_up_method(method: Option<String>) -> anyhow::Result<Option<String>> {
    match method.as_deref() {
        None => Ok(None),
        Some("totp" | "webauthn") => Ok(method),
        Some(_) => anyhow::bail!("stored step-up method is invalid"),
    }
}

fn allowed_scopes(audience: &str) -> anyhow::Result<&'static [&'static str]> {
    match audience {
        ACCOUNT_AUDIENCE => Ok(&ACCOUNT_SCOPES),
        BILLING_AUDIENCE => Ok(&BILLING_SCOPES),
        _ => anyhow::bail!("token audience has no scope policy"),
    }
}

#[cfg(test)]
mod tests {
    use super::{ACCOUNT_AUDIENCE, BILLING_AUDIENCE, validate_amr, validate_scopes};

    #[test]
    fn scopes_are_bound_to_their_audience() {
        assert!(validate_scopes(ACCOUNT_AUDIENCE, "account:read account:close").is_ok());
        assert!(validate_scopes(BILLING_AUDIENCE, "billing:checkout").is_ok());
        assert!(validate_scopes(ACCOUNT_AUDIENCE, "billing:checkout").is_err());
        assert!(validate_scopes(ACCOUNT_AUDIENCE, "account:read account:read").is_err());
        assert!(validate_scopes(ACCOUNT_AUDIENCE, "account:read  account:close").is_err());
    }

    #[test]
    fn amr_requires_password_and_known_step_up_methods() {
        assert!(validate_amr(&["pwd".into(), "totp".into()]).is_ok());
        assert!(validate_amr(&["webauthn".into()]).is_err());
        assert!(validate_amr(&["pwd".into(), "sms".into()]).is_err());
    }
}
