use super::*;
use crate::cockpit_model::ServiceId;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

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

    let client = ContextClient::new(
        vec![ServiceEndpoint {
            service: ServiceId::Account,
            base_url: "https://account.example/".into(),
        }],
        true,
    )
    .expect("https origins allowed in production");
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
    assert!(
        ContextClient::new(
            vec![ServiceEndpoint {
                service: ServiceId::Identity,
                base_url: "https://identity.example/?probe=1".into(),
            }],
            false
        )
        .is_err()
    );
    assert!(
        ContextClient::new(
            vec![ServiceEndpoint {
                service: ServiceId::Account,
                base_url: "https://account.example/#fragment".into(),
            }],
            false
        )
        .is_err()
    );
    assert!(
        ContextClient::new(
            vec![ServiceEndpoint {
                service: ServiceId::Billing,
                base_url: "https://user@billing.example/".into(),
            }],
            false
        )
        .is_err()
    );
    assert!(
        ContextClient::new(
            vec![ServiceEndpoint {
                service: ServiceId::Email,
                base_url: "ftp://email.example/".into(),
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

fn serve_one_ready_response() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let mut buf = [0_u8; 1024];
        let _ = stream.read(&mut buf);
        let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n");
    });
    format!("http://{addr}/")
}

#[tokio::test]
async fn snapshot_marks_ready_and_unavailable_endpoints() {
    let ready_url = serve_one_ready_response();
    let client = ContextClient::new(
        vec![
            ServiceEndpoint {
                service: ServiceId::Identity,
                base_url: ready_url,
            },
            ServiceEndpoint {
                service: ServiceId::Account,
                base_url: "http://127.0.0.1:9/".into(),
            },
        ],
        false,
    )
    .expect("local endpoints");

    let snapshot = client.snapshot().await;
    let identity = snapshot
        .iter()
        .find(|item| item.service == ServiceId::Identity)
        .expect("identity");
    let account = snapshot
        .iter()
        .find(|item| item.service == ServiceId::Account)
        .expect("account");
    assert_eq!(identity.status, "ready");
    assert_eq!(account.status, "unavailable");
    assert!(
        snapshot
            .iter()
            .filter(|item| item.service != ServiceId::Identity
                && item.service != ServiceId::Account)
            .all(|item| item.status == "not_configured")
    );
}
