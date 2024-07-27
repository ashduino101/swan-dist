use bytes::{Buf, Bytes};
use uuid::Uuid;
use crate::server::packets::packet::PacketC2S;
use crate::server::packets::stage::Stage;
use crate::server::utils::{read_string, read_uuid, read_varint};
use crate::server::version::ProtocolVersion;

#[derive(Debug, Clone)]
pub struct LoginHelloC2S {
    pub(crate) name: String,
    pub(crate) uuid: Option<Uuid>,
    /// Only in 1.19 to 1.19.2
    pub(crate) expires_at: Option<u64>,
    /// Only in 1.19 to 1.19.2
    pub(crate) public_key: Option<Bytes>,
    /// Only in 1.19 to 1.19.2
    pub(crate) signature: Option<Bytes>,
}

impl PacketC2S for LoginHelloC2S {
    fn decode(buf: &mut Bytes, v: ProtocolVersion) -> Self {
        let name = read_string(buf);
        let mut uuid = None;
        let mut expires_at = None;
        let mut public_key = None;
        let mut signature = None;
        if v >= ProtocolVersion::V1_19 {
            if v < ProtocolVersion::V1_19_3 {  // only present for a few versions
                let has_sig_data = buf.get_u8() != 0;
                if has_sig_data {
                    expires_at = Some(buf.get_u64());
                    let public_key_len = read_varint(buf) as usize;
                    public_key = Some(buf.slice(0..public_key_len));
                    buf.advance(public_key_len);
                    let signature_len = read_varint(buf) as usize;
                    signature = Some(buf.slice(0..signature_len));
                    buf.advance(signature_len);
                }
            }
            let has_uuid = if v < ProtocolVersion::V1_20_2 {
                buf.get_u8() != 0
            } else {
                true
            };
            if has_uuid {
                uuid = Some(read_uuid(buf));
            }
        }
        LoginHelloC2S {
            name,
            uuid,
            expires_at,
            public_key,
            signature
        }
    }

    fn id(_: ProtocolVersion) -> i32 {
        0
    }
}


#[derive(Debug, Clone)]
pub struct LoginKeyC2S {
    pub(crate) shared_secret: Bytes,
    /// Can be none in 1.19 to 1.19.2
    pub(crate) nonce: Option<Bytes>,
    /// Only in 1.19 to 1.19.2
    pub(crate) salt: Option<u64>,
    /// Only in 1.19 to 1.19.2
    pub(crate) message_signature: Option<Bytes>,
}

impl PacketC2S for LoginKeyC2S {
    fn decode(buf: &mut Bytes, v: ProtocolVersion) -> Self {
        let shared_secret_len = read_varint(buf) as usize;
        let shared_secret = buf.slice(0..shared_secret_len);
        let mut nonce = None;
        let mut salt = None;
        let mut message_signature = None;
        buf.advance(shared_secret_len);
        if v >= ProtocolVersion::V1_19 && v < ProtocolVersion::V1_19_3 {
            let has_nonce = buf.get_u8() != 0;
            if has_nonce {
                let nonce_len = read_varint(buf) as usize;
                nonce = Some(buf.slice(0..nonce_len));
                buf.advance(nonce_len);
            } else {
                salt = Some(buf.get_u64());
                let sig_len = read_varint(buf) as usize;
                message_signature = Some(buf.slice(0..sig_len));
                buf.advance(sig_len);
            }
        } else {
            let nonce_len = read_varint(buf) as usize;
            nonce = Some(buf.slice(0..nonce_len));
            buf.advance(nonce_len);
        }
        LoginKeyC2S {
            shared_secret,
            nonce,
            salt,
            message_signature
        }
    }

    fn id(_: ProtocolVersion) -> i32 {
        1
    }
}

#[derive(Debug, Clone)]
pub struct LoginQueryResponseC2S {
    pub(crate) query_id: i32,
    pub(crate) response: Bytes
}

impl PacketC2S for LoginQueryResponseC2S {
    fn decode(buf: &mut Bytes, _: ProtocolVersion) -> Self {
        LoginQueryResponseC2S {
            query_id: read_varint(buf),
            response: buf.slice(0..)
        }
    }

    fn id(_: ProtocolVersion) -> i32 {
        2
    }
}

#[derive(Debug, Clone)]
pub struct EnterConfigurationC2S {
}

impl PacketC2S for EnterConfigurationC2S {
    fn decode(_: &mut Bytes, _: ProtocolVersion) -> Self {
        EnterConfigurationC2S { }
    }

    fn id(_: ProtocolVersion) -> i32 {
        3
    }
}

#[derive(Debug, Clone)]
pub struct CookieResponseC2S {
    pub(crate) key: String,
    pub(crate) payload: Option<Bytes>
}

impl PacketC2S for CookieResponseC2S {
    fn decode(buf: &mut Bytes, _: ProtocolVersion) -> Self {
        CookieResponseC2S {
            key: read_string(buf),
            payload: if buf.get_u8() != 0 { Some(buf.slice(0..)) } else { None }
        }
    }

    fn id(_: ProtocolVersion) -> i32 {
        4
    }
}
