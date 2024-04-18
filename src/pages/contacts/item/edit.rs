use crate::{
    model::{self, ContactId},
    pages::contacts::shared::{Contact, Layout},
    Result,
};
use askama::Template;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use axum_extra::routing::TypedPath;
use axum_flash::IncomingFlashes;
use serde::Deserialize;

#[derive(Deserialize, TypedPath)]
#[typed_path("/contacts/:id/edit")]
pub struct Path {
    pub id: ContactId,
}

impl Path {
    pub fn new(id: &ContactId) -> Self {
        Self { id: *id }
    }
}

#[derive(Template)]
#[template(path = "edit-contact.html")]
pub struct Tmpl {
    pub layout: Layout,
    pub contact: Contact,
}

pub async fn get(
    Path { id }: Path,
    flashes: IncomingFlashes,
    State(contacts): State<model::Contacts>,
) -> Result<Response> {
    let contact = contacts.get_by_id(id).await?;

    let Some(contact) = contact else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };

    let contact = super::super::shared::Contact {
        id,
        first: contact.first,
        last: contact.last,
        phone: contact.phone,
        email: contact.email,
        errors: Default::default(),
    };
    Ok((
        flashes.clone(),
        Tmpl {
            layout: Layout {
                flashes: Some(flashes),
            },
            contact,
        },
    )
        .into_response())
}
