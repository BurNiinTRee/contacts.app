use std::{
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex},
    time::Duration,
};

use futures::{stream::StreamExt, Stream};
use serde::Deserialize;
use sqlx::SqlitePool;
use thiserror::Error;
use tokio::{fs::File, io::AsyncWriteExt, task::AbortHandle};

type Result<T, E = self::Error> = std::result::Result<T, E>;

pub struct Contact {
    pub id: i64,
    pub first: String,
    pub last: String,
    pub phone: String,
    pub email: String,
}

#[derive(Clone)]
pub struct Contacts {
    db: SqlitePool,
}

impl Contacts {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }

    pub async fn count(&self) -> Result<u64> {
        let (result, _) = tokio::join!(
            sqlx::query!("SELECT COUNT(*) as count FROM Contacts").fetch_one(&self.db),
            tokio::time::sleep(Duration::from_secs(2))
        );
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

#[derive(Clone)]
pub struct Archiver {
    contacts: Contacts,
    state: Arc<Mutex<ArchiverState>>,
}

enum ArchiverState {
    Waiting,
    Running {
        progress: Arc<Mutex<f32>>,
        abort_handle: AbortHandle,
    },
    Complete(Arc<Result<()>>),
}

pub enum ArchiverStatus {
    Waiting,
    Running(f32),
    Complete(Arc<Result<()>>),
}

impl Archiver {
    pub fn new(contacts: Contacts) -> Self {
        Self {
            contacts,
            state: Arc::new(Mutex::new(ArchiverState::Waiting)),
        }
    }

    pub async fn status(&self) -> ArchiverStatus {
        let mut state = self.state.lock().unwrap();
        match state.deref_mut() {
            ArchiverState::Waiting => ArchiverStatus::Waiting,
            ArchiverState::Running { progress, .. } => {
                ArchiverStatus::Running(*progress.lock().unwrap())
            }
            ArchiverState::Complete(result) => ArchiverStatus::Complete(result.clone()),
        }
    }

    pub async fn run(&self) -> Result<()> {
        if let ArchiverState::Running { .. } = self.state.lock().unwrap().deref() {
            return Err(Error::ArchiverRunning);
        }
        let contacts = self.contacts.clone();
        let mut out_file = File::create("run/export.csv").await?;
        out_file
            .write_all(b"id,firstname,lastname,phone,email\n")
            .await?;
        let progress = Arc::new(Mutex::new(0.0));
        let task_progress = progress.clone();
        let state = self.state.clone();

        let handle = tokio::spawn(async move {
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
                    tokio::time::sleep(Duration::from_nanos(1)).await;
                    out_file
                        .write_all(format!("{id},{first},{last},{phone},{email}\n").as_bytes())
                        .await?;
                    if written & 0xFF == 0 {
                        let progress = written as f32 / count as f32;
                        *task_progress.lock().unwrap() = progress;
                    }
                }
                Ok(out_file.shutdown().await?)
            }
            .await;
            *state.lock().unwrap() = ArchiverState::Complete(Arc::new(res))
        });

        *self.state.lock().unwrap() = ArchiverState::Running {
            progress,
            abort_handle: handle.abort_handle(),
        };

        Ok(())
    }

    pub async fn reset(&self) {
        let mut state = self.state.lock().unwrap();
        match state.deref() {
            ArchiverState::Waiting => {}
            ArchiverState::Running { abort_handle, .. } => {
                abort_handle.abort();
                *state = ArchiverState::Waiting;
            }
            ArchiverState::Complete(_) => *state = ArchiverState::Waiting,
        }
    }
}
