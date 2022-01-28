use super::{Registry, Request};
use tokio::sync::mpsc;

#[tokio::test]
async fn flow() {
    tokio::time::pause();
    let ids = vec![1, 2, 3, 4];
    let registry = Registry::from_ids(&ids);
    let request_tx = registry.spawn();

    tokio::join!(tokio::spawn(subscribe_then_wait_until_changed_4x(
        2,
        request_tx.clone()
    )))
    .0
    .unwrap();
}

async fn subscribe_then_wait_until_changed_4x(id: u8, request_tx: mpsc::Sender<Request<u8>>) {
    let mut watch_rx = Registry::subscribe(id, &request_tx).await;

    watch_rx.changed().await.unwrap();
    let new_val = *watch_rx.borrow();
    assert_eq!(new_val, false);

    watch_rx.changed().await.unwrap();
    let new_val = *watch_rx.borrow();
    assert_eq!(new_val, true);

    request_tx.send(Request::Trigger(id)).await.unwrap();

    watch_rx.changed().await.unwrap();
    let new_val = *watch_rx.borrow();
    assert_eq!(new_val, false);

    watch_rx.changed().await.unwrap();
    let new_val = *watch_rx.borrow();
    assert_eq!(new_val, true);
}
