use crate::registry::error::Error;

use super::{Registry, Request};
use tokio::sync::mpsc;

#[tokio::test]
async fn subscribe_then_watch_multiple() {
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
    let mut watch_rx = Registry::subscribe(id, &request_tx).await.unwrap();

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

#[tokio::test]
async fn subscription_fails_for_invalid_id() {
    let ids = vec![1, 2, 3, 4];
    let registry = Registry::from_ids(&ids);
    let request_tx = registry.spawn();

    assert!(matches!(
        Registry::subscribe(5, &request_tx).await,
        Err(Error::InvalidId(5))
    ));
}

#[tokio::test]
async fn swallows_trigger_for_invalid_id() {
    let ids = vec![1, 2, 3, 4];
    let registry = Registry::from_ids(&ids);
    let request_tx = registry.spawn();

    assert!(Registry::trigger(5, &request_tx).await.is_ok());
}
