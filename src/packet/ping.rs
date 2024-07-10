use std::io;
use std::io::{Cursor, ErrorKind, Read};

use byteorder::{BigEndian, ReadBytesExt};

#[derive(Debug)]
pub struct PingPacket {
    timestamp: u64,
    server_ip: Option<String>,
    server_port: Option<u16>,
}

impl PingPacket {
    pub fn read_from(cursor: &mut Cursor<&[u8]>) -> Result<Self, io::Error> {
        let timestamp = cursor.read_u64::<BigEndian>()?;
        if cursor.is_empty() {
            return Ok(
                PingPacket {
                    timestamp: 0,
                    server_ip: None,
                    server_port: None,
                }
            );
        }

        let server_ip_length = cursor.read_i16::<BigEndian>()?;
        let mut server_ip_bytes: Vec<u8> = Vec::new();
        server_ip_bytes.resize(server_ip_length as usize, 0);
        cursor.read_exact(&mut server_ip_bytes)?;
        
        let server_ip = match String::from_utf8(server_ip_bytes) {
            Ok(server_ip) => server_ip,
            Err(err) => return Err(io::Error::new(ErrorKind::InvalidData, err.to_string()))
        };

        let server_port = cursor.read_u16::<BigEndian>()?;

        Ok(
            PingPacket {
                timestamp,
                server_ip: Some(server_ip),
                server_port: Some(server_port),
            }
        )
    }   
}