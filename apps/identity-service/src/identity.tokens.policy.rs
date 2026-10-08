use std::collections::BTreeSet;

use crate::tokens_error::TokenError;

pub const ACCOUNT_AUDIENCE: &str = "nvbes-account-service";
pub const BILLING_AUDIENCE: &str = "nvbes-billing-service";
pub const USERINFO_AUDIENCE: &str = "nvbes-identity-userinfo";

const ACCOUNT_SCOPES: [&str; 4] = [
    "account:read",
    "account:write",
    "account:export",
    "account:close",
];
const BILLING_SCOPES: [&str; 2] = ["billing:read", "billing:checkout"];
const ALLOWED_AMR: [&str; 3] = ["pwd", "totp", "webauthn"];

pub fn validate_scopes(audience: &str, scope: &str) -> Result<(), TokenError> {
    if scope.len() > 1024
        || scope.split(' ').any(|item| {
            item.is_empty()
                || item.bytes().any(|byte| {
                    !byte.is_ascii_alphanumeric() && !matches!(byte, b':' | b'-' | b'_' | b'.')
                })
        })
    {
        return Err(TokenError::InvalidPolicy);
    }
    let requested = scope.split_whitespace().collect::<BTreeSet<_>>();
    if requested.is_empty() || requested.len() != scope.split_whitespace().count() {
        return Err(TokenError::InvalidPolicy);
    }
    let allowed = allowed_scopes(audience)?;
    if requested.iter().any(|item| !allowed.contains(item))
        || (audience == USERINFO_AUDIENCE && !requested.contains("openid"))
    {
        return Err(TokenError::InvalidPolicy);
    }
    Ok(())
}

pub(crate) fn access_scope(audience: &str, requested: &str) -> Result<String, TokenError> {
    let scope = requested
        .split(' ')
        .filter(|s| {
            *s != "offline_access"
                && (audience == USERINFO_AUDIENCE || !matches!(*s, "openid" | "profile" | "email"))
        })
        .collect::<Vec<_>>()
        .join(" ");
    validate_scopes(audience, &scope)?;
    Ok(scope)
}

pub fn validate_amr(amr: &[String]) -> Result<(), TokenError> {
    if amr.is_empty()
        || amr
            .iter()
            .any(|method| !ALLOWED_AMR.contains(&method.as_str()))
    {
        return Err(TokenError::InvalidAuthentication);
    }
    if amr.iter().collect::<BTreeSet<_>>().len() != amr.len()
        || !amr.iter().any(|m| matches!(m.as_str(), "pwd" | "webauthn"))
    {
        return Err(TokenError::InvalidAuthentication);
    }
    Ok(())
}

fn allowed_scopes(audience: &str) -> Result<&'static [&'static str], TokenError> {
    match audience {
        ACCOUNT_AUDIENCE => Ok(&ACCOUNT_SCOPES),
        BILLING_AUDIENCE => Ok(&BILLING_SCOPES),
        USERINFO_AUDIENCE => Ok(&["openid", "profile", "email"]),
        _ => Err(TokenError::InvalidPolicy),
    }
}

#[cfg(test)]
mod tests {
    use super::{ACCOUNT_AUDIENCE, BILLING_AUDIENCE, validate_amr, validate_scopes};

    #[test]
    fn userinfo_requires_openid_and_never_accepts_api_permissions() {
        use super::{USERINFO_AUDIENCE, access_scope};
        assert_eq!(
            access_scope(USERINFO_AUDIENCE, "openid email offline_access").unwrap(),
            "openid email"
        );
        for scope in [
            "email",
            "openid account:read",
            "openid billing:read",
            "openid openid",
        ] {
            assert!(access_scope(USERINFO_AUDIENCE, scope).is_err());
        }
        assert_eq!(
            access_scope(ACCOUNT_AUDIENCE, "openid email account:read offline_access").unwrap(),
            "account:read"
        );
        assert!(access_scope(ACCOUNT_AUDIENCE, "openid email").is_err());
    }

    #[test]
    fn scopes_are_bound_to_their_audience() {
        assert!(validate_scopes(ACCOUNT_AUDIENCE, "account:read account:close").is_ok());
        assert!(validate_scopes(BILLING_AUDIENCE, "billing:checkout").is_ok());
        assert!(validate_scopes(ACCOUNT_AUDIENCE, "billing:checkout").is_err());
        assert!(validate_scopes(ACCOUNT_AUDIENCE, "account:read account:read").is_err());
        assert!(validate_scopes(ACCOUNT_AUDIENCE, "account:read  account:close").is_err());
    }

    #[test]
    fn amr_accepts_passwordless_but_not_a_standalone_totp() {
        assert!(validate_amr(&["pwd".into(), "totp".into()]).is_ok());
        assert!(validate_amr(&["webauthn".into()]).is_ok());
        assert!(validate_amr(&["totp".into()]).is_err());
        assert!(validate_amr(&["pwd".into(), "sms".into()]).is_err());
    }
}
