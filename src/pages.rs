use axum::response::Redirect;
use axum_extra::routing::TypedPath;

pub mod contacts;

#[derive(TypedPath)]
#[typed_path("/")]
pub struct Path;

pub async fn get(_: Path) -> Redirect {
    Redirect::to(&contacts::Path.to_string())
}
