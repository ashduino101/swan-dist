use std::collections::HashMap;
use bytes::{BufMut, Bytes, BytesMut};
use crate::chunk::Chunk;
use crate::server::common::Position;
use crate::server::packets::packet::PacketS2C;
use crate::server::text::TextComponent;
use crate::server::utils::{write_string, write_varint};
use crate::server::version::ProtocolVersion;
use crate::Tag;

#[derive(Debug, Clone)]
pub struct PlayDisconnectS2C {
    pub(crate) reason: TextComponent
}


impl PacketS2C for PlayDisconnectS2C {
    fn encode(&self, v: ProtocolVersion) -> BytesMut {
        let mut buf = BytesMut::new();
        self.reason.to_nbt().serialize(&mut buf, v >= ProtocolVersion::V1_20_2);
        buf
    }

    fn id(&self, _: ProtocolVersion) -> i32 {
        0x1d
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

    fn id(&self, v: ProtocolVersion) -> i32 {
        if v >= ProtocolVersion::V1_20_6 {
            0x26
        } else if v >= ProtocolVersion::V1_20_2 {
            0x24
        } else if v >= ProtocolVersion::V1_19_4 {
            0x23
        } else if v >= ProtocolVersion::V1_19_3 {
            0x1f
        } else if v >= ProtocolVersion::V1_19_2 {
            0x20
        } else if v >= ProtocolVersion::V1_19 {
            0x1e
        } else if v >= ProtocolVersion::V1_17 {
            0x21
        } else if v >= ProtocolVersion::V1_16_2 {
            0x1f
        } else if v >= ProtocolVersion::V1_16_1 {
            0x20
        } else if v >= ProtocolVersion::V1_15 {
            0x21
        } else if v >= ProtocolVersion::V1_14 {
            0x20
        } else {  // 1.13.2
            0x21
        }
    }
}

#[derive(Debug, Clone)]
pub enum EventType {
    NoRespawnBlock,
    RainStarted,
    RainStopped,
    GameModeChanged,
    GameWon,
    DemoMessageShown,
    ProjectileHitPlayer,
    RainGradientChanged,
    ThunderGradientChanged,
    PufferfishSting,
    ElderGuardianEffect,
    ImmediateRespawn,
    LimitedCraftingToggled,
    InitialChunksComing
}

impl EventType {
    pub fn get_id(&self) -> u8 {
        match self {
            EventType::NoRespawnBlock => 0,
            EventType::RainStarted => 1,
            EventType::RainStopped => 2,
            EventType::GameModeChanged => 3,
            EventType::GameWon => 4,
            EventType::DemoMessageShown => 5,
            EventType::ProjectileHitPlayer => 6,
            EventType::RainGradientChanged => 7,
            EventType::ThunderGradientChanged => 8,
            EventType::PufferfishSting => 9,
            EventType::ElderGuardianEffect => 10,
            EventType::ImmediateRespawn => 11,
            EventType::LimitedCraftingToggled => 12,
            EventType::InitialChunksComing => 13
        }
    }
}

#[derive(Debug, Clone)]
pub struct GameEventS2C {
    pub(crate) event: EventType,
    pub(crate) value: f32
}

impl PacketS2C for GameEventS2C {
    fn encode(&self, v: ProtocolVersion) -> BytesMut {
        let mut buf = BytesMut::new();
        buf.put_u8(self.event.get_id());
        buf.put_f32(self.value);
        buf
    }

    fn id(&self, v: ProtocolVersion) -> i32 {
        if v >= ProtocolVersion::V1_20_6 {
            0x22
        } else if v >= ProtocolVersion::V1_20_2 {
            0x20
        } else if v >= ProtocolVersion::V1_19_4 {
            0x1f
        } else if v >= ProtocolVersion::V1_19_3 {
            0x1c
        } else if v >= ProtocolVersion::V1_19_2 {
            0x1d
        } else if v >= ProtocolVersion::V1_19 {
            0x1b
        } else if v >= ProtocolVersion::V1_17 {
            0x1e
        } else if v >= ProtocolVersion::V1_16_2 {
            0x1d
        } else if v >= ProtocolVersion::V1_16_1 {
            0x1e
        } else if v >= ProtocolVersion::V1_15 {
            0x1f
        } else {  // 1.13.2
            0x1e
        }
    }
}

#[derive(Debug, Clone)]
pub struct JoinGameS2C {
    pub(crate) entity_id: i32,
    pub(crate) is_hardcore: bool,
    pub(crate) gamemode: u8,
    pub(crate) previous_gamemode: i8,
    pub(crate) dimensions: Vec<String>,
    /// For versions before 1.20.2
    pub(crate) registry_codec: Tag,
    /// For versions 1.16 to 1.19
    pub(crate) legacy_dimension_nbt: Tag,
    pub(crate) max_players: i32,
    pub(crate) view_distance: i32,
    pub(crate) simulation_distance: i32,
    pub(crate) reduced_debug_info: bool,
    pub(crate) enable_respawn_screen: bool,
    pub(crate) do_limited_crafting: bool,
    /// For versions before 1.20.6
    pub(crate) legacy_dimension_type: String,
    /// For versions before 1.16
    /// -1: Nether, 0: Overworld, 1: End
    pub(crate) legacy_dimension: i32,
    /// For versions before 1.16
    pub(crate) legacy_level_type: String,
    pub(crate) dimension_type: i32,
    pub(crate) dimension_name: String,
    pub(crate) hashed_seed: u64,
    pub(crate) is_debug: bool,
    pub(crate) is_flat: bool,
    pub(crate) death_dimension: Option<String>,
    pub(crate) death_location: Option<Position>,
    pub(crate) portal_cooldown: i32,
    pub(crate) enforces_secure_chat: bool,
}

impl PacketS2C for JoinGameS2C {
    fn encode(&self, v: ProtocolVersion) -> BytesMut {
        let mut buf = BytesMut::new();
        buf.put_i32(self.entity_id);
        if v >= ProtocolVersion::V1_20_6 {
            buf.put_u8(if self.is_hardcore { 1 } else { 0 });
            write_varint(&mut buf, self.dimensions.len() as i32);
            for dim in &self.dimensions {
                write_string(&mut buf, dim);
            }
            write_varint(&mut buf, self.max_players);
            write_varint(&mut buf, self.view_distance);
            write_varint(&mut buf, self.simulation_distance);
            buf.put_u8(if self.reduced_debug_info { 1 } else { 0 });
            buf.put_u8(if self.enable_respawn_screen { 1 } else { 0 });
            buf.put_u8(if self.do_limited_crafting { 1 } else { 0 });
            write_varint(&mut buf, self.dimension_type);
            write_string(&mut buf, &self.dimension_name);
            buf.put_u64(self.hashed_seed);
            buf.put_u8(self.gamemode);
            buf.put_i8(self.previous_gamemode);
            buf.put_u8(if self.is_debug { 1 } else { 0 });
            buf.put_u8(if self.is_flat { 1 } else { 0 });
            buf.put_u8(if self.death_location.is_some() { 1 } else { 0 });
            if let Some(death_location) = self.death_location {
                write_string(&mut buf, &self.death_dimension.clone().unwrap());
                death_location.write_to(&mut buf, v);
            }
            write_varint(&mut buf, self.portal_cooldown);
            buf.put_u8(if self.enforces_secure_chat { 1 } else { 0 });
            buf
        } else if v >= ProtocolVersion::V1_20_2 {
            buf.put_u8(if self.is_hardcore { 1 } else { 0 });
            write_varint(&mut buf, self.dimensions.len() as i32);
            for dim in &self.dimensions {
                write_string(&mut buf, dim);
            }
            write_varint(&mut buf, self.max_players);
            write_varint(&mut buf, self.view_distance);
            write_varint(&mut buf, self.simulation_distance);
            buf.put_u8(if self.reduced_debug_info { 1 } else { 0 });
            buf.put_u8(if self.enable_respawn_screen { 1 } else { 0 });
            buf.put_u8(if self.do_limited_crafting { 1 } else { 0 });
            write_string(&mut buf, &self.legacy_dimension_type);
            write_string(&mut buf, &self.dimension_name);
            buf.put_u64(self.hashed_seed);
            buf.put_u8(self.gamemode);
            buf.put_i8(self.previous_gamemode);
            buf.put_u8(if self.is_debug { 1 } else { 0 });
            buf.put_u8(if self.is_flat { 1 } else { 0 });
            buf.put_u8(if self.death_location.is_some() { 1 } else { 0 });
            if let Some(death_location) = self.death_location {
                write_string(&mut buf, &self.death_dimension.clone().unwrap());
                death_location.write_to(&mut buf, v);
            }
            write_varint(&mut buf, self.portal_cooldown);
            buf
        } else if v >= ProtocolVersion::V1_19 {
            buf.put_u8(if self.is_hardcore { 1 } else { 0 });
            buf.put_u8(self.gamemode);
            buf.put_i8(self.previous_gamemode);
            write_varint(&mut buf, self.dimensions.len() as i32);
            for dim in &self.dimensions {
                write_string(&mut buf, dim);
            }
            self.registry_codec.serialize(&mut buf, false);
            write_string(&mut buf, &self.legacy_dimension_type);
            write_string(&mut buf, &self.dimension_name);
            buf.put_u64(self.hashed_seed);
            write_varint(&mut buf, self.max_players);
            write_varint(&mut buf, self.view_distance);
            write_varint(&mut buf, self.simulation_distance);
            buf.put_u8(if self.reduced_debug_info { 1 } else { 0 });
            buf.put_u8(if self.enable_respawn_screen { 1 } else { 0 });
            buf.put_u8(if self.is_debug { 1 } else { 0 });
            buf.put_u8(if self.is_flat { 1 } else { 0 });
            buf.put_u8(if self.death_location.is_some() { 1 } else { 0 });
            if let Some(death_location) = self.death_location {
                write_string(&mut buf, &self.death_dimension.clone().unwrap());
                death_location.write_to(&mut buf, v);
            }
            if v > ProtocolVersion::V1_20 {
                write_varint(&mut buf, self.portal_cooldown);
            }
            buf
        } else if v >= ProtocolVersion::V1_17 {
            buf.put_u8(if self.is_hardcore { 1 } else { 0 });
            buf.put_u8(self.gamemode);
            buf.put_i8(self.previous_gamemode);
            write_varint(&mut buf, self.dimensions.len() as i32);
            for dim in &self.dimensions {
                write_string(&mut buf, dim);
            }
            self.registry_codec.serialize(&mut buf, false);
            self.legacy_dimension_nbt.serialize(&mut buf, false);
            write_string(&mut buf, &self.dimension_name);
            buf.put_u64(self.hashed_seed);
            write_varint(&mut buf, self.max_players);
            write_varint(&mut buf, self.view_distance);
            buf.put_u8(if self.reduced_debug_info { 1 } else { 0 });
            buf.put_u8(if self.enable_respawn_screen { 1 } else { 0 });
            buf.put_u8(if self.is_debug { 1 } else { 0 });
            buf.put_u8(if self.is_flat { 1 } else { 0 });
            buf
        } else if v >= ProtocolVersion::V1_16 {
            buf.put_u8(self.gamemode);
            buf.put_i8(self.previous_gamemode);
            write_varint(&mut buf, self.dimensions.len() as i32);
            for dim in &self.dimensions {
                write_string(&mut buf, dim);
            }
            self.registry_codec.serialize(&mut buf, false);
            write_string(&mut buf, &self.legacy_dimension_type);
            write_string(&mut buf, &self.dimension_name);
            buf.put_u64(self.hashed_seed);
            buf.put_u8(self.max_players as u8);
            write_varint(&mut buf, self.view_distance);
            buf.put_u8(if self.reduced_debug_info { 1 } else { 0 });
            buf.put_u8(if self.enable_respawn_screen { 1 } else { 0 });
            buf.put_u8(if self.is_debug { 1 } else { 0 });
            buf.put_u8(if self.is_flat { 1 } else { 0 });
            buf
        } else if v >= ProtocolVersion::V1_15 {
            buf.put_u8(self.gamemode);
            buf.put_i32(self.legacy_dimension);
            buf.put_u64(self.hashed_seed);
            buf.put_u8(self.max_players as u8);
            write_string(&mut buf, &self.legacy_level_type);
            write_varint(&mut buf, self.view_distance);
            buf.put_u8(if self.reduced_debug_info { 1 } else { 0 });
            buf.put_u8(if self.enable_respawn_screen { 1 } else { 0 });
            buf
        } else if v >= ProtocolVersion::V1_14 {
            buf.put_u8(self.gamemode);
            buf.put_i32(self.legacy_dimension);
            buf.put_u8(self.max_players as u8);
            write_string(&mut buf, &self.legacy_level_type);
            write_varint(&mut buf, self.view_distance);
            buf.put_u8(if self.reduced_debug_info { 1 } else { 0 });
            buf
        } else if v >= ProtocolVersion::V1_13_2 {
            buf.put_u8(self.gamemode);
            buf.put_i32(self.legacy_dimension);
            buf.put_u8(2);  // FIXME: we set the difficulty to normal no matter what
            buf.put_u8(self.max_players as u8);
            write_string(&mut buf, &self.legacy_level_type);
            buf.put_u8(if self.reduced_debug_info { 1 } else { 0 });
            buf
        } else {
            buf
        }
    }

    fn id(&self, v: ProtocolVersion) -> i32 {
        return if v >= ProtocolVersion::V1_20_6 {
            0x2B
        } else if v >= ProtocolVersion::V1_20_2 {
            0x29
        } else if v >= ProtocolVersion::V1_20 {
            0x28
        } else if v >= ProtocolVersion::V1_19_3 {
            0x24
        } else if v >= ProtocolVersion::V1_19 {
            0x23
        } else if v >= ProtocolVersion::V1_18_1 {
            0x26
        } else if v >= ProtocolVersion::V1_16 {
            0x25
        } else if v >= ProtocolVersion::V1_15_2 {
            0x26
        } else if v >= ProtocolVersion::V1_13_2 {
            0x25
        } else {
            0x25
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChunkDataS2C {
    pub(crate) x: i32,
    pub(crate) z: i32,
    pub(crate) heightmaps: Tag,
    pub(crate) chunk: Chunk,
}


impl PacketS2C for ChunkDataS2C {
    fn encode(&self, v: ProtocolVersion) -> BytesMut {
        // TODO: fix versioning
        let mut buf = BytesMut::new();
        buf.put_i32(self.x);
        buf.put_i32(self.z);
        self.heightmaps.serialize(&mut buf, v >= ProtocolVersion::V1_20_2);
        self.chunk.serialize_to_chunk_packet(&mut buf, v);
        buf
    }

    fn id(&self, v: ProtocolVersion) -> i32 {
        if v >= ProtocolVersion::V1_20_6 {
            0x27
        } else if v >= ProtocolVersion::V1_20_2 {
            0x25
        } else if v >= ProtocolVersion::V1_19_4 {
            0x24
        } else if v >= ProtocolVersion::V1_19_3 {
            0x20
        } else if v >= ProtocolVersion::V1_19_2 {
            0x21
        } else if v >= ProtocolVersion::V1_19 {
            0x1f
        } else if v >= ProtocolVersion::V1_17 {
            0x22
        } else if v >= ProtocolVersion::V1_16_2 {
            0x20
        } else if v >= ProtocolVersion::V1_16_1 {
            0x21
        } else if v >= ProtocolVersion::V1_15 {
            0x22
        } else if v >= ProtocolVersion::V1_14 {
            0x21
        } else {  // 1.13.2
            0x22
        }
    }
}


#[derive(Debug, Clone)]
pub struct SyncPlayerPositionS2C {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) z: f64,
    pub(crate) yaw: f32,
    pub(crate) pitch: f32,
    pub(crate) flags: u8,
    pub(crate) teleport_id: i32,
    pub(crate) dismount: bool,
}


impl PacketS2C for SyncPlayerPositionS2C {
    fn encode(&self, v: ProtocolVersion) -> BytesMut {
        let mut buf = BytesMut::new();
        buf.put_f64(self.x);
        buf.put_f64(self.y);
        buf.put_f64(self.z);
        buf.put_f32(self.yaw);
        buf.put_f32(self.pitch);
        buf.put_u8(self.flags);
        write_varint(&mut buf, self.teleport_id);
        if v >= ProtocolVersion::V1_17 && v < ProtocolVersion::V1_19_4 {
            buf.put_u8(if self.dismount { 1 } else { 0 });
        }
        buf
    }

    fn id(&self, v: ProtocolVersion) -> i32 {
        if v >= ProtocolVersion::V1_20_6 {
            0x40
        } else if v >= ProtocolVersion::V1_20_2 {
            0x3e
        } else if v >= ProtocolVersion::V1_19_4 {
            0x3c
        } else if v >= ProtocolVersion::V1_19_3 {
            0x38
        } else if v >= ProtocolVersion::V1_19_2 {
            0x39
        } else if v >= ProtocolVersion::V1_19 {
            0x36
        } else if v >= ProtocolVersion::V1_17 {
            0x38
        } else if v >= ProtocolVersion::V1_16_2 {
            0x34
        } else if v >= ProtocolVersion::V1_16_1 {
            0x35
        } else if v >= ProtocolVersion::V1_15 {
            0x36
        } else if v >= ProtocolVersion::V1_14 {
            0x35
        } else {  // 1.13.2
            0x32
        }
    }
}


#[derive(Debug, Clone)]
pub struct GameMessageS2C {
    pub(crate) text: TextComponent,
    pub(crate) overlay: bool,
}


impl PacketS2C for GameMessageS2C {
    fn encode(&self, v: ProtocolVersion) -> BytesMut {
        let mut buf = BytesMut::new();
        self.text.to_nbt().serialize(&mut buf, v >= ProtocolVersion::V1_20_2);
        buf.put_u8(if self.overlay { 1 } else { 0 });
        buf
    }

    fn id(&self, _: ProtocolVersion) -> i32 {
        0x6c
    }
}
