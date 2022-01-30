use std::fmt::Debug;
use thiserror::Error;
use tokio::sync::mpsc::error::SendError;

use super::Request;

#[derive(Debug, Error)]
pub enum Error<T: Debug> {
    #[error("The given ID {0:?} isn't in the registry")]
    InvalidId(T),

    #[error("The registry has shut down, unable send request {0:?}")]
    Shutdown(#[source] SendError<Request<T>>),
}
