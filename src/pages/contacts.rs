use crate::{model, Result};
use askama::Template;
use axum::{
    extract::{Query, State},
    response::{IntoResponse, Redirect, Response},
    Form,
};
use axum_extra::routing::{TypedPath, WithQueryParams};
use axum_flash::{Flash, IncomingFlashes};
use axum_htmx::HxTrigger;
use serde::{Deserialize, Serialize};

pub mod item;
pub mod new;
pub mod shared;

#[derive(TypedPath)]
#[typed_path("/contacts")]
pub struct Path;

#[derive(Template)]
#[template(path = "contacts.html")]
pub struct Page {
    pub layout: shared::Layout,
    pub search_term: Option<String>,
    pub page: u64,
    pub contacts: Vec<shared::Contact>,
}

#[derive(Template)]
#[template(path = "contacts.html", block = "table")]
pub struct Table {
    pub contacts: Vec<shared::Contact>,
    pub search_term: Option<String>,
    pub page: u64,
}

#[derive(Deserialize, Serialize)]
pub struct Params {
    q: Option<String>,
    page: Option<u64>,
}

impl Path {
    pub fn with_params(
        self,
        q: &Option<String>,
        page: Option<&u64>,
    ) -> WithQueryParams<Self, Params> {
        self.with_query_params(Params {
            q: q.clone(),
            page: page.copied(),
        })
    }
}

pub async fn get(
    _: Path,
    flashes: IncomingFlashes,
    HxTrigger(hx_trigger): HxTrigger,
    State(contacts): State<model::Contacts>,
    Query(query): Query<Params>,
) -> Result<Response> {
    let page = query.page.unwrap_or(1);
    let contacts = match query.q {
        Some(ref q) => {
            let result = contacts.get_filtered_page(q, page).await?;
            result
                .into_iter()
                .map(|res| shared::Contact {
                    id: res.id,
                    first: res.first,
                    last: res.last,
                    phone: res.phone,
                    email: res.email,
                    errors: shared::ContactFieldErrors::default(),
                })
                .collect()
        }
        None => {
            let result = contacts.get_page(page).await?;
            result
                .into_iter()
                .map(|res| shared::Contact {
                    id: res.id,
                    first: res.first,
                    last: res.last,
                    phone: res.phone,
                    email: res.email,
                    errors: shared::ContactFieldErrors::default(),
                })
                .collect()
        }
    };
    match hx_trigger {
        Some(trigger) if trigger == "search" => Ok(Table {
            contacts,
            page,
            search_term: query.q,
        }
        .into_response()),
        _ => Ok((
            flashes.clone(),
            Page {
                layout: shared::Layout {
                    flashes: Some(flashes),
                },
                contacts,
                page,
                search_term: query.q,
            },
        )
            .into_response()),
    }
}

pub async fn post(
    _: Path,
    State(db): State<model::Contacts>,
    flash: Flash,
    Form(contact): Form<model::ContactCandidate>,
) -> Result<Response> {
    let result = db.create(&contact).await;
    match result {
        Ok(id) => Ok((
            flash.success("Contact created"),
            Redirect::to(&item::Path { id }.to_string()),
        )
            .into_response()),
        Err(model::Error::DuplicateEmail) => Ok((
            flash.error("Contact could not be saved"),
            new::Tmpl {
                layout: shared::Layout { flashes: None },
                contact: shared::Contact {
                    id: 0,
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
            eprintln!("{:?}", err);
            Err(err)?
        }
    }
}
