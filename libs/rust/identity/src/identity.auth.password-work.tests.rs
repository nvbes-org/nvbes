use super::password_work;
use std::{sync::mpsc, time::Duration};
use tokio::sync::{Semaphore, oneshot};

#[tokio::test]
async fn cancellation_does_not_release_running_password_work_budget() {
    static PERMITS: Semaphore = Semaphore::const_new(1);
    let (started, running) = oneshot::channel();
    let (release, gate) = mpsc::channel();
    let waiter = tokio::spawn(password_work(&PERMITS, move || {
        started.send(()).unwrap();
        gate.recv_timeout(Duration::from_secs(5)).unwrap();
    }));
    tokio::time::timeout(Duration::from_secs(5), running)
        .await
        .unwrap()
        .unwrap();
    waiter.abort();
    assert!(waiter.await.unwrap_err().is_cancelled());
    assert!(
        PERMITS.try_acquire().is_err(),
        "cancelled HTTP work still consumes Argon2 capacity"
    );
    release.send(()).unwrap();
    let permit = tokio::time::timeout(Duration::from_secs(5), PERMITS.acquire())
        .await
        .unwrap()
        .unwrap();
    drop(permit);
    assert_eq!(password_work(&PERMITS, || 42).await.unwrap(), 42);
}

#[tokio::test]
async fn panicking_password_work_releases_its_budget() {
    static PERMITS: Semaphore = Semaphore::const_new(1);
    assert!(
        password_work(&PERMITS, || panic!("injected worker failure"))
            .await
            .is_err()
    );
    assert!(PERMITS.try_acquire().is_ok());
}
