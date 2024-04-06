use axum::{
    extract::Request,
    response::{IntoResponse, Response},
};
use axum_extra::routing::TypedPath;
use tower_service::Service;

use crate::Result;

#[derive(TypedPath)]
#[typed_path("/assets/style.css")]
pub struct Style;

#[derive(TypedPath)]
#[typed_path("/styles/index.scss")]
pub struct StyleSource;

pub async fn get_style(_: Style, req: Request) -> Result<Response> {
    tokio::process::Command::new("sass")
        .args(["--update", "styles/index.scss:run/style.css"])
        .status()
        .await?;

    Ok(tower_http::services::ServeFile::new("run/style.css")
        .call(req)
        .await?
        .into_response())
}
