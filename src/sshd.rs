use std::{net::SocketAddr, sync::Arc, time::Duration};

use anyhow::Result;
use async_trait::async_trait;
use russh::{
    server::{self, Auth, Handle, Session},
    ChannelMsg,
};
use tokio::{io::AsyncWriteExt, net::TcpStream};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info};

use crate::{errors::StaticError, settings};

pub struct Server {
    config: Arc<russh::server::Config>,
    server_port: u16,
    server_pubkey: Arc<russh_keys::key::PublicKey>,
    id: usize,
    tcpip_forward_listener: Option<tokio::task::JoinHandle<Result<(), StaticError>>>,
}

impl Clone for Server {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            server_port: self.server_port,
            server_pubkey: self.server_pubkey.clone(),
            id: self.id,
            tcpip_forward_listener: None,
        }
    }
}

impl Server {
    pub fn new(sshd_settings: &settings::Sshd) -> Result<Self, StaticError> {
        let server_key = russh_keys::decode_secret_key(&sshd_settings.server_key, None)?;
        let server_pubkey = Arc::new(server_key.clone_public_key()?);
        let config = russh::server::Config {
            methods: russh::MethodSet::PASSWORD,
            connection_timeout: Some(Duration::from_secs(3600)),
            keys: vec![server_key],
            ..russh::server::Config::default()
        };
        let config = Arc::new(config);
        Ok(Self {
            config,
            server_port: sshd_settings.server_port,
            server_pubkey,
            id: 0,
            tcpip_forward_listener: None,
        })
    }

    pub async fn start(self, cancellation_token: CancellationToken) -> Result<()> {
        info!(
            "sshd server key fingerprint: {}",
            self.server_pubkey.fingerprint()
        );

        let server_port = format!("0.0.0.0:{}", self.server_port);
        tokio::select! {
            res = russh::server::run(self.config.clone(), server_port, self) => {
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
    type Error = StaticError;

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

        let listener = tokio::net::TcpListener::bind(format!("{address}:{port}")).await?;
        let address = address.to_owned();
        let listen_addr = listener.local_addr()?;
        *port = listen_addr.port().into();
        let client_handle = session.handle();
        self.tcpip_forward_listener = Some(tokio::task::spawn(async move {
            while let Ok((tcp_stream, addr)) = listener.accept().await {
                tokio::task::spawn(handler_tcpip_forward_stream(
                    address.clone(),
                    listen_addr.port(),
                    client_handle.clone(),
                    tcp_stream,
                    addr,
                ));
            }
            Ok(())
        }));
        Ok((self, true, session))
    }
}

async fn handler_tcpip_forward_stream(
    local_addr: String,
    local_port: u16,
    client_handle: Handle,
    mut tcp_stream: TcpStream,
    addr: SocketAddr,
) -> Result<()> {
    let (remote_addr, remote_port) = (addr.ip(), addr.port());
    debug!("handler_tcpip_forward_stream: {remote_addr} {remote_port} / {local_addr} {local_port}");
    let mut channel = client_handle
        .channel_open_forwarded_tcpip(
            local_addr.to_string(),
            local_port.into(),
            remote_addr.to_string(),
            remote_port.into(),
        )
        .await?;

    let (read_half, write_half) = tcp_stream.split();

    loop {
        let mut buf: [u8; 512] = [0; 512];
        tokio::select! {
            _ = read_half.readable() => {
                match read_half.try_read(&mut buf) {
                    Ok(0) => {
                        channel.eof().await?;
                    },
                    Ok(_) => {
                        channel.data(&buf[..]).await?;
                    },
                    Err(e) => { return Err(e.into()); },
                }
            },
            data = channel.wait() => {
                match data {
                    Some(ChannelMsg::Data { data }) => {
                        match write_half.try_write(&data) {
                            Ok(_) => {},
                            Err(e) => return Err(e.into()),
                        }
                    },
                    Some(ChannelMsg::Eof) => {
                        tcp_stream.shutdown().await?;
                        return Ok(());
                    },
                    Some(_) | None => { return Ok(()); },
                }
            }
        };
    }
}
