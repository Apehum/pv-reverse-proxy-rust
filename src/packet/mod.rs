mod ping;

use std::io;
use std::io::{Cursor, ErrorKind, Read};

use byteorder::{BigEndian, ReadBytesExt};
use uuid::Uuid;
use crate::packet::ping::PingPacket;

#[derive(Debug)]
pub struct VoicePacketWrapper {
    pub secret: Uuid,
    pub packet: Option<VoicePacket>
}

#[derive(Debug)]
pub enum VoicePacket {
    Ping(PingPacket)
}

impl VoicePacket {
    fn deserialize(packet_type: u8, cursor: &mut Cursor<&[u8]>) -> Result<Self, io::Error> {
        let packet = match packet_type {
            1 => VoicePacket::Ping(PingPacket::read_from(cursor)?),

            _ => return Err(io::Error::new(ErrorKind::InvalidData, "Invalid packet type"))
        };
        
        Ok(packet)
    }
}

const MAGIC_NUMBER: i32 = 0x4e9004e9;

impl TryFrom<&[u8]> for VoicePacketWrapper {
    type Error = io::Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let mut cursor = Cursor::new(value);

        let magic_number = cursor.read_i32::<BigEndian>()?;
        if magic_number != MAGIC_NUMBER {
            return Err(Self::Error::new(ErrorKind::InvalidData, "Invalid magic number"));
        }

        let packet_type = cursor.read_u8()?;
        let mut secret_bytes = [0; 16];
        cursor.read_exact(&mut secret_bytes)?;

        let secret = Uuid::from_bytes(secret_bytes);

        cursor.read_u64::<BigEndian>()?;

        let packet = match VoicePacket::deserialize(packet_type, &mut cursor) {
            Ok(packet) => Some(packet),
            Err(_) => None
        };

        Ok(
            VoicePacketWrapper {
                secret,
                packet,
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use std::io::{BufWriter, Write};

    use byteorder::{BigEndian, WriteBytesExt};

    use super::*;

    fn create_valid_packet() -> Result<VoicePacketWrapper, io::Error> {
        let mut buffer = Vec::with_capacity(16);

        {
            let mut stream = BufWriter::new(&mut buffer);
            stream.write_i32::<BigEndian>(0x4e9004e9)?;
            stream.write_u8(1u8)?;

            let secret = Uuid::nil();
            stream.write_all(secret.as_bytes())?;

            stream.write_u64::<BigEndian>(0u64)?;

            stream.write_u64::<BigEndian>(0u64)?;
        }

        VoicePacketWrapper::try_from(&buffer[..])
    }

    fn create_unsupported_packet() -> Result<VoicePacketWrapper, io::Error> {
        let mut buffer = Vec::with_capacity(16);

        {
            let mut stream = BufWriter::new(&mut buffer);
            stream.write_i32::<BigEndian>(0x4e9004e9)?;
            stream.write_u8(2u8)?;

            let secret = Uuid::nil();
            stream.write_all(secret.as_bytes())?;

            stream.write_u64::<BigEndian>(0u64)?;
        }

        VoicePacketWrapper::try_from(&buffer[..])
    }

    fn create_malformed_packet() -> Result<VoicePacketWrapper, io::Error> {
        let mut buffer = Vec::with_capacity(16);

        {
            let mut stream = BufWriter::new(&mut buffer);
            stream.write_i32::<BigEndian>(0x4444)?;
        }

        VoicePacketWrapper::try_from(&buffer[..])
    }
    
    #[test]
    fn test_valid_packet() {
        let packet = create_valid_packet();
        assert!(packet.is_ok_and(|wrapper| wrapper.packet.is_some()));
    }

    #[test]
    fn test_unsupported_packet() {
        let packet = create_unsupported_packet();
        assert!(packet.is_ok_and(|wrapper| wrapper.packet.is_none()));
    }

    #[test]
    fn test_malformed_packet() {
        let packet = create_malformed_packet();
        assert!(packet.is_err());
    }
}
