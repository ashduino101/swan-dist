use std::fmt::Debug;
use bytes::{Bytes, BytesMut};
use crate::server::version::ProtocolVersion;

pub trait PacketS2C : Debug {
    fn encode(&self, v: ProtocolVersion) -> BytesMut;
    fn id(&self, v: ProtocolVersion) -> i32;
}

pub trait PacketC2S {
    fn decode(buf: &mut Bytes, v: ProtocolVersion) -> Self;
    fn id(v: ProtocolVersion) -> i32;
}
