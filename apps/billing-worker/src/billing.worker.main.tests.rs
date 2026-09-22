use tokio::sync::watch;

use super::server_shutdown;

#[tokio::test]
async fn server_shutdown_completes_when_watch_channel_closes() {
    let (sender, receiver) = watch::channel(false);
    drop(sender);
    server_shutdown(receiver).await;
}

#[tokio::test]
async fn server_shutdown_completes_when_shutdown_flag_is_set() {
    let (sender, receiver) = watch::channel(false);
    let waiting = tokio::spawn(server_shutdown(receiver));
    sender.send(true).unwrap();
    waiting.await.unwrap();
}
