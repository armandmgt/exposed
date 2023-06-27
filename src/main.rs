mod conf;
mod connections;
mod errors;
mod home;
mod proxy;
mod settings;
mod sshd;
mod util;

use actix_web::middleware::TrailingSlash::Trim;
use actix_web::{middleware, web, App, HttpServer};
use anyhow::Result;
use futures_util::future::join_all;
use sqlx::postgres::PgPoolOptions;
use tokio::signal;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info};

#[allow(clippy::redundant_pub_crate)]
fn http_server_task(
    http_server: actix_web::dev::Server,
    cancellation_token: CancellationToken,
) -> tokio::task::JoinHandle<Result<()>> {
    let handle = http_server.handle();
    tokio::task::spawn(async move {
        tokio::select! {
            res = http_server => {
                res.map_err(Into::into)
            },
            _ = cancellation_token.cancelled() => {
                handle.stop(true).await;
                Ok(())
            }
        }
    })
}

fn sshd_server_task(
    sshd_server: sshd::Server,
    cancellation_token: CancellationToken,
) -> tokio::task::JoinHandle<Result<()>> {
    tokio::task::spawn(async move { sshd_server.start(cancellation_token).await })
}

#[actix_web::main]
async fn main() -> Result<()> {
    env_logger::init();

    let settings = settings::Settings::new()?;

    let db_pool = PgPoolOptions::new()
        .connect(settings.database.url.as_str())
        .await?;
    sqlx::migrate!().run(&db_pool).await?;

    let sshd_server = sshd::Server::new(settings.clone(), db_pool.clone())?;

    let shared_settings = web::Data::new(settings.clone());
    let db_pool = web::Data::new(db_pool);

    let server = HttpServer::new(move || {
        App::new()
            .app_data(db_pool.clone())
            .app_data(shared_settings.clone())
            .wrap(middleware::NormalizePath::new(Trim))
            .wrap(middleware::Logger::new(
                r#"%a %{X-Real-IP}i %t "%r" %s %b "%{Referer}i" "%{User-Agent}i" %T"#,
            ))
            .wrap(middleware::Compress::default())
            .configure(|cfg| home::controller::urls(&shared_settings, cfg))
            .configure(|cfg| conf::controller::urls(&shared_settings, cfg))
            .configure(|cfg| connections::controller::urls(&shared_settings, cfg))
            .configure(|cfg| proxy::controller::urls(&shared_settings, cfg))
    })
    .disable_signals()
    .bind((
        settings
            .http
            .bind_addr
            .unwrap_or_else(|| "127.0.0.1".to_string()),
        settings.http.bind_port.unwrap_or(8080),
    ))?;
    info!(
        "Server listening on {:?}",
        server
            .addrs_with_scheme()
            .into_iter()
            .map(|(addr, scheme)| { format!("{scheme}://{addr}") })
            .collect::<Vec<_>>()
            .join(", ")
    );
    let cancellation_token = CancellationToken::new();
    let tasks = vec![
        http_server_task(server.run(), cancellation_token.clone()),
        sshd_server_task(sshd_server, cancellation_token.clone()),
        tokio::task::spawn(async move {
            signal::ctrl_c()
                .await
                .map(|_| {
                    debug!("received ctrl-c, should now shutdown");
                    cancellation_token.cancel();
                })
                .map_err(Into::into)
        }),
    ];
    let results = join_all(tasks).await;
    results
        .into_iter()
        .map(|join_res| join_res?)
        .fold(Ok(()), Result::and)
}
