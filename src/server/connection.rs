use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use bytes::{Bytes, BytesMut, BufMut};
use core::time::Duration;
use image::ImageReader;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, Mutex};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use log::{info, trace, warn};
use aes::cipher::{BlockEncryptMut, BlockDecryptMut, BlockSizeUser, KeyIvInit, generic_array::GenericArray, AsyncStreamCipher};
use crypto::blockmodes::{PaddingProcessor, PkcsPadding};
use num_bigint::BigInt;
use rand::RngCore;
use reqwest::StatusCode;
use rsa::traits::PublicKeyParts;
use serde_derive::Deserialize;
use sha1::{Sha1, Digest};
use tokio::{task, time};
use uuid::Uuid;
use crate::{Region, Server, Tag};
use crate::chunk::Chunk;
use crate::server::common::{ClientInfo, Profile};
use crate::server::handler::PacketHandler;
use crate::server::packets::c2s::config::{ClientInfoC2S, CustomPayloadC2S, KeepAliveC2S, PongC2S, ReadyC2S, ResourcePackStatus, ResourcePackStatusC2S, SelectKnownPacksC2S, CookieResponseC2S as ConfigCookieResponseC2S, VersionedIdentifier};
use crate::server::packets::c2s::handshake::HandshakeC2S;
use crate::server::packets::c2s::login::{CookieResponseC2S, EnterConfigurationC2S, LoginHelloC2S, LoginKeyC2S, LoginQueryResponseC2S};
use crate::server::packets::c2s::play::ChatC2S;
use crate::server::packets::c2s::status::{PingRequestC2S, StatusRequestC2S};
use crate::server::packets::packet::{PacketS2C, PacketC2S};
use crate::server::packets::s2c::config::{CustomPayloadS2C, DynamicRegistriesS2C, FeaturesS2C, Link, LinkLabel, LinksS2C, ReadyS2C, RegistryEntry, SelectKnownPacksS2C};
use crate::server::packets::s2c::login::{LoginDisconnectS2C, LoginHelloS2C, LoginSuccessS2C};
use crate::server::packets::s2c::play::{ChunkDataS2C, EventType, GameEventS2C, GameMessageS2C, JoinGameS2C, KeepAliveS2C, SyncPlayerPositionS2C};
use crate::server::packets::s2c::status::{PingResponseS2C, StatusResponseS2C};
use crate::server::packets::stage::Stage;
use crate::server::status::StatusBuilder;
use crate::server::text::{ChatColor, HoverEvent, TextComponent};
use crate::server::utils::{read_varint, write_string, write_varint};
use crate::server::version::ProtocolVersion;

type EncCipher = cfb8::Encryptor<aes::Aes128>;
type DecCipher = cfb8::Decryptor<aes::Aes128>;

static REGISTRY_121: &[u8] = include_bytes!("registry_1.21.nbt");
static REGISTRY_1206: &[u8] = include_bytes!("registry_1.20.6.nbt");
static REGISTRY_1194: &[u8] = include_bytes!("registry_1.19.4.nbt");
static REGISTRY_DEFAULT: &[u8] = include_bytes!("registry.nbt");

macro_rules! tri_handle {
    ($($t:tt)+) => {
        match $($t)+ {
            Ok(_) => {},
            Err(e) => {
                warn!("error in packet handler: {e}");
            }
        }
    }
}

macro_rules! packet_case {
    ($($typ:ident = $cls:ident @ $v:ident => {$($t:tt)+}),*,?? => {$($other:tt)*}) => {
        if false {
            unreachable!();
        }
        $(
            else if $typ == $cls::id($v) {
                $($t)+;
            }
        )*
        else {
            $($other)*;
        }
    };
}

pub fn sha_digest(sha: Sha1) -> String {
    let mut sha_bytes = sha.finalize();
    BigInt::from_signed_bytes_be(&sha_bytes).to_str_radix(16)
}

pub struct ClientConnection {
    handler: Arc<Mutex<Box<dyn PacketHandler + Send>>>,
    version: Mutex<ProtocolVersion>,
    auth_nonce: Mutex<Option<Bytes>>,
    secret: Option<Vec<u8>>,
    username: Mutex<String>,
    enc_cipher: Option<EncCipher>,
    dec_cipher: Option<DecCipher>,
    client_info: ClientInfo,
    parent: Arc<Mutex<Server>>  // shared globally
}

unsafe impl Send for ClientConnection {

}

impl ClientConnection {
    pub fn new(handler: Box<dyn PacketHandler + Send>, parent: Arc<Mutex<Server>>) -> ClientConnection {
        ClientConnection {
            handler: Arc::new(Mutex::new(handler)),
            version: Mutex::new(ProtocolVersion::Unknown),
            auth_nonce: Mutex::new(None),
            secret: None,
            username: Mutex::new("Offline".to_owned()),
            enc_cipher: None,
            dec_cipher: None,
            client_info: Default::default(),
            parent
        }

    }

    fn maybe_decrypt(&mut self, block: &mut [u8]) {
        if self.dec_cipher.is_some() {
            for chunk in block.chunks_mut(DecCipher::block_size()) {
                let gen_arr = GenericArray::from_mut_slice(chunk);
                self.dec_cipher.as_mut().unwrap().decrypt_block_mut(gen_arr);
            }
        }
    }

    async fn send_game_join(&self) {
        let mut handler = self.handler.lock().await;
        handler.send_packet(Box::new(JoinGameS2C {
            entity_id: 123,
            is_hardcore: false,
            gamemode: 3,
            previous_gamemode: -1,
            dimensions: vec!["minecraft:overworld".to_owned()],
            registry_codec: Tag::parse(&mut Bytes::from(REGISTRY_DEFAULT)),
            legacy_dimension_nbt: Tag::Compound(HashMap::new()),
            max_players: 1,
            view_distance: 0,
            simulation_distance: 1,
            reduced_debug_info: false,
            enable_respawn_screen: false,
            do_limited_crafting: false,
            legacy_dimension_type: "".to_string(),
            legacy_dimension: 0,
            legacy_level_type: "".to_string(),
            dimension_type: 0,
            dimension_name: "minecraft:overworld".to_string(),
            hashed_seed: 0,
            is_debug: false,
            is_flat: false,
            death_dimension: None,
            death_location: None,
            portal_cooldown: 20,
            enforces_secure_chat: false
        })).unwrap();
        // println!("sent join");
    }

    pub async fn handle(&mut self, mut socket: TcpStream) {
        let (tx, mut rx): (UnboundedSender<Box<dyn PacketS2C + Send>>, UnboundedReceiver<Box<dyn PacketS2C + Send>>) = mpsc::unbounded_channel();
        {
            self.handler.lock().await.set_channel(tx);
        }

        let key = {
            self.parent.lock().await.key.clone()
        };

        let handler_arc = self.handler.clone();

        let (mut read_half, mut write_half) = socket.split();

        loop {
            let mut first_byte = vec![0u8; 1];
            tokio::select! {
                Some(m) = rx.recv() => {
                    let v = {
                        self.version.lock().await.clone()
                    };
                    let body = m.encode(v);
                    let mut temp_writer = BytesMut::new();
                    write_varint(&mut temp_writer, m.id(v));
                    temp_writer.put(body);
                    let mut packet_writer = BytesMut::new();
                    write_varint(&mut packet_writer, temp_writer.len() as i32);
                    packet_writer.put(temp_writer);

                    if self.enc_cipher.is_some() {
                        for chunk in packet_writer.chunks_mut(EncCipher::block_size()) {
                            let gen_arr = GenericArray::from_mut_slice(chunk);
                            self.enc_cipher.as_mut().unwrap().encrypt_block_mut(gen_arr);
                        }
                    };

                    match write_half
                        .write_all(&packet_writer[..])
                        .await {
                        Ok(_) => {},
                        Err(_) => break
                    }
                }
                Ok(first_byte_size) = read_half.read(&mut first_byte) => {
                    let stage = {
                        handler_arc.lock().await.get_stage().clone()
                    };

                    if first_byte_size != 1 {  // should be 1 unless the connection closed
                        break;
                    }

                    // modified reader to read from the socket
                    // FIXME: this is kind of a mess, but it works fine
                    self.maybe_decrypt(&mut first_byte[..]);
                    let mut num = first_byte[0] as i32;

                    if stage == Stage::Handshake && num == 0xFE {
                        // Legacy ping, close the connection since we aren't a legacy server
                        break;
                    }

                    if num & 0b10000000 != 0 {  // is the packet larger than 127 bytes?
                        num &= 0b01111111;
                        let mut i = 1;
                        loop {
                            if let Ok(n) = read_half.read(&mut first_byte).await {
                                if n != 1 {
                                    warn!("partial decode!");
                                    continue;
                                }
                                self.maybe_decrypt(&mut first_byte[..]);
                                num |= (i32::from(first_byte[0]) & 0b01111111) << (i * 7);
                                if first_byte[0] & 0b10000000 == 0 {
                                    break;
                                }
                                i += 1;
                            }
                        }
                    }

                    let num = num as usize;

                    if num == 0 {
                        break;
                    }

                    let mut buf = vec![0u8; num];
                    if let Ok(num_read) = read_half.read(&mut buf[..]).await {
                        if num_read != num {
                            warn!("buffer size mismatch! expected {}, but got {} -- this may cause issues!", num, num_read);
                        }
                    } else {
                        warn!("invalid read!");
                        continue;
                    };

                    let v = {
                        self.version.lock().await.clone()
                    };

                    self.maybe_decrypt(&mut buf);

                    let mut reader = Bytes::from(buf);

                    let packet_type = read_varint(&mut reader);
                    // println!("got packet {} of size {} during stage {:?}", packet_type, num, stage);


                    match stage {
                        // HANDSHAKE ------------------------------------------------------
                        Stage::Handshake => {
                            // Handled internally by default
                            packet_case!(
                                packet_type = HandshakeC2S @ v => {
                                    let packet = HandshakeC2S::decode(&mut reader, v);
                                    *self.version.lock().await = packet.version;

                                    let mut h = handler_arc.lock().await;
                                    h.set_stage(packet.next_stage);
                                    tri_handle!(h.on_handshake(packet).await);
                                },
                                ?? => {

                                }
                            );
                        },
                        // STATUS ---------------------------------------------------------
                        Stage::Status => {
                            packet_case!(
                                packet_type = StatusRequestC2S @ v => {
                                    let packet = StatusRequestC2S::decode(&mut reader, v);
                                    let mut description = TextComponent::new();
                                    description.set_text("SwanCraft World Download");
                                    description.set_gradient(&[ChatColor::Aqua, ChatColor::LightPurple]);
                                    tri_handle!(handler_arc.lock().await.send_packet(Box::new(StatusResponseS2C::new(
                                        StatusBuilder::new(v)
                                        .with_description(description)
                                        .with_favicon(ImageReader::open("favicon.png").unwrap().decode().unwrap())
                                        .finish()
                                    ))));
                                    tri_handle!(handler_arc.lock().await.on_status_request(packet).await);
                                },
                                packet_type = PingRequestC2S @ v => {
                                    let packet = PingRequestC2S::decode(&mut reader, v);
                                    tri_handle!(handler_arc.lock().await.send_packet(Box::new(PingResponseS2C::new(packet.payload))));
                                    tri_handle!(handler_arc.lock().await.on_ping_request(packet).await);
                                },
                                ?? => {

                                }
                            );
                        },
                        // LOGIN ----------------------------------------------------------
                        Stage::Login => {
                            packet_case!(
                                packet_type = LoginHelloC2S @ v => {
                                    if v != ProtocolVersion::V1_21 {
                                        self.handler.lock().await.kick(TextComponent::plain(format!("Outdated client! Please use {}", ProtocolVersion::V1_21.get_name()).as_str())).unwrap();
                                        continue;
                                    }
                                    let packet = LoginHelloC2S::decode(&mut reader, v);
                                    // println!("{:?}", packet);
                                    {
                                        *self.username.lock().await = packet.name;
                                    }
                                    // Send an encryption response
                                    let key_bytes = {
                                        rsa_der::public_key_to_der(&key.n().to_bytes_be(), &key.e().to_bytes_be())
                                    };
                                    let mut verify_token = &mut [0u8; 4];
                                    rand::thread_rng().fill_bytes(&mut verify_token[..]);
                                    {
                                        *self.auth_nonce.lock().await = Some(Bytes::copy_from_slice(verify_token));
                                    }
                                    let packet = LoginHelloS2C {
                                        server_id: "".to_owned(),
                                        public_key: key_bytes.into(),
                                        nonce: Bytes::copy_from_slice(verify_token),
                                        needs_authentication: true
                                    };
                                    self.handler.lock().await.send_packet(Box::new(packet)).unwrap();
                                },
                                packet_type = LoginKeyC2S @ v => {
                                    let packet = LoginKeyC2S::decode(&mut reader, v);
                                    // Set up encryption
                                    let (secret, sha) = {
                                        let secret = key.decrypt(rsa::Pkcs1v15Encrypt, &packet.shared_secret).unwrap();
                                        if let Some(encrypted_nonce) = &packet.nonce {
                                            let nonce = key.decrypt(rsa::Pkcs1v15Encrypt, &encrypted_nonce[..]).unwrap();
                                            {
                                                let our_nonce = self.auth_nonce.lock().await.clone();
                                                if let Some(check) = our_nonce {
                                                    if nonce != check {
                                                        warn!("failed to verify nonce");
                                                        self.handler.lock().await.send_packet(Box::new(LoginDisconnectS2C {
                                                            reason: TextComponent::plain("Failed to verify token")
                                                        })).unwrap();
                                                        continue;  // FIXME: disconnect
                                                    }
                                                }
                                            }
                                        }

                                        let mut sha = Sha1::new();
                                        sha.update(secret.clone());
                                        sha.update(rsa_der::public_key_to_der(&key.n().to_bytes_be(), &key.e().to_bytes_be()));
                                        (secret, sha_digest(sha))
                                    };
                                    // Retrieve Mojang profile
                                    let resp = {
                                        let username = {
                                            self.username.lock().await.clone()
                                        };
                                        let username_enc = urlencoding::encode(&username);
                                        reqwest::get(
                                            format!(
                                                "https://sessionserver.mojang.com/session/minecraft/hasJoined?username={}&serverId={}",
                                                username_enc,
                                                sha
                                            )
                                        ).await.unwrap()
                                    };

                                    if resp.status() != StatusCode::OK {
                                        {
                                            warn!("profile retrieval failed with code {}", resp.status());
                                            self.handler.lock().await.kick(TextComponent::plain("Failed to retrieve Mojang profile")).unwrap();
                                        }
                                    }

                                    let profile: Profile = resp.json().await.unwrap();

                                    // update our username if necessary
                                    {
                                        *self.username.lock().await = profile.name.clone();
                                    }

                                    // update our profile on the handler
                                    {
                                        self.handler.lock().await.set_profile(profile.clone()).await;
                                    }

                                    // enable encryption
                                    self.secret = Some(secret.clone());

                                    self.enc_cipher = Some(EncCipher::new_from_slices(&secret[..], &secret[..]).unwrap());
                                    self.dec_cipher = Some(DecCipher::new_from_slices(&secret[..], &secret[..]).unwrap());
                                    {
                                        let mut handler = self.handler.lock().await;
                                        handler.send_packet(Box::new(LoginSuccessS2C {
                                            profile,
                                            strict_error_handling: false
                                        })).unwrap();

                                        // Before 1.20.2, this switches the stage to Play
                                        if v < ProtocolVersion::V1_20_2 {
                                            handler.set_stage(Stage::Play);
                                        }
                                    }
                                    if v < ProtocolVersion::V1_20_2 {
                                        self.send_game_join().await;
                                    }
                                },
                                packet_type = LoginQueryResponseC2S @ v => {
                                    let packet = LoginQueryResponseC2S::decode(&mut reader, v);
                                    // println!("{:?}", packet);
                                },
                                packet_type = EnterConfigurationC2S @ v => {
                                    let packet = EnterConfigurationC2S::decode(&mut reader, v);
                                    // println!("entering configuration stage");
                                    {
                                        let mut h = handler_arc.lock().await;
                                        h.set_stage(Stage::Config);
                                    }
                                },
                                packet_type = CookieResponseC2S @ v => {
                                    let packet = CookieResponseC2S::decode(&mut reader, v);
                                    // println!("{:?}", packet);
                                },
                                ?? => {

                                }
                            );
                        },
                        // CONFIG ---------------------------------------------------------
                        Stage::Config => {
                            packet_case!(
                                packet_type = ClientInfoC2S @ v => {
                                    let packet = ClientInfoC2S::decode(&mut reader, v);
                                    // println!("{:?}", packet);
                                    self.client_info = packet.client_info;

                                    let mut handler = self.handler.lock().await;
                                    // Send our client brand
                                    let mut brand_buf = BytesMut::new();
                                    write_string(&mut brand_buf, "WorldFreezer");
                                    handler.send_packet(Box::new(CustomPayloadS2C {
                                        key: "minecraft:brand".to_owned(),
                                        payload: brand_buf.into()
                                    })).unwrap();

                                    // Tell them our features
                                    handler.send_packet(Box::new(FeaturesS2C {
                                        features: vec!["minecraft:vanilla".to_owned()]
                                    })).unwrap();

                                    if v >= ProtocolVersion::V1_21 {
                                        // Send our default known packs
                                        handler.send_packet(Box::new(SelectKnownPacksS2C {
                                            known_packs: vec![VersionedIdentifier {
                                                namespace: "minecraft".to_owned(),
                                                id: "core".to_owned(),
                                                version: "1.21".to_owned(),
                                            }]
                                        })).unwrap();
                                    } else if v >= ProtocolVersion::V1_20_5 {
                                        handler.send_packet(Box::new(SelectKnownPacksS2C {
                                            known_packs: vec![]  // 1.20.5 doesn't need any
                                        })).unwrap();
                                    }
                                },
                                packet_type = ConfigCookieResponseC2S @ v => {
                                    let packet = ConfigCookieResponseC2S::decode(&mut reader, v);
                                    // println!("{:?}", packet);
                                },
                                packet_type = CustomPayloadC2S @ v => {
                                    let packet = CustomPayloadC2S::decode(&mut reader, v);
                                    // println!("{:?}", packet);
                                },
                                packet_type = ReadyC2S @ v => {
                                    let packet = ReadyC2S::decode(&mut reader, v);
                                    // println!("{:?}", packet);
                                    // Our client is ready now also, let's enter the play stage
                                    {
                                        let mut handler = self.handler.lock().await;
                                        handler.set_stage(Stage::Play);
                                    }

                                    self.send_game_join().await;

                                    // Tell them the initial chunks are coming
                                    if v >= ProtocolVersion::V1_20_4 {
                                        let mut handler = self.handler.lock().await;
                                        handler.send_packet(Box::new(GameEventS2C {
                                            event: EventType::InitialChunksComing,
                                            value: 0.0
                                        })).unwrap();
                                    }

                                    // Teleport to initial pos
                                    {
                                        let mut handler = self.handler.lock().await;
                                        handler.send_packet(Box::new(SyncPlayerPositionS2C {
                                            x: 0.0,
                                            y: 128.0,
                                            z: 0.0,
                                            yaw: 0.0,
                                            pitch: 0.0,
                                            flags: 0,
                                            teleport_id: 0,
                                            dismount: true
                                        })).unwrap();
                                    }

                                    // Start a keepalive loop to prevent the connection from closing
                                    let mut keepalive_handler = self.handler.clone();
                                    task::spawn(async move {
                                        let mut interval = time::interval(Duration::from_secs(1));

                                        loop {
                                            interval.tick().await;
                                            {
                                                let mut handler = keepalive_handler.lock().await;
                                                match handler.send_packet(Box::new(KeepAliveS2C {
                                                    payload: rand::thread_rng().next_u64()
                                                })) {
                                                    Ok(_) => {},
                                                    Err(_) => break
                                                };
                                            }
                                        }
                                    });

                                    {
                                        // Send message
                                        let mut decor = TextComponent::new();
                                        decor.set_text("៚ ");

                                        let mut title = TextComponent::plain("Welcome!");
                                        title.set_bold(true);
                                        title.set_gradient(&[ChatColor::DarkCyan, ChatColor::Aqua]);
                                        title.prepend_component(decor.clone());
                                        let mut handler = self.handler.lock().await;
                                        handler.send_packet(Box::new(GameMessageS2C {
                                            text: title,
                                            overlay: false
                                        })).unwrap();

                                        let mut instructions = TextComponent::plain("To verify your account, please send your one-time code in the game chat. It will not be shared with others.");
                                        instructions.set_color(ChatColor::Gold);
                                        handler.send_packet(Box::new(GameMessageS2C {
                                            text: instructions,
                                            overlay: false
                                        })).unwrap();
                                    }

                                    // Start sending chunks
                                    let mut chunk_handler = self.handler.clone();
                                    task::spawn(async move {
                                        // let mut data = include_bytes!("../../server/world/region/r.0.0.mca");
                                        // let mut region = Region::load(Cursor::new(&mut data));

                                        let diam = 3i32;

                                        let mut x = 0;
                                        let mut z = 0;
                                        let mut dx = 0;
                                        let mut dz = -1;
                                        for i in 0..diam.pow(2) {
                                            if ((-diam / 2) < x && x <= (diam / 2)) && ((-diam / 2) < z && z <= (diam / 2)) {
                                                // match region.get_chunk(x, z) {
                                                //     Some(chunk) => ,
                                                //     None => {}
                                                // };
                                                {
                                                    let mut heightmaps = HashMap::new();
                                                    heightmaps.insert("MOTION_BLOCKING".to_owned(), Tag::LongArray(vec![0i64; 37]));
                                                    heightmaps.insert("WORLD_SURFACE".to_owned(), Tag::LongArray(vec![0i64; 37]));

                                                    let mut heightmaps = Tag::Compound(heightmaps);
                                                    let mut handler = chunk_handler.lock().await;
                                                    handler.send_packet(Box::new(ChunkDataS2C {
                                                        x: x,
                                                        z: z,
                                                        heightmaps,
                                                        chunk: Chunk::empty()
                                                    })).unwrap();
                                                }
                                            }
                                            if x == z || (x < 0 && x == -z) || (x > 0 && x == 1 - z) {
                                                (dx, dz) = (-dz, dx);
                                            }
                                            (x, z) = (x + dx, z + dz);
                                        }
                                        // }
                                    });
                                },
                                packet_type = KeepAliveC2S @ v => {
                                    let packet = KeepAliveC2S::decode(&mut reader, v);
                                    // println!("{:?}", packet);
                                },
                                packet_type = PongC2S @ v => {
                                    let packet = PongC2S::decode(&mut reader, v);
                                    // println!("{:?}", packet);
                                },
                                packet_type = ResourcePackStatusC2S @ v => {
                                    let packet = ResourcePackStatusC2S::decode(&mut reader, v);
                                    // println!("{:?}", packet);
                                },
                                packet_type = SelectKnownPacksC2S @ v => {
                                    let packet = SelectKnownPacksC2S::decode(&mut reader, v);
                                    // println!("{:?}", packet);
                                    // Now that we've received this, let's send the registries and finish configuration

                                    // println!("sending reg for {:?}", v);
                                    let registries = Tag::parse(&mut Bytes::from(match v {
                                        ProtocolVersion::V1_21 => REGISTRY_121,
                                        ProtocolVersion::V1_20_4 | ProtocolVersion::V1_20_5 => REGISTRY_1206,
                                        ProtocolVersion::V1_19_4 => REGISTRY_1194,
                                        _ => REGISTRY_DEFAULT,
                                    }));
                                    let registries = registries.as_compound().unwrap();

                                    let mut handler = self.handler.lock().await;

                                    for (k, v) in registries {
                                        let mut entries = Vec::new();

                                        let mut value = v.get("value").unwrap().as_list().unwrap();
                                        for entry in value {
                                            entries.push(RegistryEntry {
                                                id: entry.get("name").unwrap().as_string().unwrap().clone(),
                                                data: match entry.get("element") {
                                                    Ok(d) => Some(d.clone()),
                                                    Err(_) => None
                                                }
                                            });
                                        }

                                        handler.send_packet(Box::new(DynamicRegistriesS2C {
                                            registry_id: k.clone(),
                                            entries
                                        })).unwrap();
                                    }

                                    if v >= ProtocolVersion::V1_21 {
                                        let mut left = TextComponent::new();
                                        let mut right = TextComponent::new();
                                        left.set_text("៚ ");
                                        right.set_text("");

                                        let mut website = TextComponent::new();
                                        website.set_text("Website");
                                        website.set_gradient(&[ChatColor::Aqua, ChatColor::White]);
                                        website.prepend_component(left.clone());
                                        website.add_component(right.clone());
                                        let mut store = TextComponent::new();
                                        store.set_text("Store");
                                        store.set_gradient(&[ChatColor::DarkGreen, ChatColor::Green]);
                                        store.prepend_component(left.clone());
                                        store.add_component(right.clone());
                                        let mut vote = TextComponent::new();
                                        vote.set_text("Vote!");
                                        vote.set_gradient(&[ChatColor::DarkCyan, ChatColor::Aqua]);
                                        vote.prepend_component(left.clone());
                                        vote.add_component(right.clone());

                                        // let mut topmc = TextComponent::new();
                                        // topmc.set_text("Top MC Servers");
                                        // topmc.set_gradient(&[ChatColor::Blue, ChatColor::Gray]);
                                        // let mut mcsl = TextComponent::new();
                                        // mcsl.set_text("Minecraft SL");
                                        // mcsl.set_gradient(&[ChatColor::White, ChatColor::Gray]);
                                        // let mut mcs = TextComponent::new();
                                        // mcs.set_text("Minecraft Servers");
                                        // mcs.set_gradient(&[ChatColor::LightPurple, ChatColor::Purple]);
                                        // let mut pmc = TextComponent::new();
                                        // pmc.set_text("Planet Minecraft");
                                        // pmc.set_gradient(&[ChatColor::DarkGreen, ChatColor::DarkCyan]);
                                        // let mut mmp = TextComponent::new();
                                        // mmp.set_text("Minecraft MP");
                                        // mmp.set_color(ChatColor::DarkGreen);
                                        // let mut topg = TextComponent::new();
                                        // topg.set_text("TopG");
                                        // topg.set_gradient(&[ChatColor::Gold, ChatColor::Gray]);
                                        // let mut buzz = TextComponent::new();
                                        // buzz.set_text("Buzz");
                                        // buzz.set_gradient(&[ChatColor::Yellow, ChatColor::Gold]);

                                        handler.send_packet(Box::new(LinksS2C {
                                            links: vec![Link {
                                                label: LinkLabel::Custom(website),
                                                url: "https://swancraft.guildtag.com/".to_owned()
                                            }, Link {
                                                label: LinkLabel::Custom(store),
                                                url: "https://swancraft.buycraft.net/".to_owned()
                                            }, Link {
                                                label: LinkLabel::Custom(vote),
                                                url: "https://swancraft.guildtag.com/vote".to_owned()
                                            }], // Link {
                                            //     label: LinkLabel::Custom(topmc),
                                            //     url: "https://topminecraftservers.org/vote/4455".to_owned()
                                            // }, Link {
                                            //     label: LinkLabel::Custom(mcsl),
                                            //     url: "https://minecraft-server-list.com/server/389267/vote/".to_owned()
                                            // }, Link {
                                            //     label: LinkLabel::Custom(mcs),
                                            //     url: "https://minecraftservers.org/vote/410424".to_owned()
                                            // }, Link {
                                            //     label: LinkLabel::Custom(pmc),
                                            //     url: "https://www.planetminecraft.com/server/swancraft-3882768/vote/".to_owned()
                                            // }, Link {
                                            //     label: LinkLabel::Custom(mmp),
                                            //     url: "https://minecraft-mp.com/server/145239/vote/".to_owned()
                                            // }, Link {
                                            //     label: LinkLabel::Custom(topg),
                                            //     url: "https://topg.org/minecraft-servers/server-449700".to_owned()
                                            // }, Link {
                                            //     label: LinkLabel::Custom(buzz),
                                            //     url: "https://minecraft.buzz/vote/5680".to_owned()
                                            // }]
                                        })).unwrap();
                                    }

                                    handler.send_packet(Box::new(ReadyS2C {})).unwrap();
                                },
                                ?? => {

                                }
                            );
                        },
                        Stage::Play => {
                            packet_case!(
                                packet_type = ChatC2S @ v => {
                                    let packet = ChatC2S::decode(&mut reader, v);
                                    // println!("{:?}", packet);

                                    // let mut resp = TextComponent::plain(&packet.message);
                                    // resp.set_color(ChatColor::Red);
                                    // {
                                    //     let mut handler = self.handler.lock().await;
                                    //     handler.send_packet(Box::new(GameMessageS2C {
                                    //         text: resp,
                                    //         overlay: false
                                    //     })).unwrap();
                                    // }

                                    {
                                        let mut handler = self.handler.lock().await;
                                        tri_handle!(handler.on_chat(packet).await);
                                    }
                                },
                                ?? => {

                                }
                            )
                        },
                        _ => println!("unsupported stage {:?}", stage)
                    }
                }
            }
        }
        info!("Channel closed");
    }
}
