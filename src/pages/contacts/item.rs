use crate::{
    model,
    pages::{self, contacts::shared},
    Result,
};
use anyhow::Context;
use askama::Template;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    Form,
};
use axum_extra::routing::TypedPath;
use axum_flash::{Flash, IncomingFlashes};
use axum_htmx::HxTrigger;
use serde::Deserialize;

pub mod edit;
pub mod email;

#[derive(TypedPath, Deserialize)]
#[typed_path("/contacts/:id")]
pub struct Path {
    pub id: i64,
}
impl Path {
    pub fn new(id: &i64) -> Self {
        Self { id: *id }
    }
}

#[derive(Template)]
#[template(path = "view-contact.html")]
pub struct Tmpl {
    pub layout: shared::Layout,
    pub contact: shared::Contact,
}

pub async fn get(
    Path { id }: Path,
    flashes: IncomingFlashes,
    State(db): State<model::Contacts>,
) -> Result<Response> {
    let contact = db.get_by_id(id).await?;
    let Some(contact) = contact else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };
    let contact = shared::Contact {
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
            layout: shared::Layout {
                flashes: Some(flashes),
            },
            contact,
        },
    )
        .into_response())
}

pub async fn put(
    Path { id }: Path,
    State(db): State<model::Contacts>,
    flash: Flash,
    Form(contact): Form<model::ContactCandidate>,
) -> Result<Response> {
    let result = db.update_by_id(id, &contact).await;
    match result {
        Ok(res) => Ok((
            flash.success("Contact updated"),
            Redirect::to(
                &pages::contacts::item::Path {
                    id: res.with_context(|| format!("No Contact with id: {}", id))?,
                }
                .to_string(),
            ),
        )
            .into_response()),
        Err(model::Error::DuplicateEmail) => Ok((
            flash.error("Contact could not be saved"),
            edit::Tmpl {
                layout: shared::Layout { flashes: None },
                contact: shared::Contact {
                    id,
                    first: contact.first,
                    last: contact.last,
                    phone: contact.phone,
                    email: contact.email,
                    errors: shared::ContactFieldErrors {
                        email: String::from("Email already exists"),
                        ..Default::default()
                    },
                },
            },
        )
            .into_response()),
        Err(err) => {
            eprintln!("{}", err);
            Err(err)?
        }
    }
}

pub async fn delete(
    Path { id }: Path,
    flash: Flash,
    HxTrigger(hx_trigger): HxTrigger,
    State(contacts): State<model::Contacts>,
) -> Result<Response> {
    contacts.delete_by_id(id).await?;
    match hx_trigger.as_deref() {
        Some("delete-btn") => Ok((
            flash.success("Contact deleted"),
            Redirect::to(&pages::contacts::Path.to_string()),
        )
            .into_response()),
        _ => Ok(().into_response()),
    }
}
