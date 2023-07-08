use std::str::FromStr;

use actix_web::dev::RequestHead;
use actix_web::guard::{Guard, GuardContext};
use actix_web::http::{header, Uri};
use tracing::debug;

pub fn get_uri_host(req: &RequestHead) -> Option<String> {
    req.headers
        .get(header::HOST)
        .and_then(|header_value| header_value.to_str().ok())
        .and_then(|host_str| Uri::from_str(host_str).ok())
        .and_then(|host_uri| host_uri.host().map(ToOwned::to_owned))
        .or_else(|| req.uri.host().map(ToOwned::to_owned))
}

#[doc(hidden)]
pub struct WildcardHostGuard {
    pub host: String,
}

impl Guard for WildcardHostGuard {
    fn check(&self, ctx: &GuardContext<'_>) -> bool {
        let Some(host) = get_uri_host(ctx.head()) else {
            return false;
        };

        debug!("wildcard_host_guard: uri_host {host:?}");
        host.ends_with(&*self.host)
    }
}
