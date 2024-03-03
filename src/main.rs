use anyhow::Context;
use axum_extra::routing::RouterExt;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    routing::get,
    Form,
};
use hypertext::Renderable;
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
    Ok(tmpl::contacts(contacts, query.map(|Query(SearchQuery { q })| q)).render())
}

async fn get_contacts_new(_: paths::NewContact) -> impl IntoResponse {
    tmpl::new_contact().render()
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
    Ok(tmpl::view_contact(contact).render())
}

async fn get_contacts_edit(
    paths::EditContact { id }: paths::EditContact,
    State(db): State<SqlitePool>,
) -> Result<impl IntoResponse> {
    let contact = sqlx::query_as!(
        tmpl::Contact,
        "SELECT id, first, last, phone, email FROM Contacts WHERE id = ?",
        id,
    )
    .fetch_one(&db)
    .await?;
    Ok(tmpl::edit_contact(contact).render())
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

// #[get("/contacts/<id>")]
// async fn contacts_view(db: &State<DatabaseConnection>, id: i32) -> ContactViewTemplate {
//     let contact = Contact::find_by_id(id)
//         .one(db.inner())
//         .await
//         .unwrap()
//         .unwrap();
//     ContactViewTemplate { contact }
// }
// #[get("/contacts/<id>/edit")]
// async fn contacts_edit(db: &State<DatabaseConnection>, id: i32) -> ContactEditTemplate {
//     let contact = Contact::find_by_id(id)
//         .one(db.inner())
//         .await
//         .unwrap()
//         .unwrap();
//     let contact = EditContact {
//         id: contact.id,
//         first: contact.first.unwrap_or(String::new()),
//         last: contact.last.unwrap_or(String::new()),
//         phone: contact.phone.unwrap_or(String::new()),
//         email: contact.email.unwrap_or(String::new()),
//     };
//     ContactEditTemplate { contact }
// }
// #[post("/contacts/<id>/edit", data = "<form>")]
// async fn contacts_edit_post(
//     db: &State<DatabaseConnection>,
//     id: i32,
//     form: Form<NewContact<'_>>,
// ) -> Redirect {
//     let contact = contact::ActiveModel {
//         id: ActiveValue::set(id),
//         first: ActiveValue::set(Some(form.first.to_owned())),
//         last: ActiveValue::set(Some(form.last.to_owned())),
//         phone: ActiveValue::set(Some(form.phone.to_owned())),
//         email: ActiveValue::set(Some(form.email.to_owned())),
//     };

//     contact.update(db.inner()).await.unwrap();
//     Redirect::to(uri!(contacts_view(id)))
// }

// #[post("/contact/<id>/delete")]
// async fn contacts_delete(db: &State<DatabaseConnection>, id: i32) -> Redirect {
//     Contact::delete_by_id(id).exec(db.inner()).await.unwrap();
//     Redirect::to(uri!(contacts()))
// }

// #[post("/contacts/new", data = "<form>")]
// async fn contacts_new_post(db: &State<DatabaseConnection>, form: Form<NewContact<'_>>) -> Redirect {
//     let contact = contact::ActiveModel {
//         first: ActiveValue::set(form.first.to_owned().into()),
//         last: ActiveValue::set(form.last.to_owned().into()),
//         phone: ActiveValue::set(form.phone.to_owned().into()),
//         email: ActiveValue::set(form.email.to_owned().into()),
//         ..Default::default()
//     };

//     Contact::insert(contact).exec(db.inner()).await.unwrap();
//     Redirect::to(uri!(contacts()))
// }

#[derive(Deserialize)]
struct NewContact {
    first: String,
    last: String,
    phone: String,
    email: String,
}
