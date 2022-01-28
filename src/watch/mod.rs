#[cfg(test)]
mod test;

use rand::Rng;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::Duration,
};
use tokio::sync::{mpsc, oneshot, watch};

type Table = HashMap<u8, (watch::Sender<bool>, watch::Receiver<bool>)>;
pub struct Registry {
    table: Arc<RwLock<Table>>,
}

pub type Subscription = (u8, oneshot::Sender<watch::Receiver<bool>>);

impl Registry {
    #[must_use]
    pub fn from_ids(ids: &[u8]) -> Self {
        let table = ids.iter().map(|id| (*id, watch::channel(true))).collect();
        Self {
            table: Arc::new(RwLock::new(table)),
        }
    }

    /// Get a watch channel for the given id.
    pub async fn subscribe(
        id: u8,
        subscribe_tx: mpsc::Sender<Subscription>,
    ) -> watch::Receiver<bool> {
        let (reply_tx, reply_rx) = oneshot::channel();
        subscribe_tx.send((id, reply_tx)).await.unwrap();
        reply_rx.await.unwrap()
    }

    #[must_use = "The sender handle must be used to subscribe to IDs"]
    pub fn spawn(self) -> mpsc::Sender<Subscription> {
        let (subscribe_tx, subscribe_rx) = mpsc::channel(64);
        tokio::spawn(self.run(subscribe_rx));
        subscribe_tx
    }

    async fn run(self, mut subscribe: mpsc::Receiver<Subscription>) {
        let (done_tx, mut done_rx) = mpsc::channel(1);
        loop {
            tokio::select! {
                Some((id, reply)) = subscribe.recv() => {
                    let watch = self.start_operation(id, done_tx.clone()).await;
                    reply.send(watch).unwrap();
                },
                Some(id) = done_rx.recv() => {
                    self.mark_done(id).await;
                },
                else => return,
            }
        }
    }

    async fn start_operation(&self, id: u8, on_stop: mpsc::Sender<u8>) -> watch::Receiver<bool> {
        // Reset state to false if it is true
        // Then spawn operation
        if *self.table.read().unwrap().get(&id).unwrap().1.borrow() {
            self.table
                .read()
                .unwrap()
                .get(&id)
                .unwrap()
                .0
                .send(false)
                .unwrap();
            let watch = self.table.read().unwrap().get(&id).unwrap().1.clone();
            tokio::spawn(some_operation(id, on_stop));
            return watch;
        }
        self.table.read().unwrap().get(&id).unwrap().1.clone()
    }

    async fn mark_done(&self, id: u8) {
        let state = *self.table.read().unwrap().get(&id).unwrap().1.borrow();
        if !state {
            self.table
                .read()
                .unwrap()
                .get(&id)
                .unwrap()
                .0
                .send(true)
                .unwrap();
        }
    }
}

pub async fn some_operation(id: u8, on_stop: mpsc::Sender<u8>) {
    tokio::time::sleep(random_delay()).await;
    let _ = on_stop.send(id).await;
}

fn random_delay() -> Duration {
    let mut rng = rand::thread_rng();
    Duration::from_millis(rng.gen_range(250..1000))
}
