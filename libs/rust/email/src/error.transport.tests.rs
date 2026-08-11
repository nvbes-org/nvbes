use std::time::Duration;

use tokio::{io::AsyncWriteExt, net::TcpListener};

use super::{EmailError, EmailFailureClass};

async fn status_error(status: u16) -> reqwest::Error {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        stream
            .write_all(format!("HTTP/1.1 {status} Test\r\nContent-Length: 0\r\n\r\n").as_bytes())
            .await
            .unwrap();
    });
    reqwest::get(format!("http://{address}"))
        .await
        .unwrap()
        .error_for_status()
        .unwrap_err()
}

#[tokio::test]
async fn http_failures_distinguish_permanent_transient_and_ambiguous_delivery() {
    let permanent = EmailError::Http(status_error(400).await);
    assert_eq!(permanent.failure_class(), EmailFailureClass::Permanent);

    let transient_status = EmailError::Http(status_error(503).await);
    assert_eq!(
        transient_status.failure_class(),
        EmailFailureClass::Transient
    );

    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let _connection = listener.accept().await.unwrap();
        tokio::time::sleep(Duration::from_millis(200)).await;
    });
    let timeout = reqwest::Client::builder()
        .timeout(Duration::from_millis(20))
        .build()
        .unwrap()
        .get(format!("http://{address}"))
        .send()
        .await
        .unwrap_err();
    assert_eq!(
        EmailError::Http(timeout).failure_class(),
        EmailFailureClass::Ambiguous
    );
    server.abort();

    let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    let transport = reqwest::get(endpoint).await.unwrap_err();
    assert_eq!(
        EmailError::Http(transport).failure_class(),
        EmailFailureClass::Transient
    );
}

#[test]
fn safe_metadata_covers_every_local_error_variant() {
    let serialization = serde_json::from_str::<serde_json::Value>("{").unwrap_err();
    let address = "not an address".parse::<lettre::Address>().unwrap_err();
    let build = lettre::Message::builder().body(String::new()).unwrap_err();
    let cases = [
        (
            EmailError::Serialization(serialization),
            "email_serialization",
            "Email provider payload could not be serialized",
        ),
        (
            EmailError::Config("secret detail".into()),
            "email_configuration",
            "Email delivery configuration is invalid",
        ),
        (
            EmailError::Address(address),
            "email_address",
            "Email address is invalid",
        ),
        (
            EmailError::Build(build),
            "email_message_build",
            "Email message could not be built",
        ),
    ];
    for (error, code, summary) in cases {
        assert_eq!(error.failure_class(), EmailFailureClass::Permanent);
        assert_eq!(error.safe_code(), code);
        assert_eq!(error.safe_summary(), summary);
        assert!(!error.is_retryable());
    }
}
