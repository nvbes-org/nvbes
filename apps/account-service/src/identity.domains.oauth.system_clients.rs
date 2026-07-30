pub(crate) fn is_system_client(client_id: &str) -> bool {
    matches!(
        client_id,
        "console-web"
            | "cloud-web"
            | "cloud-worker"
            | "developer-service"
            | "billing-service"
            | "gateway-cloud"
            | "backoffice-service"
    )
}

#[cfg(test)]
mod tests {
    use super::is_system_client;

    #[test]
    fn developer_portal_client_is_system_client() {
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
}
