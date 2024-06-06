use futures::stream::{Stream, StreamExt};
use serde::Deserialize;
use sqlx::PgPool;

use crate::model::Result;

mod id;
pub use id::ContactId;

use super::Error;

pub struct Contact {
    pub id: ContactId,
    pub first: String,
    pub last: String,
    pub phone: String,
    pub email: String,
}

#[derive(Debug, Clone)]
pub struct Contacts {
    db: PgPool,
}

impl Contacts {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    pub async fn count(&self) -> Result<u64> {
        // let (count, _) = tokio::join!(
        //     sqlx::query_scalar!(r#"SELECT COUNT(*) as "count!" FROM Contacts"#).fetch_one(&self.db),
        //     tokio::time::sleep(std::time::Duration::from_secs(2))
        // );
        let count = sqlx::query_scalar!(r#"SELECT COUNT(*) as "count!" FROM Contacts"#)
            .fetch_one(&self.db)
            .await;
        Ok(count? as u64)
    }

    pub async fn get_by_id(&self, id: ContactId) -> Result<Option<Contact>> {
        let contact = sqlx::query_as!(
            Contact,
            "SELECT id, first, last, phone, email FROM Contacts WHERE id = $1",
            id as ContactId
        )
        .fetch_optional(&self.db)
        .await?;
        Ok(contact)
    }

    pub async fn get_by_email(&self, email: &str) -> Result<Option<Contact>> {
        let contact = sqlx::query_as!(
            Contact,
            "SELECT id, first, last, phone, email FROM Contacts WHERE email = $1",
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
                    WHERE first ILIKE CONCAT('%', $1::TEXT, '%')
                       OR last ILIKE CONCAT('%', $1::TEXT, '%')
                    ORDER BY first, last, email ASC LIMIT $2 OFFSET $3
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
                ORDER BY first, last, email ASC LIMIT $1 OFFSET $2
            ",
            pagesize,
            offset
        )
        .fetch_all(&self.db)
        .await?;
        Ok(contacts)
    }

    pub async fn delete_by_id(&self, id: ContactId) -> Result<()> {
        sqlx::query!("DELETE FROM Contacts WHERE id = $1", id as ContactId)
            .execute(&self.db)
            .await?;
        Ok(())
    }

    pub async fn create(&self, new_contact: &ContactCandidate) -> Result<ContactId> {
        let result = sqlx::query_scalar!(
        r#"INSERT INTO Contacts (first, last, phone, email) VALUES ($1, $2, $3, $4) RETURNING id as "id: ContactId""#,
        new_contact.first,
        new_contact.last,
        new_contact.phone,
        new_contact.email,
    )
    .fetch_one(&self.db)
    .await;
        match result {
            Ok(result) => Ok(result),
            Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
                Err(Error::DuplicateEmail)
            }
            Err(err) => Err(err)?,
        }
    }

    pub async fn update_by_id(
        &self,
        id: ContactId,
        new_contact: &ContactCandidate,
    ) -> Result<ContactId> {
        let result = sqlx::query_scalar!(
    r#"UPDATE Contacts SET first = $1, last = $2, phone = $3, email = $4 WHERE id = $5 RETURNING id as "id: ContactId""#,
    new_contact.first,
    new_contact.last,
    new_contact.phone,
    new_contact.email,
    id as ContactId
)
    .fetch_one(&self.db)
    .await;
        match result {
            Ok(result) => Ok(result),
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
