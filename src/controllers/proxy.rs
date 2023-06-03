use crate::errors::AppError;
use crate::errors::AppResponse;
use crate::models::connection::Connection;
use crate::settings::Settings;
use crate::util;
use crate::util::wildcard_host_guard::get_host_uri;
use actix_web::http::header::HeaderMap;
use actix_web::http::header::HeaderName;
use actix_web::http::header::CONNECTION;
use actix_web::http::header::PROXY_AUTHENTICATE;
use actix_web::http::header::PROXY_AUTHORIZATION;
use actix_web::http::header::TE;
use actix_web::http::header::TRAILER;
use actix_web::http::header::TRANSFER_ENCODING;
use actix_web::http::header::X_FORWARDED_FOR;
use actix_web::http::uri::Parts;
use actix_web::http::Uri;
use actix_web::HttpResponseBuilder;
use actix_web::{web, HttpRequest, HttpResponse};
use anyhow::Context;
use anyhow::Result;
use sqlx::PgPool;
use std::time::Duration;

const HOP_BY_HOP_HEADERS: [HeaderName; 6] = [
    CONNECTION,
    PROXY_AUTHENTICATE,
    PROXY_AUTHORIZATION,
    TE,
    TRAILER,
    TRANSFER_ENCODING,
];

static STATIC_X_FORWARDED_FOR: HeaderName = X_FORWARDED_FOR;

fn forward_uri_value(connection: &Connection, req: &HttpRequest) -> Result<Uri> {
    let mut forward_url_parts: Parts = req.uri().clone().into();

    forward_url_parts.scheme = Some("http".parse()?);
    let proxy_host = format!(
        "localhost:{}",
        connection
            .proxy_port
            .as_ref()
            .context("No proxy port available for connection")?
    );
    forward_url_parts.authority = Some(proxy_host.parse()?);
    Uri::try_from(forward_url_parts).context("Failed creating proxy URI from parts")
}

fn x_forwarded_for_value(req: &HttpRequest) -> String {
    let mut result = String::new();

    for (key, value) in req.headers() {
        if key == STATIC_X_FORWARDED_FOR {
            if let Ok(value) = value.to_str() {
                result.push_str(value);
            }
            break;
        }
    }

    if let Some(peer_addr) = req.peer_addr() {
        if !result.is_empty() {
            result.push_str(", ");
        }
        let client_ip_str = &format!("{}", peer_addr.ip());
        result.push_str(client_ip_str);
    }
    result
}

fn remove_connection_headers(headers: &mut HeaderMap) {
    headers.remove(CONNECTION);
}

fn remove_hop_by_hop_headers(headers: &mut HeaderMap) {
    for header in HOP_BY_HOP_HEADERS {
        headers.remove(header);
    }
}

fn copy_except_hop_by_hop(source_headers: &HeaderMap, resp_builder: &mut HttpResponseBuilder) {
    for header in source_headers {
        if !HOP_BY_HOP_HEADERS.contains(header.0) {
            resp_builder.insert_header(header);
        }
    }
}

#[allow(clippy::future_not_send)]
pub async fn process(
    req: HttpRequest,
    payload: web::Payload,
    db: web::Data<PgPool>,
    settings: web::Data<Settings>,
) -> AppResponse {
    let host = get_host_uri(req.head())
        .context("Could parse Host")?
        .to_string();
    let host_without_port = host.split(':').next().context("Could not get subdomain")?;
    let Some(subdomain) = host_without_port.strip_suffix(&*settings.http.vhost_suffix) else {
        return Ok(HttpResponse::NotFound().finish())
    };
    let connection = Connection::get_by_subdomain(&db, subdomain)
        .await
        .map_err(|_| AppError::NotFound)?;
    if connection.proxy_port.is_none() {
        return Err(AppError::NotFound);
    }

    let mut forward_req = awc::Client::new()
        .request_from(req.uri(), req.head())
        .no_decompress()
        .timeout(Duration::from_secs(60))
        .uri(forward_uri_value(&connection, &req)?)
        .insert_header_if_none((actix_web::http::header::USER_AGENT, ""))
        .append_header((X_FORWARDED_FOR, x_forwarded_for_value(&req)));

    remove_connection_headers(forward_req.headers_mut());
    remove_hop_by_hop_headers(forward_req.headers_mut());

    let backend_resp = forward_req
        .send_stream(payload)
        .await?
        .timeout(Duration::from_secs(10));

    let mut resp_builder = HttpResponse::build(backend_resp.status());

    copy_except_hop_by_hop(backend_resp.headers(), &mut resp_builder);

    let mut resp = resp_builder.streaming(backend_resp);

    remove_connection_headers(resp.headers_mut());

    Ok(resp)
}

pub fn urls(settings: &Settings, cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("")
            .guard(util::wildcard_host_guard::WildcardHost(
                settings.http.vhost_suffix.clone(),
            ))
            .default_service(web::to(process)),
    );
}
