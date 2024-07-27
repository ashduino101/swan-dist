use bytes::{Buf, Bytes};
use crate::server::packets::packet::PacketC2S;
use crate::server::packets::stage::Stage;
use crate::server::utils::{read_string, read_varint};
use crate::server::version::ProtocolVersion;

/// Unchanged since Netty rewrite
#[derive(Debug, Clone)]
pub struct HandshakeC2S {
    pub(crate) version: ProtocolVersion,
    pub(crate) address: String,
    pub(crate) port: u16,
    pub(crate) next_stage: Stage
}

impl PacketC2S for HandshakeC2S {
    /// The protocol version will still be Unknown here; this packet should set it
    fn decode(buf: &mut Bytes, _: ProtocolVersion) -> Self {
        HandshakeC2S {
            version: ProtocolVersion::from_id(read_varint(buf)),
            address: read_string(buf),
            port: buf.get_u16(),
            next_stage: Stage::from_id(read_varint(buf))
        }
    }

    fn id(_: ProtocolVersion) -> i32 {
        0  // always
    }
}
