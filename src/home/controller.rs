use actix_web::{get, guard, web, HttpResponse};
use askama::Template;

use crate::{errors::AppResponse, home::views, settings::Settings};

#[allow(clippy::unused_async)]
#[get("/")]
pub async fn index() -> AppResponse {
    let template = views::Index::new("Home");
    let body = template.render()?;
    Ok(HttpResponse::Ok().content_type("text/html").body(body))
}

pub fn urls(settings: &Settings, cfg: &mut web::ServiceConfig) {
    let api_host = settings
        .http
        .url
        .host()
        .map_or_else(|| panic!("No host found for API URL"), |api_host| api_host);
    cfg.service(
        web::scope("")
            .guard(guard::Host(api_host.to_string()))
            .service(index),
    );
}
