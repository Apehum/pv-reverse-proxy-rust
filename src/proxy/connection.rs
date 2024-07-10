use std::io;
use std::io::ErrorKind;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use tokio::net::UdpSocket;
use uuid::Uuid;

#[derive(Debug)]
pub struct VoiceProxyConnection {
    proxy_socket: Arc<UdpSocket>,
    pub secret: Uuid,
    pub client_address: SocketAddr,
    server_socket: UdpSocket
}

impl VoiceProxyConnection {
    pub async fn send_to_server(&self, buf: &[u8]) -> io::Result<()> {
        self.server_socket.send(buf).await?;
        Ok(())
    }

    pub async fn listen(&self) -> io::Result<()> {
        loop {
            let sleep = tokio::time::sleep(Duration::from_secs(20));
            tokio::pin!(sleep);
            
            tokio::select! {
                _ = &mut sleep => {
                    return Err(io::Error::new(ErrorKind::TimedOut, "Connection timed out"));
                }

                _ = self.server_socket.readable() => {
                    drop(sleep)
                }
            }

            let mut buf = Vec::with_capacity(1500);
            let (packet, _) = match self.server_socket.try_recv_buf_from(&mut buf) {
                Ok((n, addr)) => {
                    (&buf[..n], addr)
                }
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                    continue;
                }
                Err(e) => {
                    return Err(e);
                }
            };

            self.proxy_socket.send_to(packet, self.client_address).await?;
        }
    }
}

impl VoiceProxyConnection {
    pub async fn new(
        proxy_socket: Arc<UdpSocket>,
        secret: Uuid,
        client_address: SocketAddr,
        server_address: SocketAddr
    ) -> io::Result<VoiceProxyConnection> {
        let server_socket = UdpSocket::bind("0.0.0.0:0").await?;
        server_socket.connect(server_address).await?;

        Ok(
            VoiceProxyConnection {
                proxy_socket,
                secret,
                client_address,
                server_socket
            }
        )
    }
}
