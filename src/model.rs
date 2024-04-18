pub use archiver::{Archiver, ArchiverStatus};
pub use contacts::{Contact, ContactCandidate, ContactId, Contacts};

mod archiver;
mod contacts;

type Result<T, E = self::Error> = std::result::Result<T, E>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("archiver is already running")]
    ArchiverRunning,
    #[error("contact with this email already exists")]
    DuplicateEmail,
    #[error("unknown database error")]
    Database(#[from] sqlx::Error),
    #[error("unknown io error")]
    Io(#[from] std::io::Error),
}
