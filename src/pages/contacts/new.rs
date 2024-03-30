use askama::Template;
use axum::response::IntoResponse;
use axum_extra::routing::TypedPath;
use axum_flash::IncomingFlashes;
use serde::Deserialize;

use super::shared;

#[derive(Template)]
#[template(path = "new-contact.html")]
pub struct Tmpl {
    pub layout: shared::Layout,
    pub contact: shared::Contact,
}

#[derive(TypedPath, Deserialize)]
#[typed_path("/contacts/new")]
pub struct Path;

pub async fn get(_: Path, flashes: IncomingFlashes) -> impl IntoResponse {
    Tmpl {
        layout: shared::Layout {
            flashes: Some(flashes),
        },
        contact: shared::Contact::default(),
    }
}
