use anyhow::Context;
use axum_extra::routing::RouterExt;
use axum_flash::IncomingFlashes;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    routing::get,
    Form,
};
use serde::Deserialize;
use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};

mod paths;
mod tmpl;

type Result<T, E = ServerError> = std::result::Result<T, E>;

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
            .typed_delete(delete_contact)
            .nest_service("/assets", tower_http::services::ServeDir::new("assets"))
            .with_state(db),
    )
    .await?;

    Ok(())
}

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
}

async fn get_contacts(
    _: paths::Contacts,
    State(db): State<SqlitePool>,
    query: Option<Query<SearchQuery>>,
) -> Result<impl IntoResponse> {
    let contacts = match query {
        Some(Query(SearchQuery { ref q })) => {
            sqlx::query_as!(
                tmpl::Contact,
                r"SELECT id, first, last, phone, email FROM Contacts 
                    WHERE first LIKE CONCAT('%', ?1, '%')
                       OR last LIKE CONCAT('%', ?1, '%')
                    ORDER BY first ASC
                ",
                q
            )
            .fetch_all(&db)
            .await?
        }
        None => {
            sqlx::query_as!(
                tmpl::Contact,
                "SELECT id, first, last, phone, email FROM Contacts ORDER BY first ASC"
            )
            .fetch_all(&db)
            .await?
        }
    };
    Ok(tmpl::Contacts {
        layout: Default::default(),
        contacts,
        search_term: query.map(|Query(SearchQuery { q })| q),
    })
}

async fn get_contacts_new(_: paths::NewContact) -> impl IntoResponse {
    tmpl::NewContact::default()
}

async fn post_contacts_new(
    _: paths::NewContact,
    State(db): State<SqlitePool>,
    Form(contact): Form<NewContact>,
) -> Result<Redirect> {
    let result = sqlx::query!(
        "INSERT INTO Contacts (first, last, phone, email) VALUES (?, ?, ?, ?) RETURNING id",
        contact.first,
        contact.last,
        contact.phone,
        contact.email,
    )
    .fetch_one(&db)
    .await?;
    Ok(Redirect::to(&paths::Contact { id: result.id }.to_string()))
}

async fn get_contacts_view(
    paths::Contact { id }: paths::Contact,
    State(db): State<SqlitePool>,
) -> Result<impl IntoResponse> {
    let contact = sqlx::query_as!(
        tmpl::Contact,
        "SELECT id, first, last, phone, email FROM Contacts WHERE id = ?",
        id,
    )
    .fetch_one(&db)
    .await?;
    Ok(tmpl::ViewContact {
        layout: Default::default(),
        contact,
    })
}

async fn get_contacts_edit(
    paths::EditContact { id }: paths::EditContact,
    flashes: IncomingFlashes,
    State(db): State<SqlitePool>,
) -> Result<impl IntoResponse> {
    let contact = sqlx::query_as!(
        tmpl::Contact,
        "SELECT id, first, last, phone, email FROM Contacts WHERE id = ?",
        id,
    )
    .fetch_one(&db)
    .await?;
    Ok(tmpl::EditContact {
        layout: Default::default(),
        contact,
    })
}

async fn post_contacts_edit(
    paths::EditContact { id }: paths::EditContact,
    State(db): State<SqlitePool>,
    Form(contact): Form<NewContact>,
) -> Result<Redirect> {
    let result = sqlx::query!(
        "UPDATE Contacts SET first = ?, last = ?, phone = ?, email = ? WHERE id = ? RETURNING id",
        contact.first,
        contact.last,
        contact.phone,
        contact.email,
        id
    )
    .fetch_one(&db)
    .await?;
    Ok(Redirect::to(
        &paths::Contact {
            id: result
                .id
                .with_context(|| format!("No Contact with id: {}", id))?,
        }
        .to_string(),
    ))
}

async fn delete_contact(
    paths::Contact { id }: paths::Contact,
    State(db): State<SqlitePool>,
) -> Result<Redirect> {
    sqlx::query!("DELETE FROM Contacts WHERE id = ?", id)
        .execute(&db)
        .await?;
    Ok(Redirect::to(&paths::Contacts.to_string()))
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
