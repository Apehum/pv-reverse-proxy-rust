use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::hash::Hash;
use std::io;
use std::net::ToSocketAddrs;
use std::sync::Arc;

use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::packet::VoicePacketWrapper;
use crate::proxy::connection::VoiceProxyConnection;

mod connection;

pub struct VoiceProxy {
    socket: Arc<UdpSocket>,
    clients: HashMap<Uuid, VoiceProxyConnection>
}

enum ConnectionMessage {
    Removed(Uuid)
}

impl VoiceProxy {
    pub async fn listen(&mut self) -> io::Result<()> {
        loop {
            let (tx, mut rx) = mpsc::channel::<ConnectionMessage>(100);

            tokio::select! {
                result = self.socket.readable() => { result? }
                
                Some(message) = rx.recv() => {
                    
                    match message {
                        ConnectionMessage::Removed(secret) => {
                            self.clients.remove(&secret);       
                        }
                    }
                    
                    continue;
                }
            }

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

                let connection = VoiceProxyConnection::new(
                    self.socket.clone(),
                    voice_packet.secret,
                    client_address,
                    "127.0.0.1:25565".to_socket_addrs()?.next().unwrap() // todo: ??
                ).await?;
                entry.insert(connection.clone());

                let tx1 = tx.clone();
                
                tokio::spawn(async move {
                    _ = connection.listen().await;
                    _ = tx1.send(ConnectionMessage::Removed(connection.secret));
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
            clients: HashMap::new()
        };

        Ok(proxy)
    }
}
