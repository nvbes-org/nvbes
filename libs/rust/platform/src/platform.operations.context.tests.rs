use super::*;
use crate::cockpit_model::ServiceId;

#[test]
fn accepts_empty_endpoints_and_http_in_non_production() {
    let client = ContextClient::new(vec![], false).expect("empty endpoints");
    assert!(client.endpoints.is_empty());

    let client = ContextClient::new(
        vec![ServiceEndpoint {
            service: ServiceId::Identity,
            base_url: "http://127.0.0.1:8080/".into(),
        }],
        false,
    )
    .expect("http allowed outside production");
    assert_eq!(client.endpoints.len(), 1);
}

#[test]
fn rejects_non_origin_urls_and_http_in_production() {
    assert!(
        ContextClient::new(
            vec![ServiceEndpoint {
                service: ServiceId::Account,
                base_url: "https://account.example/path".into(),
            }],
            true
        )
        .is_err()
    );
    assert!(
        ContextClient::new(
            vec![ServiceEndpoint {
                service: ServiceId::Billing,
                base_url: "https://user:pass@billing.example/".into(),
            }],
            true
        )
        .is_err()
    );
    assert!(
        ContextClient::new(
            vec![ServiceEndpoint {
                service: ServiceId::Email,
                base_url: "http://email.example/".into(),
            }],
            true
        )
        .is_err()
    );
    assert!(
        ContextClient::new(
            vec![ServiceEndpoint {
                service: ServiceId::TrustRisk,
                base_url: "not a url".into(),
            }],
            false
        )
        .is_err()
    );
}

#[tokio::test]
async fn snapshot_marks_missing_services_not_configured() {
    let client = ContextClient::new(vec![], false).expect("empty endpoints");

    let snapshot = client.snapshot().await;
    assert_eq!(snapshot.len(), 5);
    assert!(
        snapshot
            .iter()
            .all(|service| service.status == "not_configured")
    );
    assert!(
        snapshot
            .iter()
            .all(|service| !service.subject_context.is_empty())
    );
}
