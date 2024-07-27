use bytes::{BufMut, BytesMut};
use serde_derive::{Serialize, Deserialize};
use uuid::Uuid;
use crate::server::enums::{Arm, ChatVisibility};
use crate::server::version::ProtocolVersion;

#[derive(Debug, Clone)]
pub struct ClientInfo {
    pub(crate) lang: String,
    pub(crate) view_distance: u8,
    pub(crate) chat_visibility: ChatVisibility,
    pub(crate) chat_colors_enabled: bool,
    pub(crate) player_model_parts: u8,
    pub(crate) main_arm: Arm,
    pub(crate) filters_text: bool,
    pub(crate) allows_server_listing: bool
}

impl Default for ClientInfo {
    fn default() -> Self {
        ClientInfo {
            lang: "en_us".to_owned(),
            view_distance: 12,
            chat_visibility: ChatVisibility::Full,
            chat_colors_enabled: true,
            player_model_parts: 0x7f,
            main_arm: Arm::Right,
            filters_text: true,
            allows_server_listing: true
        }
    }
}


#[derive(Debug, Copy, Clone)]
pub struct Position {
    x: i32,
    y: i32,
    z: i32
}

impl Position {
    pub fn write_to(&self, buf: &mut BytesMut, v: ProtocolVersion) {
        if v >= ProtocolVersion::V1_14 {
            buf.put_u64((((self.x as u64) & 0x3FFFFFF) << 38) | (((self.z as u64) & 0x3FFFFFF) << 12) | ((self.y as u64) & 0xFFF));
        } else {
            buf.put_u64((((self.x as u64) & 0x3FFFFFF) << 38) | (((self.y as u64) & 0xFFF) << 26) | ((self.z as u64) & 0x3FFFFFF));
        }
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileProperty {
    pub(crate) name: String,
    pub(crate) value: String,
    pub(crate) signature: Option<String>
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub(crate) id: Uuid,
    pub(crate) name: String,
    pub(crate) properties: Vec<ProfileProperty>  // 1.19+ (759)
}
