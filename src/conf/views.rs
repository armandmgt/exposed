use derive_more::Constructor;
use serde::Serialize;
pub use sqlx::types::Uuid;

#[derive(Serialize, Constructor)]
pub struct ShowView<'a> {
    pub sshd_port: &'a str,
    pub sshd_fingerprint: &'a str,
}
