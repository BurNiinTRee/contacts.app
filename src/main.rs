use axum::{
    extract::FromRef,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use axum_extra::routing::RouterExt;
use sqlx::{postgres::PgConnectOptions, PgPool};
use tracing::info;

mod assets;
mod model;
mod pages;

#[derive(Clone, FromRef)]
struct AppState {
    db: PgPool,
    contacts: model::Contacts,
    archiver: model::Archiver,
    flash_config: axum_flash::Config,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    console_subscriber::init();
    let db_options = std::env::var("DATABASE_URL")
        .as_deref()
        .unwrap_or("sqlite:data.db?mode=rwc")
        .parse::<PgConnectOptions>()?;

    let db = sqlx::PgPool::connect_with(db_options).await?;
    sqlx::migrate!().run(&db).await?;

    let contacts = model::Contacts::new(db.clone());
    let archiver = model::Archiver::new(contacts.clone());

    let flash_config = axum_flash::Config::new(axum_flash::Key::generate());

    let app_state = AppState {
        db,
        contacts,
        archiver,
        flash_config,
    };

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    info!("Listening on http://{}", listener.local_addr()?);
    axum::serve(
        listener,
        axum::Router::new()
            .typed_get(pages::get)
            .typed_get(pages::contacts::get)
            .typed_post(pages::contacts::post)
            .typed_delete(pages::contacts::delete)
            .typed_get(pages::contacts::archive::get)
            .typed_post(pages::contacts::archive::post)
            .typed_delete(pages::contacts::archive::delete)
            .typed_get(pages::contacts::count::get)
            .typed_get(pages::contacts::new::get)
            .typed_get(pages::contacts::item::get)
            .typed_get(pages::contacts::item::edit::get)
            .typed_post(pages::contacts::item::put)
            .typed_get(pages::contacts::item::email::get)
            .typed_delete(pages::contacts::item::delete)
            .nest_service("/assets", tower_http::services::ServeDir::new("assets"))
            .route_service(
                &pages::contacts::archive::file::Path.to_string(),
                tower_http::services::ServeFile::new("run/export.csv"),
            )
            .typed_get(assets::get_style)
            .route_service(
                &assets::StyleSource.to_string(),
                tower_http::services::ServeFile::new("styles/index.scss"),
            )
            .with_state(app_state),
    )
    .await?;

    Ok(())
}

type Result<T, E = ServerError> = std::result::Result<T, E>;

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
