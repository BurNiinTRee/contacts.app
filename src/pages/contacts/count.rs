use crate::{model, Result};
use axum::extract::State;
use axum_extra::routing::TypedPath;

#[derive(TypedPath)]
#[typed_path("/contacts/count")]
pub struct Path;

pub async fn get(_: Path, State(contacts): State<model::Contacts>) -> Result<String> {
    let count = contacts.count().await?;
    Ok(format!("({} total Contacts)", count))
}
