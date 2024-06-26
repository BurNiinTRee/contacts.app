use axum::{
    extract::{Query, State},
    response::IntoResponse,
};
use axum_extra::routing::TypedPath;
use serde::Deserialize;

use crate::{
    model::{self, ContactId},
    Result,
};

#[derive(TypedPath, Deserialize)]
#[typed_path("/contacts/:id/email")]
pub struct Path {
    pub id: ContactId,
}

impl Path {
    pub fn new(id: &ContactId) -> Self {
        Self { id: *id }
    }
}

#[derive(Deserialize)]
pub struct Params {
    email: String,
}

pub async fn get(
    Path { id }: Path,
    Query(Params { email }): Query<Params>,
    State(contacts): State<model::Contacts>,
) -> Result<impl IntoResponse> {
    let result = contacts.get_by_email(&email).await?;
    match result {
        Some(res) if res.id != id => Ok("Email already exists"),
        _ => Ok(""),
    }
}
