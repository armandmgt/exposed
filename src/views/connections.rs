use crate::dto;
use derive_more::Constructor;
use serde::Serialize;
pub use sqlx::types::Uuid;

#[derive(Serialize, Constructor)]
pub struct IndexView<'a> {
    pub connections: &'a Vec<dto::connection::View>,
}
