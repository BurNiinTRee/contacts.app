use axum_extra::routing::{TypedPath, WithQueryParams};
use serde::Deserialize;

#[derive(TypedPath)]
#[typed_path("/contacts")]
pub struct Contacts;

impl Contacts {
    pub fn with_params(
        self,
        q: &Option<String>,
        page: Option<&u64>,
    ) -> WithQueryParams<Self, super::ContactsQuery> {
        self.with_query_params(super::ContactsQuery {
            q: q.clone(),
            page: page.copied(),
        })
    }
}

#[derive(TypedPath, Deserialize)]
#[typed_path("/contacts/:id/edit")]
pub struct EditContact {
    pub id: i64,
}

impl EditContact {
    pub fn new(id: &i64) -> Self {
        Self { id: *id }
    }
}

#[derive(TypedPath, Deserialize)]
#[typed_path("/contacts/:id")]
pub struct Contact {
    pub id: i64,
}

impl Contact {
    pub fn new(id: &i64) -> Self {
        Self { id: *id }
    }
}

#[derive(TypedPath, Deserialize)]
#[typed_path("/contacts/new")]
pub struct NewContact;

#[derive(TypedPath, Deserialize)]
#[typed_path("/contacts/:id/email")]
pub struct ContactEmail {
    pub id: i64,
}

impl ContactEmail {
    pub fn new(id: &i64) -> Self {
        Self { id: *id }
    }
}
