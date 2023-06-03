mod connections;
mod proxy;

use crate::errors::AppResponse;
use crate::settings::Settings;
use crate::views::Index;
use actix_files as fs;
use actix_web::{get, web, HttpResponse};
use askama::Template;

#[allow(clippy::unused_async)]
#[get("/")]
pub async fn index() -> AppResponse {
    let template = Index::new("Home");
    let body = template.render()?;
    Ok(HttpResponse::Ok().content_type("text/html").body(body))
}

pub fn urls(settings: &Settings, cfg: &mut web::ServiceConfig) {
    cfg.service(index)
        .configure(|cfg| connections::urls(settings, cfg))
        .configure(|cfg| proxy::urls(settings, cfg))
        .service(fs::Files::new("/static", &settings.files.static_dir));
}
