use crate::{
    model::{self, ArchiverStatus},
    Result,
};
use anyhow::Context;
use askama::Template;
use axum::{
    extract::{Query, RawForm, State},
    response::{IntoResponse, Redirect, Response},
    Form,
};
use axum_extra::routing::{TypedPath, WithQueryParams};
use axum_flash::{Flash, IncomingFlashes};
use axum_htmx::HxTrigger;
use serde::{Deserialize, Serialize};

pub mod archive;
pub mod count;
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
    pub archiver_status: ArchiverStatus,
    pub page: u64,
    pub contacts: Vec<shared::Contact>,
}

#[derive(Template)]
#[template(path = "contacts.html", block = "rows")]
pub struct Rows {
    pub contacts: Vec<shared::Contact>,
    pub search_term: Option<String>,
    pub page: u64,
}
#[derive(Template)]
#[template(path = "contacts.html", block = "archive")]
pub struct Archive {
    archiver_status: ArchiverStatus,
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
    State(archiver): State<model::Archiver>,
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
        Some(trigger) if trigger == "search" => Ok(Rows {
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
                archiver_status: archiver.status().await,
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

#[derive(Debug, Deserialize, Serialize)]
pub struct DeleteForm {
    selected_contact_ids: Vec<i64>,
}

pub async fn delete(
    _: Path,
    flash: Flash,
    State(contacts): State<model::Contacts>,
    RawForm(form): RawForm,
) -> Result<Response> {
    for param in dbg!(form).split(|b| *b == b'&') {
        let mut things = param.splitn(2, |b| *b == b'=');
        let name = things.next().context("param had no name")?;
        let value: i64 = things
            .next()
            .context("param had no value")
            .and_then(|bytes| std::str::from_utf8(bytes).context("value was not utf-8"))
            .and_then(|s| s.parse().context("value was not a number"))?;
        if name == b"selected_contact_ids" {
            contacts.delete_by_id(value).await?;
        }
    }
    Ok((
        flash.success("Contact deleted"),
        Redirect::to(&Path.to_string()),
    )
        .into_response())
}
