use anyhow::{anyhow, Context, Result};

use crate::settings::Settings;

pub mod wildcard_host_guard;

pub fn extract_subdomain(host: &str, settings: &Settings) -> Result<String> {
    let host_without_port = host.split(':').next().context("Could not get subdomain")?;

    host_without_port
        .strip_suffix(&*settings.http.vhost_suffix)
        .map(ToOwned::to_owned)
        .ok_or_else(|| anyhow!("No subdomain"))
}
