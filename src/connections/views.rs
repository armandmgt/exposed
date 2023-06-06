use derive_more::Constructor;
use serde::Serialize;
pub use sqlx::types::Uuid;

use super::dto;

#[derive(Serialize, Constructor)]
pub struct IndexView<'a> {
    pub connections: &'a Vec<dto::View>,
}
