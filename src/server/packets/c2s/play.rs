use bytes::{Buf, Bytes};
use crate::server::packets::packet::PacketC2S;
use crate::server::utils::{read_string, read_varint};
use crate::server::version::ProtocolVersion;

#[derive(Debug, Clone)]
pub struct ChatC2S {
    pub(crate) message: String,
    pub(crate) timestamp: u64,
    pub(crate) salt: u64,
    pub(crate) signature: Option<Bytes>,
    pub(crate) message_count: i32,
    pub(crate) acknowledged: u32,  // u24
}

impl PacketC2S for ChatC2S {
    fn decode(buf: &mut Bytes, _: ProtocolVersion) -> Self {
        ChatC2S {
            message: read_string(buf),
            timestamp: buf.get_u64(),
            salt: buf.get_u64(),
            signature: if buf.get_u8() != 0 {
                let d = buf.slice(0..256);
                buf.advance(256);
                Some(d)
            } else { None },
            message_count: read_varint(buf),
            acknowledged: ((buf.get_u16() as u32) << 8) | (buf.get_u8() as u32)

        }
    }

    fn id(_: ProtocolVersion) -> i32 {
        6
    }
}
