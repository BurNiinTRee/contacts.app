use axum::{
    extract::FromRef,
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use axum_extra::routing::RouterExt;
use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};

type Result<T, E = ServerError> = std::result::Result<T, E>;

#[derive(Clone, FromRef)]
struct AppState {
    db: SqlitePool,
    contacts: model::Contacts,
    flash_config: axum_flash::Config,
}

mod model;
mod pages;

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
                get(move || async move { Redirect::to(&pages::contacts::Path.to_string()) }),
            )
            .typed_get(pages::contacts::get)
            .typed_post(pages::contacts::post)
            .typed_get(pages::contacts::new::get)
            .typed_get(pages::contacts::item::get)
            .typed_get(pages::contacts::item::edit::get)
            .typed_post(pages::contacts::item::put)
            .typed_get(pages::contacts::item::email::get)
            .typed_delete(pages::contacts::item::delete)
            .nest_service("/assets", tower_http::services::ServeDir::new("assets"))
            .with_state(app_state),
    )
    .await?;

    Ok(())
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
