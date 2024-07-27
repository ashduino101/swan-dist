use bytes::{BufMut, BytesMut};
use crate::server::packets::packet::PacketS2C;
use crate::server::utils::write_string;
use crate::server::version::ProtocolVersion;

/// Unchanged since 1.8
#[derive(Debug, Clone)]
pub struct StatusResponseS2C {
    pub response: String
}

impl StatusResponseS2C {
    pub fn new(response: String) -> StatusResponseS2C {
        StatusResponseS2C { response }
    }
}

impl PacketS2C for StatusResponseS2C {
    fn encode(&self, _: ProtocolVersion) -> BytesMut {
        let mut buf = BytesMut::new();
        write_string(&mut buf, &self.response);
        buf
    }

    fn id(&self, _: ProtocolVersion) -> i32 {
        0  // always
    }
}

/// Unchanged since 1.8
#[derive(Debug, Clone)]
pub struct PingResponseS2C {
    pub payload: u64
}

impl PingResponseS2C {
    pub fn new(payload: u64) -> PingResponseS2C {
        PingResponseS2C { payload }
    }
}

impl PacketS2C for PingResponseS2C {
    fn encode(&self, v: ProtocolVersion) -> BytesMut {
        let mut buf = BytesMut::new();
        buf.put_u64(self.payload);
        buf
    }

    fn id(&self, _: ProtocolVersion) -> i32 {
        1  // always
    }
}
