use anyhow::Context;
use axum_extra::routing::RouterExt;
use axum_flash::{Flash, IncomingFlashes};

use axum::{
    extract::{FromRef, Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    routing::get,
    Form,
};
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};
use tmpl::Layout;

mod paths;
mod tmpl;

type Result<T, E = ServerError> = std::result::Result<T, E>;

#[derive(Clone, FromRef)]
struct AppState {
    db: SqlitePool,
    contacts: model::Contacts,
    flash_config: axum_flash::Config,
}

mod model;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let db_options = std::env::var("DATABASE_URL")
        .as_deref()
        .unwrap_or("sqlite:data.db?mode=rwc")
        .parse::<SqliteConnectOptions>()?
        .extension_with_entrypoint(std::env::var("SQLITE_ICU_EXTENSION")?, "sqlite3_icu_init");

    let db = sqlx::SqlitePool::connect_with(db_options).await?;
    sqlx::migrate!().run(&db).await?;

    let contacts = model::Contacts::new(db.clone());

    let flash_config = axum_flash::Config::new(axum_flash::Key::generate());

    let app_state = AppState {
        db,
        contacts,
        flash_config,
    };

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(
        listener,
        axum::Router::new()
            .route(
                "/",
                get(move || async move { Redirect::to(&paths::Contacts.to_string()) }),
            )
            .typed_get(get_contacts)
            .typed_get(get_contacts_new)
            .typed_post(post_contacts_new)
            .typed_get(get_contacts_view)
            .typed_get(get_contacts_edit)
            .typed_post(post_contacts_edit)
            .typed_get(get_contact_email)
            .typed_delete(delete_contact)
            .nest_service("/assets", tower_http::services::ServeDir::new("assets"))
            .with_state(app_state),
    )
    .await?;

    Ok(())
}

#[derive(Deserialize, Serialize)]
struct ContactsQuery {
    q: Option<String>,
    page: Option<u64>,
}

async fn get_contacts(
    _: paths::Contacts,
    flashes: IncomingFlashes,
    State(contacts): State<model::Contacts>,
    Query(query): Query<ContactsQuery>,
) -> Result<impl IntoResponse> {
    let page = query.page.unwrap_or(1);
    let contacts = match query.q {
        Some(ref q) => {
            let result = contacts.get_filtered_page(q, page).await?;
            result
                .into_iter()
                .map(|res| tmpl::Contact {
                    id: res.id,
                    first: res.first,
                    last: res.last,
                    phone: res.phone,
                    email: res.email,
                    errors: tmpl::ContactFieldErrors::default(),
                })
                .collect()
        }
        None => {
            let result = contacts.get_page(page).await?;
            result
                .into_iter()
                .map(|res| tmpl::Contact {
                    id: res.id,
                    first: res.first,
                    last: res.last,
                    phone: res.phone,
                    email: res.email,
                    errors: tmpl::ContactFieldErrors::default(),
                })
                .collect()
        }
    };
    Ok((
        flashes.clone(),
        tmpl::Contacts {
            layout: Layout {
                flashes: Some(flashes),
            },
            contacts,
            page,
            search_term: query.q,
        },
    ))
}

async fn get_contacts_new(_: paths::NewContact, flashes: IncomingFlashes) -> impl IntoResponse {
    tmpl::NewContact {
        layout: Layout {
            flashes: Some(flashes),
        },
        contact: tmpl::Contact::default(),
    }
}

async fn post_contacts_new(
    _: paths::NewContact,
    State(db): State<model::Contacts>,
    flash: Flash,
    Form(contact): Form<model::ContactCandidate>,
) -> Result<Response> {
    let result = db.create(&contact).await;
    match result {
        Ok(id) => Ok((
            flash.success("Contact created"),
            Redirect::to(&paths::Contact { id }.to_string()),
        )
            .into_response()),
        Err(model::Error::DuplicateEmail) => Ok((
            flash.error("Contact could not be saved"),
            tmpl::NewContact {
                layout: Layout { flashes: None },
                contact: tmpl::Contact {
                    id: 0,
                    first: contact.first,
                    last: contact.last,
                    phone: contact.phone,
                    email: contact.email,
                    errors: tmpl::ContactFieldErrors {
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

async fn get_contacts_view(
    paths::Contact { id }: paths::Contact,
    flashes: IncomingFlashes,
    State(db): State<model::Contacts>,
) -> Result<Response> {
    let contact = db.get_by_id(id).await?;
    let Some(contact) = contact else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };
    let contact = tmpl::Contact {
        id,
        first: contact.first,
        last: contact.last,
        phone: contact.phone,
        email: contact.email,
        errors: Default::default(),
    };
    Ok((
        flashes.clone(),
        tmpl::ViewContact {
            layout: Layout {
                flashes: Some(flashes),
            },
            contact,
        },
    )
        .into_response())
}

async fn get_contacts_edit(
    paths::EditContact { id }: paths::EditContact,
    flashes: IncomingFlashes,
    State(contacts): State<model::Contacts>,
) -> Result<Response> {
    let contact = contacts.get_by_id(id).await?;

    let Some(contact) = contact else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };

    let contact = tmpl::Contact {
        id,
        first: contact.first,
        last: contact.last,
        phone: contact.phone,
        email: contact.email,
        errors: Default::default(),
    };
    Ok((
        flashes.clone(),
        tmpl::EditContact {
            layout: Layout {
                flashes: Some(flashes),
            },
            contact,
        },
    )
        .into_response())
}

async fn post_contacts_edit(
    paths::EditContact { id }: paths::EditContact,
    State(db): State<model::Contacts>,
    flash: Flash,
    Form(contact): Form<model::ContactCandidate>,
) -> Result<Response> {
    let result = db.update_by_id(id, &contact).await;
    match result {
        Ok(res) => Ok((
            flash.success("Contact updated"),
            Redirect::to(
                &paths::Contact {
                    id: res.with_context(|| format!("No Contact with id: {}", id))?,
                }
                .to_string(),
            ),
        )
            .into_response()),
        Err(model::Error::DuplicateEmail) => Ok((
            flash.error("Contact could not be saved"),
            tmpl::EditContact {
                layout: Layout { flashes: None },
                contact: tmpl::Contact {
                    id,
                    first: contact.first,
                    last: contact.last,
                    phone: contact.phone,
                    email: contact.email,
                    errors: tmpl::ContactFieldErrors {
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

#[derive(Deserialize)]
struct ContactEmailQuery {
    email: String,
}

async fn get_contact_email(
    paths::ContactEmail { id }: paths::ContactEmail,
    Query(ContactEmailQuery { email }): Query<ContactEmailQuery>,
    State(contacts): State<model::Contacts>,
) -> Result<impl IntoResponse> {
    let result = contacts.get_by_email(&email).await?;
    match result {
        Some(res) if res.id != id => Ok("Email already exists"),
        _ => Ok(""),
    }
}

async fn delete_contact(
    paths::Contact { id }: paths::Contact,
    flash: Flash,
    State(contacts): State<model::Contacts>,
) -> Result<(Flash, Redirect)> {
    contacts.delete_by_id(id).await?;
    Ok((
        flash.success("Contact deleted"),
        Redirect::to(&paths::Contacts.to_string()),
    ))
}

struct ServerError(anyhow::Error);

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, self.0.to_string()).into_response()
    }
}

impl<E> From<E> for ServerError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}
