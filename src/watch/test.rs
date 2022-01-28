use super::{Registry, Subscription};
use tokio::sync::mpsc;

#[tokio::test]
async fn flow() {
    let ids = vec![1, 2, 3, 4];
    let registry = Registry::from_ids(&ids);
    let subscribe_tx = registry.spawn();

    let h2 = tokio::spawn(subscribe_then_wait_until_changed(2, subscribe_tx.clone()));

    let _ = tokio::join!(h2);
}

async fn subscribe_then_wait_until_changed(id: u8, subscribe_tx: mpsc::Sender<Subscription>) {
    let mut watch_rx = Registry::subscribe(id, subscribe_tx).await;
    watch_rx.changed().await.unwrap();
    let changed = *watch_rx.borrow();
    dbg!(changed);
}
