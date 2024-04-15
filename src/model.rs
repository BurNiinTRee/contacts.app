use std::{ops::Deref, sync::Arc, time::Duration};

use futures::{stream::StreamExt, Stream};
use serde::Deserialize;
use sqlx::SqlitePool;
use thiserror::Error;
use tokio::{fs::File, io::AsyncWriteExt, sync::watch, task::AbortHandle};
use tracing::{info, info_span, instrument, Instrument as _};

type Result<T, E = self::Error> = std::result::Result<T, E>;

pub struct Contact {
    pub id: i64,
    pub first: String,
    pub last: String,
    pub phone: String,
    pub email: String,
}

#[derive(Debug, Clone)]
pub struct Contacts {
    db: SqlitePool,
}

impl Contacts {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }

    pub async fn count(&self) -> Result<u64> {
        // let (result, _) = tokio::join!(
        //     sqlx::query!("SELECT COUNT(*) as count FROM Contacts").fetch_one(&self.db),
        //     tokio::time::sleep(Duration::from_secs(2))
        // );
        let result = sqlx::query!("SELECT COUNT(*) as count FROM Contacts")
            .fetch_one(&self.db)
            .await;
        Ok(result?.count as u64)
    }

    pub async fn get_by_id(&self, id: i64) -> Result<Option<Contact>> {
        let contact = sqlx::query_as!(
            Contact,
            "SELECT id, first, last, phone, email FROM Contacts WHERE id = ?",
            id
        )
        .fetch_optional(&self.db)
        .await?;
        Ok(contact)
    }

    pub async fn get_by_email(&self, email: &str) -> Result<Option<Contact>> {
        let contact = sqlx::query_as!(
            Contact,
            "SELECT id, first, last, phone, email FROM Contacts WHERE email = ?",
            email
        )
        .fetch_optional(&self.db)
        .await?;
        Ok(contact)
    }

    pub fn get_all(&self) -> impl Stream<Item = Result<Contact>> + '_ {
        sqlx::query_as!(
            Contact,
            "SELECT id, first, last, phone, email FROM Contacts"
        )
        .fetch(&self.db)
        .map(|res| Ok(res?))
    }

    pub async fn get_filtered_page(&self, search_term: &str, page: u64) -> Result<Vec<Contact>> {
        let pagesize = 10;
        let offset = (page as i64 - 1) * pagesize;
        let contacts = sqlx::query_as!(
            Contact,
            r"SELECT id, first, last, phone, email FROM Contacts 
                    WHERE first LIKE CONCAT('%', ?1, '%')
                       OR last LIKE CONCAT('%', ?1, '%')
                    ORDER BY first, last, email ASC LIMIT ?2 OFFSET ?3
                ",
            search_term,
            pagesize,
            offset
        )
        .fetch_all(&self.db)
        .await?;
        Ok(contacts)
    }

    pub async fn get_page(&self, page: u64) -> Result<Vec<Contact>> {
        let pagesize = 10;
        let offset = (page as i64 - 1) * pagesize;
        let contacts = sqlx::query_as!(
            Contact,
            r"SELECT id, first, last, phone, email FROM Contacts 
                ORDER BY first, last, email ASC LIMIT ?1 OFFSET ?2
            ",
            pagesize,
            offset
        )
        .fetch_all(&self.db)
        .await?;
        Ok(contacts)
    }

    pub async fn delete_by_id(&self, id: i64) -> Result<()> {
        sqlx::query!("DELETE FROM Contacts WHERE id = ?", id)
            .execute(&self.db)
            .await?;
        Ok(())
    }

    pub async fn create(&self, new_contact: &ContactCandidate) -> Result<i64> {
        let result = sqlx::query!(
            "INSERT INTO Contacts (first, last, phone, email) VALUES (?, ?, ?, ?) RETURNING id",
            new_contact.first,
            new_contact.last,
            new_contact.phone,
            new_contact.email,
        )
        .fetch_one(&self.db)
        .await;
        match result {
            Ok(result) => Ok(result.id),
            Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
                Err(Error::DuplicateEmail)
            }
            Err(err) => Err(err)?,
        }
    }

    pub async fn update_by_id(
        &self,
        id: i64,
        new_contact: &ContactCandidate,
    ) -> Result<Option<i64>> {
        let result = sqlx::query!(
        "UPDATE Contacts SET first = ?, last = ?, phone = ?, email = ? WHERE id = ? RETURNING id",
        new_contact.first,
        new_contact.last,
        new_contact.phone,
        new_contact.email,
        id
    )
        .fetch_one(&self.db)
        .await;
        match result {
            Ok(result) => Ok(result.id),
            Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
                Err(Error::DuplicateEmail)
            }
            Err(err) => Err(err)?,
        }
    }
}

#[derive(Deserialize)]
pub struct ContactCandidate {
    pub first: String,
    pub last: String,
    pub phone: String,
    pub email: String,
}

#[derive(Error, Debug)]
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

#[derive(Clone, Debug)]
pub struct Archiver {
    contacts: Contacts,
    _recv: watch::Receiver<ArchiverState>,
    state: watch::Sender<ArchiverState>,
}

#[derive(Debug)]
enum ArchiverState {
    Waiting,
    Running {
        progress: f32,
        abort_handle: AbortHandle,
    },
    Complete(Result<(), Arc<Error>>),
}

#[derive(Clone)]
pub enum ArchiverStatus {
    Waiting,
    Running(f32),
    Complete(Result<(), Arc<Error>>),
}

impl Archiver {
    pub fn new(contacts: Contacts) -> Self {
        let (state, _recv) = watch::channel(ArchiverState::Waiting);
        Self {
            contacts,
            state,
            _recv,
        }
    }

    pub async fn status(&self) -> ArchiverStatus {
        match self.state.borrow().deref() {
            ArchiverState::Waiting => ArchiverStatus::Waiting,
            ArchiverState::Running { progress, .. } => ArchiverStatus::Running(*progress),
            ArchiverState::Complete(result) => ArchiverStatus::Complete(result.clone()),
        }
    }

    async fn work(state: watch::Sender<ArchiverState>, contacts: Contacts, mut out_file: File) {
        info!("spawned worker");
        let state2 = state.clone();
        let res: Result<()> = async move {
            let count = contacts.count().await?;
            let mut contacts = contacts.get_all().enumerate();
            while let Some((
                written,
                Ok(Contact {
                    id,
                    first,
                    last,
                    phone,
                    email,
                }),
            )) = contacts.next().await
            {
                // tokio::time::sleep(Duration::from_millis(1)).await;
                out_file
                    .write_all(format!("{id},{first},{last},{phone},{email}\n").as_bytes())
                    .await?;
                if written & 0x3FFF == 0 {
                    let progress = written as f32 / count as f32;
                    state2.send_modify(|s| {
                        if let ArchiverState::Running {
                            progress: ref mut p,
                            ..
                        } = s
                        {
                            *p = progress;
                        }
                    });
                    info!(progress, "Updating progress");
                }
            }
            Ok(out_file.shutdown().await?)
        }
        .await;
        state
            .send(ArchiverState::Complete(res.map_err(Into::into)))
            .unwrap()
    }

    #[instrument(skip(self))]
    pub async fn run(&self) -> Result<()> {
        if let ArchiverState::Running { .. } = self.state.borrow().deref() {
            return Err(Error::ArchiverRunning);
        }
        let mut out_file = File::create("run/export.csv").await?;
        out_file
            .write_all(b"id,firstname,lastname,phone,email\n")
            .await?;

        let handle = tokio::spawn(
            Self::work(self.state.clone(), self.contacts.clone(), out_file)
                .instrument(info_span!("worker thread")),
        );

        self.state
            .send(ArchiverState::Running {
                progress: 0.0,
                abort_handle: handle.abort_handle(),
            })
            .unwrap();

        Ok(())
    }

    pub async fn reset(&self) {
        if let ArchiverState::Running { abort_handle, .. } = self.state.borrow().deref() {
            abort_handle.abort();
        };
        self.state.send(ArchiverState::Waiting).unwrap();
    }
}
