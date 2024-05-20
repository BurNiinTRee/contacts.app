pub use archiver::{Archiver, ArchiverStatus};
pub use contacts::{Contact, ContactCandidate, ContactId, Contacts};
use tokio::sync::{mpsc, oneshot};

mod archiver;
mod contacts;

type Result<T, E = self::Error> = std::result::Result<T, E>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("contact with this email already exists")]
    DuplicateEmail,
    #[error("unknown database error")]
    Database(#[from] sqlx::Error),
    #[error("unknown io error")]
    Io(#[from] std::io::Error),
    #[error("Couldn't get answer from archiver")]
    ArchiverNotReturn(#[from] oneshot::error::RecvError),
    #[error("Couldn't send command to archiver")]
    CommandSendFailed,
}

impl<T> From<mpsc::error::SendError<T>> for Error {
    fn from(_: mpsc::error::SendError<T>) -> Self {
        Self::CommandSendFailed
    }
}
