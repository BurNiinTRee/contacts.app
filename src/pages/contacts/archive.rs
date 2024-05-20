use crate::{model, Result};
use tracing::instrument;

use axum::{
    extract::State,
    response::{IntoResponse, Response},
};
use axum_extra::routing::TypedPath;

pub mod file {
    use axum_extra::routing::TypedPath;

    #[derive(TypedPath)]
    #[typed_path("/contacts/archive/file")]
    pub struct Path;
}

#[derive(TypedPath)]
#[typed_path("/contacts/archive")]
pub struct Path;

#[instrument(skip_all)]
pub async fn post(_: Path, State(archiver): State<model::Archiver>) -> Result<Response> {
    archiver.run().await?;
    Ok(super::Archive {
        archiver_status: archiver.status().await?,
    }
    .into_response())
}

pub async fn get(_: Path, State(archiver): State<model::Archiver>) -> Result<super::Archive> {
    Ok(super::Archive {
        archiver_status: archiver.status().await?,
    })
}

pub async fn delete(_: Path, State(archiver): State<model::Archiver>) -> Result<super::Archive> {
    archiver.reset().await?;
    Ok(super::Archive {
        archiver_status: archiver.status().await?,
    })
}
