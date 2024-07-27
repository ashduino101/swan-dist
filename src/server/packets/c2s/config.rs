use bytes::{Buf, Bytes};
use crate::server::common::ClientInfo;
use crate::server::enums::{Arm, ChatVisibility};
use crate::server::packets::c2s::config::ResourcePackStatus::{Accepted, Declined, Failed, Success};
use crate::server::packets::packet::PacketC2S;
use crate::server::utils::{read_string, read_varint};
use crate::server::version::ProtocolVersion;

#[derive(Debug, Clone)]
pub struct ClientInfoC2S {
    pub(crate) client_info: ClientInfo
}

impl PacketC2S for ClientInfoC2S {
    fn decode(buf: &mut Bytes, _: ProtocolVersion) -> Self {
        ClientInfoC2S {
            client_info: ClientInfo {
                lang: read_string(buf),
                view_distance: buf.get_u8(),
                chat_visibility: ChatVisibility::from_i32(read_varint(buf)),
                chat_colors_enabled: buf.get_u8() != 0,
                player_model_parts: buf.get_u8(),
                main_arm: Arm::from_i32(read_varint(buf)),
                filters_text: buf.get_u8() != 0,
                allows_server_listing: buf.get_u8() != 0
            }
        }
    }

    fn id(_: ProtocolVersion) -> i32 {
        0
    }
}


#[derive(Debug, Clone)]
pub struct CookieResponseC2S {
    pub(crate) key: String,
    pub(crate) payload: Option<Bytes>,
}

impl PacketC2S for CookieResponseC2S {
    fn decode(buf: &mut Bytes, _: ProtocolVersion) -> Self {
        CookieResponseC2S {
            key: read_string(buf),
            payload: if buf.get_u8() != 0 { Some({
                let p = buf.clone();
                buf.advance(p.len());
                p
            }) } else { None }
        }
    }

    fn id(_: ProtocolVersion) -> i32 {
        1
    }
}


#[derive(Debug, Clone)]
pub struct CustomPayloadC2S {
    pub(crate) key: String,
    pub(crate) payload: Bytes
}

impl PacketC2S for CustomPayloadC2S {
    fn decode(buf: &mut Bytes, _: ProtocolVersion) -> Self {
        CustomPayloadC2S {
            key: read_string(buf),
            payload: {
                let p = buf.clone();
                buf.advance(p.len());
                p
            }
        }
    }

    fn id(_: ProtocolVersion) -> i32 {
        2
    }
}


/// Transitions stage into Play
#[derive(Debug, Clone)]
pub struct ReadyC2S { }

impl PacketC2S for ReadyC2S {
    fn decode(_: &mut Bytes, _: ProtocolVersion) -> Self {
        ReadyC2S { }
    }

    fn id(_: ProtocolVersion) -> i32 {
        3
    }
}


#[derive(Debug, Clone)]
pub struct KeepAliveC2S {
    pub(crate) id: u64
}

impl PacketC2S for KeepAliveC2S {
    fn decode(buf: &mut Bytes, _: ProtocolVersion) -> Self {
        KeepAliveC2S {
            id: buf.get_u64()
        }
    }

    fn id(_: ProtocolVersion) -> i32 {
        4
    }
}


#[derive(Debug, Clone)]
pub struct PongC2S {
    pub(crate) id: u32
}

impl PacketC2S for PongC2S {
    fn decode(buf: &mut Bytes, _: ProtocolVersion) -> Self {
        PongC2S {
            id: buf.get_u32()
        }
    }

    fn id(_: ProtocolVersion) -> i32 {
        5
    }
}


#[derive(Debug, Clone)]
pub enum ResourcePackStatus {
    Success,
    Declined,
    Failed,
    Accepted
}

impl ResourcePackStatus {
    pub fn from_i32(val: i32) -> ResourcePackStatus {
        match val {
            0 => Success,
            1 => Declined,
            3 => Accepted,
            _ => Failed,  // 2 or default
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResourcePackStatusC2S {
    pub(crate) status: ResourcePackStatus
}

impl PacketC2S for ResourcePackStatusC2S {
    fn decode(buf: &mut Bytes, _: ProtocolVersion) -> Self {
        ResourcePackStatusC2S {
            status: ResourcePackStatus::from_i32(read_varint(buf))
        }
    }

    fn id(_: ProtocolVersion) -> i32 {
        6
    }
}


#[derive(Debug, Clone)]
pub struct VersionedIdentifier {
    pub(crate) namespace: String,
    pub(crate) id: String,
    pub(crate) version: String,
}

#[derive(Debug, Clone)]
pub struct SelectKnownPacksC2S {
    pub(crate) known_packs: Vec<VersionedIdentifier>
}

impl PacketC2S for SelectKnownPacksC2S {
    fn decode(buf: &mut Bytes, _: ProtocolVersion) -> Self {
        let num_packs = read_varint(buf);
        let mut packs = Vec::new();
        for _ in 0..num_packs {
            packs.push(VersionedIdentifier {
                namespace: read_string(buf),
                id: read_string(buf),
                version: read_string(buf),
            });
        }
        SelectKnownPacksC2S { known_packs: packs }
    }

    fn id(_: ProtocolVersion) -> i32 {
        7
    }
}
