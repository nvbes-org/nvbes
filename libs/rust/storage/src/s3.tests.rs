use std::time::Duration;

use crate::{ObjectStore, S3ObjectStore};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[tokio::test]
async fn operational_requests_use_the_internal_endpoint() {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("test server should bind");
    let address = listener
        .local_addr()
        .expect("test server should have an address");
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("request should connect");
        let mut request = vec![0; 8 * 1024];
        let read = socket
            .read(&mut request)
            .await
            .expect("request should be readable");
        let request = String::from_utf8_lossy(&request[..read]);
        let request_line = request.lines().next().expect("request line should exist");
        assert!(
            request_line.starts_with(
                "DELETE /nvbes/account/profile-avatars/user/avatar?x-id=DeleteObject HTTP/1.1"
            ),
            "unexpected request line: {request_line}"
        );
        socket
            .write_all(b"HTTP/1.1 204 No Content\r\nConnection: close\r\n\r\n")
            .await
            .expect("response should be writable");
    });

    let store = S3ObjectStore::new(
        "nvbes".to_owned(),
        &format!("http://{address}"),
        Some("http://127.0.0.1:1"),
        "eu-west-1",
        "access-key",
        "secret-key",
    )
    .await;

    store
        .delete_objects(&["account/profile-avatars/user/avatar".to_owned()])
        .await
        .expect("delete should use the reachable internal endpoint");
    server.await.expect("test server should complete");
}

#[tokio::test]
async fn presigned_urls_use_the_public_endpoint() {
    let store = S3ObjectStore::new(
        "nvbes".to_owned(),
        "http://storage.internal:8333",
        Some("https://content.example.com"),
        "eu-west-1",
        "access-key",
        "secret-key",
    )
    .await;

    let download = store
        .presign_download(
            "account/profile-avatars/user/avatar",
            Duration::from_secs(60),
        )
        .await
        .expect("presigning should succeed without contacting storage");

    assert!(
        download
            .url
            .starts_with("https://content.example.com/nvbes/")
    );
}
