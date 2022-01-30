mod error;
mod some_op;
#[cfg(test)]
mod test;

use self::error::Error;
use std::collections::HashMap;
use std::{fmt::Debug, hash::Hash};
use tokio::sync::{mpsc, oneshot, watch};

pub struct Registry<T> {
    table: HashMap<T, (watch::Sender<bool>, watch::Receiver<bool>)>,
}

#[derive(Debug)]
pub enum Request<T: Debug> {
    Subscription(T, oneshot::Sender<Result<watch::Receiver<bool>, Error<T>>>),
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
                    self.handle_request(request, done_tx.clone()).await
                },
                Some(id) = done_rx.recv() => {
                    self.announce_completion(id).await
                },
                else => return,
            }
        }
    }

    /// Handle request such as subscription and trigger.
    /// If the request contains an ID which is not in the registry:
    /// * In case of subscription, send `Error::IdNotFound(id)` on the reply channel.
    /// * In case of trigger, discard.
    async fn handle_request(&self, request: Request<T>, on_finish: mpsc::Sender<T>) {
        match request {
            Request::Subscription(id, reply) => {
                // clone watch before (re-)triggering operation.
                // This avoids the operation completing before the watch is cloned.
                let watch = if let Some(watch) = self.table.get(&id) {
                    watch.1.clone()
                } else {
                    // discard return as the number of watchers doesn't matter here.
                    let _ = reply.send(Err(Error::InvalidId(id)));
                    return;
                };
                self.trigger_operation(id, on_finish.clone()).await;
                let _ = reply.send(Ok(watch));
            }
            Request::Trigger(id) => self.trigger_operation(id, on_finish.clone()).await,
        }
    }

    /// Get a watch channel for the given id.
    pub async fn subscribe(
        id: T,
        request: &mpsc::Sender<Request<T>>,
    ) -> Result<watch::Receiver<bool>, Error<T>> {
        let (reply_tx, reply_rx) = oneshot::channel();
        request
            .send(Request::Subscription(id, reply_tx))
            .await
            .map_err(Error::Shutdown)?;
        // unwrap is fine here as long as we do not drop the sender handle.
        reply_rx.await.unwrap()
    }

    /// Trigger the operation with given ID.
    pub async fn trigger(id: T, request: &mpsc::Sender<Request<T>>) -> Result<(), Error<T>> {
        request
            .send(Request::Trigger(id))
            .await
            .map_err(Error::Shutdown)
    }

    /// Starts operation, unless already started.
    /// When finished, operation shall send `id` on `on_finish`.
    async fn trigger_operation(&self, id: T, on_finish: mpsc::Sender<T>) {
        // If state is true, reset it to false and spawn operation.
        // Else, do nothing.
        let state = if let Some(state) = self.table.get(&id) {
            *state.1.borrow()
        } else {
            println!("Got announcement for completion of operation for non-existent ID");
            return;
        };
        if !state {
            return;
        }
        // announce start of operation.
        // discard `Result` as the number of watchers doesn't matter here.
        let _ = self.table.get(&id).unwrap().0.send(false);
        // start operation.
        tokio::spawn(some_op::do_thing(id, on_finish));
    }

    /// Announces the completion of an ongoing operation.
    async fn announce_completion(&self, id: T) {
        if let Some(value) = self.table.get(&id) {
            let state = *value.1.borrow();
            if !state {
                // announce completion of operation.
                // discard `Result` as the number of watchers doesn't matter here.
                let _ = value.0.send(true);
            }
        } else {
            println!("Got announcement for completion of operation for non-existent ID");
            return;
        };
    }
}
