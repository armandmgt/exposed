use crate::dto;
use crate::errors::AppResponse;
use crate::models::connection::Connection;
use crate::settings::Settings;
use crate::views::connections;
use actix_web::{delete, get, guard, post, web, HttpResponse};
use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

#[get("")]
pub async fn index(db: web::Data<PgPool>) -> AppResponse {
    let connections = Connection::get_all(&db).await?;
    let connection_views = connections
        .into_iter()
        .map(|x| {
            dto::connection::View::new(
                x.id.to_string(),
                x.subdomain,
                x.proxied_port,
                x.upstream_port,
            )
        })
        .collect();
    let index_view = connections::IndexView::new(&connection_views);
    let body = serde_json::to_string(&index_view)?;
    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .body(body))
}

#[post("")]
pub async fn create(db: web::Data<PgPool>, params: web::Json<dto::connection::Create>) -> AppResponse {
    let connection = Connection::new(params.subdomain.clone(), params.proxied_port.clone());
    connection.insert(&db).await?;
    let connection_view = dto::connection::View::new(
        connection.id.to_string(),
        connection.subdomain.clone(),
        connection.proxied_port.clone(),
        connection.upstream_port.clone(),
    );
    let create_view = dto::connection::ShowView::new(connection_view);
    let body = serde_json::to_string(&create_view)?;
    Ok(HttpResponse::Created()
        .content_type("application/json")
        .body(body))
}

#[delete("/{uuid}")]
pub async fn delete(db: web::Data<PgPool>, path: web::Path<String>) -> AppResponse {
    let uuid = Uuid::parse_str(&path.into_inner()).context("Failed to parse connection UUID")?;
    let connection = Connection::get(&db, &uuid).await?;
    connection.delete(&db).await?;
    let connection_view = dto::connection::View::new(
        connection.id.to_string(),
        connection.subdomain.clone(),
        connection.proxied_port.clone(),
        connection.upstream_port.clone(),
    );
    let delete_view = dto::connection::ShowView::new(connection_view);
    let body = serde_json::to_string(&delete_view)?;
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
        web::scope("/connections")
            .guard(guard::Host(api_host.to_string()))
            .service(
                web::scope("")
                    .guard(guard::Header("Content-Type", "application/json"))
                    .service(index)
                    .service(create)
                    .service(delete),
            ),
    );
}
