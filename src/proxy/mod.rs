use std::io;
use std::net::ToSocketAddrs;
use std::sync::Arc;

use dashmap::{DashMap, Entry};
use tokio::net::UdpSocket;
use uuid::Uuid;

use crate::packet::VoicePacketWrapper;
use crate::proxy::connection::VoiceProxyConnection;

mod connection;

#[derive(Clone)]
pub struct VoiceProxy {
    socket: Arc<UdpSocket>,
    clients: Arc<DashMap<Uuid, Arc<VoiceProxyConnection>>>
}

impl VoiceProxy {
    pub async fn listen(&mut self) -> io::Result<()> {
        loop {
            self.socket.readable().await?;

            let mut buf = Vec::with_capacity(1500);
            let (packet, client_address) = match self.socket.try_recv_buf_from(&mut buf) {
                Ok((n, addr)) => {
                    (&buf[..n], addr)
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    continue;
                }
                Err(e) => {
                    return Err(e);
                }
            };

            let voice_packet = match VoicePacketWrapper::try_from(packet) {
                Ok(voice_packet) => voice_packet,
                Err(_) => continue
            };

            if let Entry::Vacant(entry) = self.clients.entry(voice_packet.secret) {
                println!("New connection {:?}", client_address);

                let connection = Arc::new(
                    VoiceProxyConnection::new(
                        self.socket.clone(),
                        voice_packet.secret,
                        client_address,
                        "127.0.0.1:25565".to_socket_addrs()?.next().unwrap() // todo: ??
                    ).await?
                );
                entry.insert(connection.clone());

                let proxy = self.clone();
                tokio::spawn(async move {
                    _ = connection.listen().await;
                    proxy.clients.remove(&connection.secret);
                    println!("Connection {:?} removed", connection.client_address);
                });
            }

            let connection = if let Some(connection) = self.clients.get(&voice_packet.secret) {
                connection
            } else {
                continue
            };

            connection.send_to_server(packet).await?;
        }
    }

    pub async fn new(port: u16) -> io::Result<Self> {
        let socket = UdpSocket::bind(format!("0.0.0.0:{}", port)).await?;

        let proxy = Self {
            socket: Arc::new(socket),
            clients: Arc::new(DashMap::new())
        };

        Ok(proxy)
    }
}
