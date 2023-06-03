use anyhow::Result;
use config::Config;
use serde::Deserialize;
use std::env;
use std::path::PathBuf;
use url::Url;

#[derive(Debug, Deserialize, Clone)]
pub struct Database {
    pub url: Url,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Http {
    pub url: Url,
    pub bind_addr: Option<String>,
    pub bind_port: Option<u16>,
    pub secure: bool,
    pub secret: String,
    pub vhost_suffix: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Sshd {
    pub server_port: u16,
    pub server_key: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Files {
    pub static_dir: PathBuf,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub database: Database,
    pub http: Http,
    pub sshd: Sshd,
    pub files: Files,
}

impl Settings {
    pub fn new() -> Result<Self, config::ConfigError> {
        let env = env::var("ENV_TYPE").unwrap_or_else(|_| "development".into());
        let s = Config::builder()
            .add_source(config::File::with_name("conf/default"))
            .add_source(config::File::with_name(&format!("conf/{env}")))
            .add_source(config::Environment::with_prefix("APP"))
            .build()?;
        s.try_deserialize::<Self>()
    }
}
