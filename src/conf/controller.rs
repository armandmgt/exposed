use actix_web::{get, guard, http::header, web, HttpResponse};

use crate::{conf::views, errors::AppResponse, settings::Settings};

#[get("")]
#[allow(clippy::unused_async)]
pub async fn show(settings: web::Data<Settings>) -> AppResponse {
    let port_str = format!("{}", settings.sshd.server_port);
    let server_key = russh_keys::decode_secret_key(&settings.sshd.server_key, None)?;
    let fingerprint = server_key.clone_public_key()?.fingerprint();

    let index_view = views::ShowView::new(&port_str, &fingerprint);
    let body = serde_json::to_string(&index_view)?;

    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .body(body))
}

pub fn urls(settings: &Settings, cfg: &mut web::ServiceConfig) {
    let api_host = settings
        .http
        .url
        .host()
        .map_or_else(|| panic!("No host found for API URL"), |api_host| api_host);
    cfg.service(
        web::scope("/conf")
            .guard(guard::Host(api_host.to_string()))
            .guard(guard::Header(header::ACCEPT.as_str(), "application/json"))
            .service(show),
    );
}
