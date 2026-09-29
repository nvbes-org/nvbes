use super::*;
use crate::config::DispatchQueueConfig;

#[tokio::test]
async fn in_memory_queue_enqueues_and_receives() {
    let mut runtime = runtime(&DispatchMode::InMemory);
    let rx = runtime.local_receiver.as_mut().expect("must have receiver");
    runtime
        .publisher
        .enqueue("evt_123")
        .await
        .expect("enqueue should succeed");
    let received = tokio::time::timeout(std::time::Duration::from_millis(200), rx.recv())
        .await
        .expect("enqueue must deliver without hanging")
        .expect("must receive event");
    assert_eq!(received, "evt_123");
}

#[tokio::test]
async fn in_memory_queue_fails_when_receiver_is_dropped() {
    let runtime = runtime(&DispatchMode::InMemory);
    drop(runtime.local_receiver);
    let err = runtime
        .publisher
        .enqueue("evt_closed")
        .await
        .expect_err("closed queue must fail");
    assert!(err.to_string().contains("closed"));
}

#[test]
fn scaleway_runtime_builds_without_local_receiver() {
    let mode = DispatchMode::Scaleway(DispatchQueueConfig {
        endpoint: "https://sqs.mnq.fr-par.scaleway.com".into(),
        queue_url: "https://sqs.mnq.fr-par.scaleway.com/123/billing-dispatch".into(),
        region: "fr-par".into(),
        access_key: "SCWXXXXXXXXXXXXXXXXX".into(),
        secret_key: "11111111-2222-3333-4444-555555555555".into(),
    });
    let runtime = runtime(&mode);
    assert!(runtime.local_receiver.is_none());
}
