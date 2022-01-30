mod some_op;
#[cfg(test)]
mod test;

use std::collections::HashMap;
use std::{fmt::Debug, hash::Hash};
use tokio::sync::{mpsc, oneshot, watch};

pub struct Registry<T> {
    table: HashMap<T, (watch::Sender<bool>, watch::Receiver<bool>)>,
}

#[derive(Debug)]
pub enum Request<T> {
    Subscription(T, oneshot::Sender<watch::Receiver<bool>>),
    Trigger(T),
}

impl<T> Registry<T>
where
    T: Debug + Copy + Eq + Hash + Send + Sync + 'static,
{
    #[must_use]
    /// Make a registry with a table with the given IDs.
    /// The watch channels for all entries are initialized with `true`.
    pub fn from_ids(ids: &[T]) -> Self {
        let table = ids.iter().map(|id| (*id, watch::channel(true))).collect();
        Self { table }
    }

    #[must_use]
    /// Spawn the registry, yielding a request sender handle.
    pub fn spawn(self) -> mpsc::Sender<Request<T>> {
        let (request_tx, request_rx) = mpsc::channel(64);
        tokio::spawn(self.run(request_rx));
        request_tx
    }

    /// Run the registry loop, handling requests and completions of operations.
    async fn run(self, mut request: mpsc::Receiver<Request<T>>) {
        let (done_tx, mut done_rx) = mpsc::channel(64);
        loop {
            tokio::select! {
                Some(request) = request.recv() => {
                    self.handle_request(request, done_tx.clone()).await;
                },
                Some(id) = done_rx.recv() => {
                    self.complete(id).await;
                },
                else => return,
            }
        }
    }

    /// Get a watch channel for the given id.
    pub async fn subscribe(id: T, request: &mpsc::Sender<Request<T>>) -> watch::Receiver<bool> {
        let (reply_tx, reply_rx) = oneshot::channel();
        request
            .send(Request::Subscription(id, reply_tx))
            .await
            .unwrap();
        reply_rx.await.unwrap()
    }

    async fn handle_request(&self, request: Request<T>, done_tx: mpsc::Sender<T>) {
        match request {
            Request::Subscription(id, reply) => {
                // clone watch before (re-)triggering operation.
                // This avoids the operation completing before the watch is cloned.
                let watch = self.table.get(&id).unwrap().1.clone();
                self.trigger(id, done_tx.clone()).await;
                reply.send(watch).unwrap();
            }
            Request::Trigger(id) => {
                self.trigger(id, done_tx.clone()).await;
            }
        }
    }

    /// Start operation, unless already started.
    /// When finished, operation shall send `id` on `on_finish`.
    async fn trigger(&self, id: T, on_finish: mpsc::Sender<T>) {
        // If state is true, reset it to false and spawn operation.
        // Else, do nothing.
        let state = *self.table.get(&id).unwrap().1.borrow();
        if !state {
            return;
        }
        self.table.get(&id).unwrap().0.send(false).unwrap();
        tokio::spawn(some_op::do_thing(id, on_finish));
    }

    async fn complete(&self, id: T) {
        let state = *self.table.get(&id).unwrap().1.borrow();
        if !state {
            self.table.get(&id).unwrap().0.send(true).unwrap();
        }
    }
}
