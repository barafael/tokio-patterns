#[cfg(test)]
mod test;

use rand::Rng;
use std::{collections::HashMap, time::Duration};
use tokio::sync::{mpsc, oneshot, watch};

type Table<T> = HashMap<T, (watch::Sender<bool>, watch::Receiver<bool>)>;
pub struct Registry<T> {
    table: Table<T>,
}

#[derive(Debug)]
pub enum Request<T> {
    Subscribe(T, oneshot::Sender<watch::Receiver<bool>>),
    Trigger(T),
}

impl<T> Registry<T>
where
    T: std::fmt::Debug + Default + Copy + Eq + std::hash::Hash + Send + Sync + 'static,
{
    #[must_use]
    pub fn from_ids(ids: &[T]) -> Self {
        let table = ids.iter().map(|id| (*id, watch::channel(true))).collect();
        Self { table }
    }

    /// Get a watch channel for the given id.
    pub async fn subscribe(id: T, request: &mpsc::Sender<Request<T>>) -> watch::Receiver<bool> {
        let (reply_tx, reply_rx) = oneshot::channel();
        request
            .send(Request::Subscribe(id, reply_tx))
            .await
            .unwrap();
        reply_rx.await.unwrap()
    }

    #[must_use]
    pub fn spawn(self) -> mpsc::Sender<Request<T>> {
        let (request_tx, request_rx) = mpsc::channel(64);
        tokio::spawn(self.run(request_rx));
        request_tx
    }

    async fn run(self, mut request: mpsc::Receiver<Request<T>>) {
        let (done_tx, mut done_rx) = mpsc::channel(1);
        loop {
            tokio::select! {
                Some(request) = request.recv() => {
                    self.handle_request(request, done_tx.clone()).await;
                },
                Some(id) = done_rx.recv() => {
                    self.mark_done(id).await;
                },
                else => return,
            }
        }
    }

    async fn handle_request(&self, request: Request<T>, done_tx: mpsc::Sender<T>) {
        match request {
            Request::Subscribe(id, reply) => {
                let watch = self.table.get(&id).unwrap().1.clone();
                self.start_operation(id, done_tx.clone()).await;
                reply.send(watch).unwrap();
            }
            Request::Trigger(id) => {
                self.start_operation(id, done_tx.clone()).await;
            }
        }
    }

    /// Start operation, unless already started.
    /// When finished, operation will send `id` on `on_stop`.
    async fn start_operation(&self, id: T, on_stop: mpsc::Sender<T>) {
        // Reset state to false if it is true
        // Then spawn operation
        let state = *self.table.get(&id).unwrap().1.borrow();
        if state {
            self.table.get(&id).unwrap().0.send(false).unwrap();
            tokio::spawn(some_operation(id, on_stop));
        }
    }

    async fn mark_done(&self, id: T) {
        let state = *self.table.get(&id).unwrap().1.borrow();
        if !state {
            self.table.get(&id).unwrap().0.send(true).unwrap();
        }
    }
}

pub async fn some_operation<T>(id: T, on_stop: mpsc::Sender<T>) {
    tokio::time::sleep(random_delay()).await;
    let _ = on_stop.send(id).await;
}

fn random_delay() -> Duration {
    let mut rng = rand::thread_rng();
    Duration::from_millis(rng.gen_range(250..1000))
}
