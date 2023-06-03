use actix_web::dev::RequestHead;
use actix_web::guard::{Guard, GuardContext};
use actix_web::http::{header, Uri};

#[allow(non_snake_case)]
pub fn WildcardHost(host: impl AsRef<str>) -> WildcardHostGuard {
    WildcardHostGuard {
        host: host.as_ref().to_string(),
    }
}

pub fn get_host_uri(req: &RequestHead) -> Option<Uri> {
    req.headers
        .get(header::HOST)
        .and_then(|host_value| host_value.to_str().ok())
        .or_else(|| req.uri.host())
        .and_then(|host| host.parse().ok())
}

#[doc(hidden)]
pub struct WildcardHostGuard {
    host: String,
}

impl Guard for WildcardHostGuard {
    fn check(&self, ctx: &GuardContext<'_>) -> bool {
        let Some(req_host_uri) = get_host_uri(ctx.head()) else {
            return false;
        };

        match req_host_uri.host() {
            Some(uri_host) if uri_host.ends_with(&*self.host) => {}
            _ => return false,
        }
        true
    }
}
