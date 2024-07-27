use bytes::{Buf, Bytes};
use crate::server::packets::packet::PacketC2S;
use crate::server::version::ProtocolVersion;

/// Unchanged since 1.8
#[derive(Debug, Clone)]
pub struct StatusRequestC2S {

}

impl PacketC2S for StatusRequestC2S {
    fn decode(_: &mut Bytes, _: ProtocolVersion) -> StatusRequestC2S {
        StatusRequestC2S {}
    }

    fn id(_: ProtocolVersion) -> i32 {
        0  // always
    }
}

/// Unchanged since 1.8
#[derive(Debug, Clone)]
pub struct PingRequestC2S {
    pub payload: u64
}

impl PacketC2S for PingRequestC2S {
    fn decode(buf: &mut Bytes, _: ProtocolVersion) -> Self {
        PingRequestC2S {
            payload: buf.get_u64()
        }
    }

    fn id(_: ProtocolVersion) -> i32 {
        1  // always
    }
}
