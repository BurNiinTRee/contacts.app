use std::time::Duration;

use serde::Deserialize;
use sqlx::SqlitePool;
use thiserror::Error;

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
        let wait = tokio::time::sleep(Duration::from_secs(2));
        let result = sqlx::query!("SELECT COUNT(*) as count FROM Contacts").fetch_one(&self.db);
        let result = tokio::join!(result, wait);
        Ok(result.0?.count as u64)
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
    #[error("contact with this email already exists")]
    DuplicateEmail,
    #[error("unknown database error")]
    DatabaseError(#[from] sqlx::Error),
}
