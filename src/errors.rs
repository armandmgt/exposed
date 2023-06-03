use crate::views::Error as ViewError;
use actix_web::http::StatusCode;
use actix_web::HttpResponse;
use actix_web::ResponseError;
use askama::Template;
use std::fmt::Debug;
use thiserror::Error;
use tokio::task::JoinError;
use tracing::error;

fn internal_server_error(e: &dyn std::error::Error) -> HttpResponse {
    error!("Internal server error: {:#?}", e);
    let mut res = HttpResponse::InternalServerError();
    res.content_type("text/html");
    let template = ViewError::new("Internal Server Error", 500, "Internal Server Error.");
    let Ok(body) = template.render() else {
        return res.finish();
    };
    res.body(body)
}

fn unprocessable_entity(msg: &str) -> HttpResponse {
    error!("Unprocessable_entity: {:#?}", msg);
    let mut res = HttpResponse::UnprocessableEntity();
    res.content_type("text/html");
    let template = ViewError::new("Unprocessable Entity", 422, msg);
    let Ok(body) = template.render() else {
        return res.finish();
    };
    res.body(body)
}

#[derive(Error, Debug)]
pub enum AppError {
    #[error("actix web error {0}")]
    Actix(#[from] actix_web::Error),
    #[error("askama error {0}")]
    Template(#[from] askama::Error),
    #[error("database error {0}")]
    Database(#[from] sqlx::Error),
    #[error("awc error {0}")]
    Awc(#[from] awc::error::SendRequestError),
    #[error("serde_json error {0}")]
    SerdeJson(#[from] serde_json::Error),
    #[error("not found")]
    NotFound,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    #[tracing::instrument]
    fn error_response(&self) -> HttpResponse {
        match self {
            Self::NotFound => {
                let mut res = HttpResponse::NotFound();
                res.content_type("text/html");
                let template =
                    ViewError::new("Error 404", 404, "The page you visited was not found.");
                let Ok(body) = template.render() else {
                    return res.finish();
                };
                res.body(body)
            }
            Self::Database(reason) => unprocessable_entity(
                reason
                    .as_database_error()
                    .map_or("Database error", |e| e.message()),
            ),
            e => internal_server_error(e),
        }
    }
}

pub type AppResponse<T = HttpResponse> = Result<T, AppError>;

#[derive(Error, Debug)]
pub enum StaticError {
    #[error("io error {0}")]
    Io(#[from] std::io::Error),
    #[error("task join error {0}")]
    TokioJoin(#[from] JoinError),
    #[error("russh error {0}")]
    Thrussh(#[from] russh::Error),
    #[error("russh_keys error {0}")]
    ThrusshKeys(#[from] russh_keys::Error),
}
