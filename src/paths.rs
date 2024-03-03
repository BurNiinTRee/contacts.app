use axum_extra::routing::TypedPath;
use serde::Deserialize;

#[derive(TypedPath)]
#[typed_path("/contacts")]
pub struct Contacts;

#[derive(TypedPath, Deserialize)]
#[typed_path("/contacts/:id/edit")]
pub struct EditContact {
    pub id: i64,
}

#[derive(TypedPath, Deserialize)]
#[typed_path("/contacts/:id/delete")]
pub struct DeleteContact {
    pub id: i64,
}

#[derive(TypedPath, Deserialize)]
#[typed_path("/contacts/:id")]
pub struct Contact {
    pub id: i64,
}

#[derive(TypedPath, Deserialize)]
#[typed_path("/contacts/new")]
pub struct NewContact;
