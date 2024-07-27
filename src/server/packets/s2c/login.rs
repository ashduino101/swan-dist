use bytes::{Buf, BufMut, Bytes, BytesMut};
use serde_derive::Deserialize;
use uuid::Uuid;
use crate::server::common::Profile;
use crate::server::packets::packet::PacketS2C;
use crate::server::packets::stage::Stage;
use crate::server::text::TextComponent;
use crate::server::utils::{write_string, write_uuid, write_varint};
use crate::server::version::ProtocolVersion;

/// Unchanged since Netty rewrite
#[derive(Debug, Clone)]
pub struct LoginDisconnectS2C {
    pub(crate) reason: TextComponent
}

impl PacketS2C for LoginDisconnectS2C {
    fn encode(&self, _: ProtocolVersion) -> BytesMut {
        let mut buf = BytesMut::new();
        write_string(&mut buf, &serde_json::ser::to_string(&self.reason).unwrap());
        buf
    }

    fn id(&self, _: ProtocolVersion) -> i32 {
        0
    }
}

#[derive(Debug, Clone)]
pub struct LoginHelloS2C {
    pub(crate) server_id: String,  // max 20
    pub(crate) public_key: Bytes,
    pub(crate) nonce: Bytes,
    pub(crate) needs_authentication: bool  // Added in 1.20.5pre1
}

impl PacketS2C for LoginHelloS2C {
    fn encode(&self, v: ProtocolVersion) -> BytesMut {
        let mut buf = BytesMut::new();
        write_string(&mut buf, &self.server_id);
        write_varint(&mut buf, self.public_key.len() as i32);
        buf.put(&self.public_key[..]);
        write_varint(&mut buf, self.nonce.len() as i32);
        buf.put(&self.nonce[..]);
        if v >= ProtocolVersion::V1_20_5 {  // v1.20.5pre1+
            buf.put_u8(if self.needs_authentication { 1 } else { 0 });
        }
        buf
    }

    fn id(&self, _: ProtocolVersion) -> i32 {
        1
    }
}

#[derive(Debug, Clone)]
pub struct LoginSuccessS2C {
    pub(crate) profile: Profile,
    pub(crate) strict_error_handling: bool,  // 1.20.5+ (766)
}

impl PacketS2C for LoginSuccessS2C {
    fn encode(&self, v: ProtocolVersion) -> BytesMut {
        let mut buf = BytesMut::new();
        // Before 20w12a, UUIDs were encoded as strings with hyphens
        if v < ProtocolVersion::V20w12a {
            write_string(&mut buf, &self.profile.id.as_hyphenated().to_string());
        } else {
            write_uuid(&mut buf, self.profile.id);
        }
        write_string(&mut buf, &self.profile.name);
        // 1.19 added properties
        if v > ProtocolVersion::V1_19 {
            write_varint(&mut buf, self.profile.properties.len() as i32);
            for prop in &self.profile.properties {
                write_string(&mut buf, &prop.name);
                write_string(&mut buf, &prop.value);
                buf.put_u8(if prop.signature.is_some() { 1 } else { 0 });
                if let Some(signature) = &prop.signature {
                    write_string(&mut buf, &signature);
                }
            }
        }
        // 1.20.5 added strict error handling (but it's probably temporary!)
        if v >= ProtocolVersion::V1_20_5 {
            buf.put_u8(if self.strict_error_handling { 1 } else { 0 });
        }
        buf
    }

    fn id(&self, _: ProtocolVersion) -> i32 {
        2
    }
}

/// Unchanged since Netty rewrite
#[derive(Debug, Clone)]
pub struct LoginCompressionS2C {
    pub(crate) threshold: i32
}

impl PacketS2C for LoginCompressionS2C {
    fn encode(&self, _: ProtocolVersion) -> BytesMut {
        let mut buf = BytesMut::new();
        write_varint(&mut buf, self.threshold);
        buf
    }

    fn id(&self, _: ProtocolVersion) -> i32 {
        3
    }
}

#[derive(Debug, Clone)]
pub struct LoginQueryRequestS2C {
    pub(crate) query_id: i32,
    pub(crate) channel: String,
    pub(crate) data: Bytes
}

impl PacketS2C for LoginQueryRequestS2C {
    fn encode(&self, _: ProtocolVersion) -> BytesMut {
        let mut buf = BytesMut::new();
        write_varint(&mut buf, self.query_id);
        write_string(&mut buf, &self.channel);
        buf.put(self.data.clone());
        buf
    }

    fn id(&self, _: ProtocolVersion) -> i32 {
        4
    }
}

#[derive(Debug, Clone)]
pub struct LoginCookieRequestS2C {
    pub(crate) key: String
}

impl PacketS2C for LoginCookieRequestS2C {
    fn encode(&self, _: ProtocolVersion) -> BytesMut {
        let mut buf = BytesMut::new();
        write_string(&mut buf, &self.key);
        buf
    }

    fn id(&self, _: ProtocolVersion) -> i32 {
        5
    }
}
