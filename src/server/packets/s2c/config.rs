use std::collections::HashMap;
use base64::Engine;
use bytes::{BufMut, Bytes, BytesMut};
use crate::server::packets::c2s::config::VersionedIdentifier;
use crate::server::packets::packet::PacketS2C;
use crate::server::text::TextComponent;
use crate::server::utils::{write_string, write_varint};
use crate::server::version::ProtocolVersion;
use crate::Tag;

#[derive(Debug, Clone)]
pub struct CookieRequestS2C {
    pub(crate) key: String,
}

impl PacketS2C for CookieRequestS2C {
    fn encode(&self, _: ProtocolVersion) -> BytesMut {
        let mut buf = BytesMut::new();
        write_string(&mut buf, &self.key);
        buf
    }

    fn id(&self, _: ProtocolVersion) -> i32 {
        0
    }
}


#[derive(Debug, Clone)]
pub struct CustomPayloadS2C {
    pub(crate) key: String,
    pub(crate) payload: Bytes,
}

impl PacketS2C for CustomPayloadS2C {
    fn encode(&self, _: ProtocolVersion) -> BytesMut {
        let mut buf = BytesMut::new();
        write_string(&mut buf, &self.key);
        buf.put(self.payload.clone());
        buf
    }

    fn id(&self, _: ProtocolVersion) -> i32 {
        1
    }
}


#[derive(Debug, Clone)]
pub struct ConfigDisconnectS2C {
    pub(crate) reason: TextComponent
}

impl PacketS2C for ConfigDisconnectS2C {
    fn encode(&self, _: ProtocolVersion) -> BytesMut {
        let mut buf = BytesMut::new();
        write_string(&mut buf, &serde_json::ser::to_string(&self.reason).unwrap());
        buf
    }

    fn id(&self, _: ProtocolVersion) -> i32 {
        2
    }
}


#[derive(Debug, Clone)]
pub struct ReadyS2C { }

impl PacketS2C for ReadyS2C {
    fn encode(&self, _: ProtocolVersion) -> BytesMut {
        BytesMut::new()
    }

    fn id(&self, _: ProtocolVersion) -> i32 {
        3
    }
}


#[derive(Debug, Clone)]
pub struct KeepAliveS2C {
    pub(crate) payload: u64
}

impl PacketS2C for KeepAliveS2C {
    fn encode(&self, _: ProtocolVersion) -> BytesMut {
        let mut buf = BytesMut::new();
        buf.put_u64(self.payload);
        buf
    }

    fn id(&self, _: ProtocolVersion) -> i32 {
        4
    }
}


#[derive(Debug, Clone)]
pub struct PingS2C {
    pub(crate) parameter: u32
}

impl PacketS2C for PingS2C {
    fn encode(&self, _: ProtocolVersion) -> BytesMut {
        let mut buf = BytesMut::new();
        buf.put_u32(self.parameter);
        buf
    }

    fn id(&self, _: ProtocolVersion) -> i32 {
        5
    }
}


#[derive(Debug, Clone)]
pub struct ResetChatS2C { }

impl PacketS2C for ResetChatS2C {
    fn encode(&self, _: ProtocolVersion) -> BytesMut {
        BytesMut::new()
    }

    fn id(&self, _: ProtocolVersion) -> i32 {
        6
    }
}


#[derive(Debug, Clone)]
pub struct RegistryEntry {
    pub(crate) id: String,
    pub(crate) data: Option<Tag>
}

#[derive(Debug, Clone)]
pub struct DynamicRegistriesS2C {
    pub(crate) registry_id: String,
    pub(crate) entries: Vec<RegistryEntry>
}

impl PacketS2C for DynamicRegistriesS2C {
    fn encode(&self, v: ProtocolVersion) -> BytesMut {
        let mut buf = BytesMut::new();
        write_string(&mut buf, &self.registry_id);
        write_varint(&mut buf, self.entries.len() as i32);
        for entry in &self.entries {
            write_string(&mut buf, &entry.id);
            buf.put_u8(if entry.data.is_some() { 1 } else { 0 });
            if let Some(data) = &entry.data {
                data.serialize(&mut buf, v >= ProtocolVersion::V1_20_2);
            }
        }
        buf
    }

    fn id(&self, _: ProtocolVersion) -> i32 {
        7
    }
}


#[derive(Debug, Clone)]
pub struct RemoveResourcePackS2C {
    pub(crate) id: Option<String>
}

impl PacketS2C for RemoveResourcePackS2C {
    fn encode(&self, _: ProtocolVersion) -> BytesMut {
        let mut buf = BytesMut::new();
        buf.put_u8(if self.id.is_some() { 1 } else { 0 });
        if let Some(id) = &self.id {
            write_string(&mut buf, id);
        }
        buf
    }

    fn id(&self, _: ProtocolVersion) -> i32 {
        8
    }
}


#[derive(Debug, Clone)]
pub struct SendResourcePackS2C {
    pub(crate) url: String,
    pub(crate) hash: String,
    pub(crate) required: bool,
    pub(crate) prompt: Option<String>
}

impl PacketS2C for SendResourcePackS2C {
    fn encode(&self, _: ProtocolVersion) -> BytesMut {
        let mut buf = BytesMut::new();
        write_string(&mut buf, &self.url);
        write_string(&mut buf, &self.hash);
        buf.put_u8(if self.required { 1 } else { 0 });
        buf.put_u8(if self.prompt.is_some() { 1 } else { 0 });
        if let Some(prompt) = &self.prompt {
            write_string(&mut buf, prompt);
        }
        buf
    }

    fn id(&self, _: ProtocolVersion) -> i32 {
        9
    }
}


#[derive(Debug, Clone)]
pub struct StoreCookieS2C {
    pub(crate) key: String,
    pub(crate) payload: Bytes,
}

impl PacketS2C for StoreCookieS2C {
    fn encode(&self, _: ProtocolVersion) -> BytesMut {
        let mut buf = BytesMut::new();
        write_string(&mut buf, &self.key);
        buf.put(self.payload.clone());
        buf
    }

    fn id(&self, _: ProtocolVersion) -> i32 {
        10
    }
}


#[derive(Debug, Clone)]
pub struct ServerTransferS2C {
    pub(crate) host: String,
    pub(crate) port: u16,
}

impl PacketS2C for ServerTransferS2C {
    fn encode(&self, _: ProtocolVersion) -> BytesMut {
        let mut buf = BytesMut::new();
        write_string(&mut buf, &self.host);
        write_varint(&mut buf, self.port as i32);
        buf
    }

    fn id(&self, _: ProtocolVersion) -> i32 {
        11
    }
}


#[derive(Debug, Clone)]
pub struct FeaturesS2C {
    pub(crate) features: Vec<String>,
}

impl PacketS2C for FeaturesS2C {
    fn encode(&self, _: ProtocolVersion) -> BytesMut {
        let mut buf = BytesMut::new();
        write_varint(&mut buf, self.features.len() as i32);
        for feature in &self.features {
            write_string(&mut buf, feature);
        }
        buf
    }

    fn id(&self, _: ProtocolVersion) -> i32 {
        12
    }
}


#[derive(Debug, Clone)]
pub struct RegistryTag {
    pub(crate) name: String,
    pub(crate) entries: Vec<i32>,
}

#[derive(Debug, Clone)]
pub struct SyncTagsS2C {
    pub(crate) tags: HashMap<String, Vec<RegistryTag>>,
}

impl PacketS2C for SyncTagsS2C {
    fn encode(&self, _: ProtocolVersion) -> BytesMut {
        let mut buf = BytesMut::new();
        write_varint(&mut buf, self.tags.len() as i32);
        for (registry, tags) in &self.tags {
            write_string(&mut buf, registry);
            write_varint(&mut buf, tags.len() as i32);
            for tag in tags {
                write_string(&mut buf, &tag.name);
                write_varint(&mut buf, tag.entries.len() as i32);
                for entry in &tag.entries {
                    write_varint(&mut buf, *entry);
                }
            }
        }
        buf
    }

    fn id(&self, _: ProtocolVersion) -> i32 {
        13
    }
}


#[derive(Debug, Clone)]
pub struct SelectKnownPacksS2C {
    pub(crate) known_packs: Vec<VersionedIdentifier>,
}

impl PacketS2C for SelectKnownPacksS2C {
    fn encode(&self, _: ProtocolVersion) -> BytesMut {
        let mut buf = BytesMut::new();
        write_varint(&mut buf, self.known_packs.len() as i32);
        for pack in &self.known_packs {
            write_string(&mut buf, &pack.namespace);
            write_string(&mut buf, &pack.id);
            write_string(&mut buf, &pack.version);
        }
        buf
    }

    fn id(&self, _: ProtocolVersion) -> i32 {
        14
    }
}


#[derive(Debug, Clone)]
pub struct ReportDetailsS2C {
    pub(crate) details: HashMap<String, String>,
}

impl PacketS2C for ReportDetailsS2C {
    fn encode(&self, _: ProtocolVersion) -> BytesMut {
        let mut buf = BytesMut::new();
        write_varint(&mut buf, self.details.len() as i32);
        for (title, description) in &self.details {
            write_string(&mut buf, title);
            write_string(&mut buf, description);
        }
        buf
    }

    fn id(&self, _: ProtocolVersion) -> i32 {
        15
    }
}


#[derive(Debug, Clone)]
pub enum LinkLabel {
    BugReport,
    CommunityGuidelines,
    Support,
    Status,
    Feedback,
    Community,
    Website,
    Forums,
    News,
    Announcements,
    Custom(TextComponent)
}

impl LinkLabel {
    pub fn get_id(&self) -> i32 {
        match self {
            LinkLabel::BugReport => 0,
            LinkLabel::CommunityGuidelines => 1,
            LinkLabel::Support => 2,
            LinkLabel::Status => 3,
            LinkLabel::Feedback => 4,
            LinkLabel::Community => 5,
            LinkLabel::Website => 6,
            LinkLabel::Forums => 7,
            LinkLabel::News => 8,
            LinkLabel::Announcements => 9,
            _ => -1  // Custom text should be handled separately
        }
    }

    pub fn write_to(&self, buf: &mut BytesMut, v: ProtocolVersion) {
        if let LinkLabel::Custom(text) = self {
            buf.put_u8(0);
            text.to_nbt().serialize(buf, v >= ProtocolVersion::V1_20_2);
        } else {
            buf.put_u8(1);
            write_varint(buf, self.get_id());
        }
    }
}


#[derive(Debug, Clone)]
pub struct Link {
    pub(crate) label: LinkLabel,
    pub(crate) url: String

}

#[derive(Debug, Clone)]
pub struct LinksS2C {
    pub(crate) links: Vec<Link>,
}

impl PacketS2C for LinksS2C {
    fn encode(&self, v: ProtocolVersion) -> BytesMut {
        let mut buf = BytesMut::new();
        write_varint(&mut buf, self.links.len() as i32);
        for link in &self.links {
            link.label.write_to(&mut buf, v);
            write_string(&mut buf, &link.url);
        }
        buf
    }

    fn id(&self, _: ProtocolVersion) -> i32 {
        16
    }
}
