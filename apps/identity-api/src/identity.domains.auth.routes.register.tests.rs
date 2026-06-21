use super::{RegisterRequest, register_input_from_request};

#[test]
fn register_input_from_request_maps_http_payload_to_onboarding_input() {
    let birthdate = chrono::NaiveDate::from_ymd_opt(1990, 5, 4).expect("valid date");
    let input = register_input_from_request(
        RegisterRequest {
            email: " User@Example.COM ".to_string(),
            password: "Secret123!".to_string(),
            firstname: Some("Ada".to_string()),
            lastname: Some("Lovelace".to_string()),
            username: "ada".to_string(),
            birthdate: Some("1990-05-04".to_string()),
            region: Some("FR".to_string()),
            workspace_name: "Ada Workspace".to_string(),
            pow_nonce: Some("nonce".to_string()),
            pow_solution: Some("solution".to_string()),
        },
        "FR".to_string(),
        "eu".to_string(),
        Some(birthdate),
        Some("203.0.113.10".to_string()),
        Some("nvbes-test".to_string()),
    );

    assert_eq!(input.email, " User@Example.COM ");
    assert_eq!(input.password, "Secret123!");
    assert_eq!(input.firstname.as_deref(), Some("Ada"));
    assert_eq!(input.lastname.as_deref(), Some("Lovelace"));
    assert_eq!(input.username, "ada");
    assert_eq!(input.birthdate, Some(birthdate));
    assert_eq!(input.region.as_deref(), Some("FR"));
    assert_eq!(input.data_region.as_deref(), Some("eu"));
    assert_eq!(input.workspace_name, "Ada Workspace");
    assert_eq!(input.ip.as_deref(), Some("203.0.113.10"));
    assert_eq!(input.user_agent.as_deref(), Some("nvbes-test"));
}
