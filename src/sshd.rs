use std::{net::SocketAddr, sync::Arc, time::Duration};

use anyhow::Result;
use async_trait::async_trait;
use russh::server::{self, Auth, Handle, Session};
use sqlx::PgPool;
use tokio::net::TcpStream;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info};

use crate::{
    connections::models::Connection, errors::StaticError, settings::Settings,
    util::extract_subdomain,
};

struct TcpIpForwardTask {
    cancellation_token: CancellationToken,
    join_handle: tokio::task::JoinHandle<Result<()>>,
}

pub struct Server {
    config: Arc<russh::server::Config>,
    settings: Arc<Settings>,
    server_pubkey: Arc<russh_keys::key::PublicKey>,
    id: usize,
    db: Arc<PgPool>,
    tcpip_forward_task: Option<TcpIpForwardTask>,
}

impl Clone for Server {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            settings: self.settings.clone(),
            server_pubkey: self.server_pubkey.clone(),
            id: self.id,
            db: self.db.clone(),
            tcpip_forward_task: None,
        }
    }
}

impl Server {
    pub fn new(settings: Settings, db: PgPool) -> Result<Self, StaticError> {
        let server_key = russh_keys::decode_secret_key(&settings.sshd.server_key, None)?;
        let pub_key = server_key.clone_public_key()?;
        let config = russh::server::Config {
            methods: russh::MethodSet::PASSWORD,
            connection_timeout: Some(Duration::from_secs(3600)),
            keys: vec![server_key],
            ..russh::server::Config::default()
        };

        Ok(Self {
            config: Arc::new(config),
            settings: Arc::new(settings),
            server_pubkey: Arc::new(pub_key),
            id: 0,
            db: Arc::new(db),
            tcpip_forward_task: None,
        })
    }

    pub async fn start(self, cancellation_token: CancellationToken) -> Result<()> {
        info!(
            "sshd server key fingerprint: {}",
            self.server_pubkey.fingerprint()
        );

        let bind_addr = format!("0.0.0.0:{}", self.settings.sshd.server_port);
        tokio::select! {
            res = russh::server::run(self.config.clone(), bind_addr, self) => {
                res.map_err(Into::into)
            },
            _ = cancellation_token.cancelled() => {
                Ok(())
            }
        }
    }
}

impl server::Server for Server {
    type Handler = Self;

    fn new_client(&mut self, _peer_addr: Option<std::net::SocketAddr>) -> Self::Handler {
        let s = self.clone();
        self.id += 1;
        s
    }
}

#[async_trait]
impl server::Handler for Server {
    type Error = anyhow::Error;

    async fn auth_password(
        self,
        _user: &str,
        _password: &str,
    ) -> Result<(Self, Auth), Self::Error> {
        Ok((self, Auth::Accept))
    }

    async fn tcpip_forward(
        mut self,
        address: &str,
        port: &mut u32,
        session: Session,
    ) -> Result<(Self, bool, Session), Self::Error> {
        debug!("tcpip_forward: {address} {port}");
        let subdomain = extract_subdomain(address, &self.settings)?;
        let mut connection = Connection::get_by_subdomain(&self.db, &subdomain).await?;

        let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
            .await
            .map_err(|e| {
                debug!("{e:?}");
                e
            })?;
        let address = address.to_owned();
        let listen_addr = listener.local_addr().map_err(|e| {
            debug!("{e:?}");
            e
        })?;
        *port = listen_addr.port().into();

        connection.proxy_port = Some(port.to_string());
        connection.save(&self.db).await.map_err(|e| {
            debug!("{e:?}");
            e
        })?;

        let client_handle = session.handle();
        let cancellation_token = CancellationToken::new();
        let task_token = cancellation_token.clone();
        let join_handle = tokio::task::spawn(async move {
            debug!("Starting forward task");
            loop {
                tokio::select! {
                    accept = listener.accept() => {
                        match accept {
                            Ok((tcp_stream, addr)) => {
                                debug!("Accepted connection starting stream task");
                                tokio::task::spawn(tcpip_forward_stream_handler(
                                    address.clone(),
                                    listen_addr.port(),
                                    client_handle.clone(),
                                    tcp_stream,
                                    addr,
                                ));
                            }
                            Err(e) => {
                                debug!("Error when accepting: {e:?}");
                                return Err(e.into());
                            }
                        }
                    },
                    _ = task_token.cancelled() => {
                        debug!("Forward task cancelled");
                        return Ok(());
                    }
                }
            }
        });
        self.tcpip_forward_task = Some(TcpIpForwardTask {
            cancellation_token,
            join_handle,
        });
        Ok((self, true, session))
    }

    async fn cancel_tcpip_forward(
        mut self,
        address: &str,
        _port: u32,
        session: Session,
    ) -> Result<(Self, bool, Session), Self::Error> {
        debug!("cancel_tcpip_forward: cancelling");
        if let Some(forward_task) = self.tcpip_forward_task.take() {
            let subdomain = extract_subdomain(address, &self.settings)?;
            let mut connection = Connection::get_by_subdomain(&self.db, &subdomain).await?;
            connection.proxy_port = None;
            connection.save(&self.db).await?;

            forward_task.cancellation_token.cancel();
            forward_task.join_handle.await??;
            Ok((self, true, session))
        } else {
            Ok((self, false, session))
        }
    }
}

async fn tcpip_forward_stream_handler(
    local_addr: String,
    local_port: u16,
    client_handle: Handle,
    mut tcp_stream: TcpStream,
    addr: SocketAddr,
) -> Result<()> {
    let (remote_addr, remote_port) = (addr.ip(), addr.port());
    debug!("handler_tcpip_forward_stream: {remote_addr} {remote_port} / {local_addr} {local_port}");
    let channel = client_handle
        .channel_open_forwarded_tcpip(
            local_addr.to_string(),
            local_port.into(),
            remote_addr.to_string(),
            remote_port.into(),
        )
        .await?;
    let mut channel = channel.into_stream();

    tokio::io::copy_bidirectional(&mut tcp_stream, &mut channel)
        .await
        .and(Ok(()))
        .map_err(Into::into)
}
