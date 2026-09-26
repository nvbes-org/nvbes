use super::*;
use crate::config::DispatchQueueConfig;
use uuid::Uuid;

#[tokio::test]
async fn in_memory_queue_enqueues_and_receives() {
    let mut runtime = runtime(&DispatchMode::InMemory);
    let rx = runtime.local_receiver.as_mut().expect("receiver");
    let id = Uuid::new_v4();
    runtime.publisher.enqueue(id).await.expect("enqueue");
    assert_eq!(rx.recv().await, Some(id));
}

#[tokio::test]
async fn in_memory_queue_fails_when_closed() {
    let runtime = runtime(&DispatchMode::InMemory);
    drop(runtime.local_receiver);
    let err = runtime
        .publisher
        .enqueue(Uuid::new_v4())
        .await
        .expect_err("closed");
    assert!(err.to_string().contains("closed"));
}

#[test]
fn scaleway_runtime_builds_without_local_receiver() {
    let mode = DispatchMode::Scaleway(DispatchQueueConfig {
        endpoint: "https://sqs.mnq.fr-par.scaleway.com".into(),
        queue_url: "https://sqs.mnq.fr-par.scaleway.com/123/email-dispatch".into(),
        region: "fr-par".into(),
        access_key: "SCWXXXXXXXXXXXXXXXXX".into(),
        secret_key: "11111111-2222-3333-4444-555555555555".into(),
    });
    let runtime = runtime(&mode);
    assert!(runtime.local_receiver.is_none());
}
