pub(crate) fn is_system_client(client_id: &str) -> bool {
    matches!(client_id, "developer-web" | "drive-web" | "drive-worker")
}

#[cfg(test)]
mod tests {
    use super::is_system_client;

    #[test]
    fn developer_portal_client_is_system_client() {
        assert!(is_system_client("developer-web"));
    }

    #[test]
    fn arbitrary_client_is_not_system_client() {
        assert!(!is_system_client("customer-app"));
    }
}
