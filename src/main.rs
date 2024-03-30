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
    flash_config: axum_flash::Config,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let db_options = std::env::var("DATABASE_URL")
        .as_deref()
        .unwrap_or("sqlite:data.db?mode=rwc")
        .parse::<SqliteConnectOptions>()?
        .extension_with_entrypoint(std::env::var("SQLITE_ICU_EXTENSION")?, "sqlite3_icu_init");

    let db = sqlx::SqlitePool::connect_with(db_options).await?;

    sqlx::migrate!().run(&db).await?;

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
            .with_state(AppState {
                db,
                flash_config: axum_flash::Config::new(axum_flash::Key::generate()),
            }),
    )
    .await?;

    Ok(())
}

#[derive(Deserialize, Serialize)]
struct ContactsQuery {
    q: Option<String>,
    page: Option<i64>,
}

async fn get_contacts(
    _: paths::Contacts,
    flashes: IncomingFlashes,
    State(db): State<SqlitePool>,
    Query(query): Query<ContactsQuery>,
) -> Result<impl IntoResponse> {
    let page = query.page.unwrap_or(1);
    let pagesize = 10;
    let offset = (page - 1) * pagesize;
    let contacts = match query.q {
        Some(ref q) => {
            let result = sqlx::query!(
                r"SELECT id, first, last, phone, email FROM Contacts 
                    WHERE first LIKE CONCAT('%', ?1, '%')
                       OR last LIKE CONCAT('%', ?1, '%')
                    ORDER BY first ASC LIMIT ?2 OFFSET ?3
                ",
                q,
                pagesize,
                offset
            )
            .fetch_all(&db)
            .await?;
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
            let result = sqlx::query!(
                "SELECT id, first, last, phone, email FROM Contacts ORDER BY first ASC LIMIT 10 OFFSET ?", offset
            )
            .fetch_all(&db)
            .await?;
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
    State(db): State<SqlitePool>,
    flash: Flash,
    Form(contact): Form<NewContact>,
) -> Result<Response> {
    let result = sqlx::query!(
        "INSERT INTO Contacts (first, last, phone, email) VALUES (?, ?, ?, ?) RETURNING id",
        contact.first,
        contact.last,
        contact.phone,
        contact.email,
    )
    .fetch_one(&db)
    .await;
    match result {
        Ok(result) => Ok((
            flash.success("Contact created"),
            Redirect::to(&paths::Contact { id: result.id }.to_string()),
        )
            .into_response()),
        Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => Ok((
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
            eprintln!("{}", err);
            Err(err)?
        }
    }
}

async fn get_contacts_view(
    paths::Contact { id }: paths::Contact,
    flashes: IncomingFlashes,
    State(db): State<SqlitePool>,
) -> Result<impl IntoResponse> {
    let contact = sqlx::query!(
        "SELECT id, first, last, phone, email FROM Contacts WHERE id = ?",
        id,
    )
    .fetch_one(&db)
    .await?;
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
    ))
}

async fn get_contacts_edit(
    paths::EditContact { id }: paths::EditContact,
    flashes: IncomingFlashes,
    State(db): State<SqlitePool>,
) -> Result<impl IntoResponse> {
    let contact = sqlx::query!(
        "SELECT id, first, last, phone, email FROM Contacts WHERE id = ?",
        id,
    )
    .fetch_one(&db)
    .await?;
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
    ))
}

async fn post_contacts_edit(
    paths::EditContact { id }: paths::EditContact,
    State(db): State<SqlitePool>,
    flash: Flash,
    Form(contact): Form<NewContact>,
) -> Result<Response> {
    let result = sqlx::query!(
        "UPDATE Contacts SET first = ?, last = ?, phone = ?, email = ? WHERE id = ? RETURNING id",
        contact.first,
        contact.last,
        contact.phone,
        contact.email,
        id
    )
    .fetch_one(&db)
    .await;
    match result {
        Ok(result) => Ok((
            flash.success("Contact updated"),
            Redirect::to(
                &paths::Contact {
                    id: result
                        .id
                        .with_context(|| format!("No Contact with id: {}", id))?,
                }
                .to_string(),
            ),
        )
            .into_response()),
        Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => Ok((
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
    State(db): State<SqlitePool>,
) -> Result<impl IntoResponse> {
    let result = sqlx::query!("SELECT id FROM Contacts WHERE email = ?", email)
        .fetch_optional(&db)
        .await?;
    match result {
        Some(res) if res.id != Some(id) => Ok("Email already exists"),
        _ => Ok(""),
    }
}

async fn delete_contact(
    paths::Contact { id }: paths::Contact,
    flash: Flash,
    State(db): State<SqlitePool>,
) -> Result<(Flash, Redirect)> {
    sqlx::query!("DELETE FROM Contacts WHERE id = ?", id)
        .execute(&db)
        .await?;
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

#[derive(Deserialize)]
struct NewContact {
    first: String,
    last: String,
    phone: String,
    email: String,
}
