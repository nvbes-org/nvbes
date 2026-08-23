pub(crate) fn is_system_client(client_id: &str) -> bool {
    matches!(
        client_id,
        "account-web"
            | "console-web"
            | "cloud-web"
            | "cloud-worker"
            | "developer-service"
            | "billing-service"
            | "gateway-cloud"
            | "backoffice-service"
    )
}

pub(crate) fn validate_system_client_audience(
    client_id: &str,
    audience: &str,
) -> Result<(), crate::http::error::AppError> {
    let expected = match client_id {
        "account-web" => Some("nvbes-account-service"),
        "cloud-web" | "cloud-worker" | "gateway-cloud" => Some("nvbes-cloud-service"),
        "console-web" | "developer-service" => Some("nvbes-developer-service"),
        "billing-service" => Some("nvbes-billing-service"),
        "backoffice-service" => Some("nvbes-backoffice-service"),
        _ => None,
    };

    if expected.is_none() || expected == Some(audience) {
        return Ok(());
    }

    Err(crate::http::error::AppError::forbidden(
        "client_audience_not_allowed",
        "The requested audience is not approved for this OAuth client.",
    ))
}

#[cfg(test)]
mod tests {
    use super::{is_system_client, validate_system_client_audience};

    #[test]
    fn first_party_clients_are_system_clients() {
        assert!(is_system_client("account-web"));
        assert!(is_system_client("console-web"));
        assert!(is_system_client("developer-service"));
        assert!(is_system_client("billing-service"));
        assert!(is_system_client("gateway-cloud"));
        assert!(is_system_client("backoffice-service"));
    }

    #[test]
    fn arbitrary_client_is_not_system_client() {
        assert!(!is_system_client("customer-app"));
    }

    #[test]
    fn account_web_is_bound_to_the_account_resource_server() {
        validate_system_client_audience("account-web", "nvbes-account-service")
            .expect("Account audience should be allowed");

        let error = validate_system_client_audience("account-web", "nvbes-cloud-service")
            .expect_err("Account client must not mint Cloud tokens");
        assert_eq!(error.code, "client_audience_not_allowed");
    }
}
