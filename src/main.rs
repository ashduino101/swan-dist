mod nbt;
mod region;
mod chunk;
mod block;
mod level;
mod filters;
mod handlers;
mod models;
mod config;
mod server;
mod claims;

use std::collections::HashMap;
use std::{env, fs};
use std::fs::File;
use std::io::prelude::*;
use std::sync::Arc;
use std::time::Instant;
use async_trait::async_trait;
use bytes::{Bytes, BytesMut, Buf, BufMut};
use clap::{Subcommand, Args};
use clap::Parser as _;
use clap_derive::Parser;
use crc32fast::Hasher;
use flate2::read::ZlibDecoder;
use log::info;
use tokio::sync::mpsc::{channel, UnboundedSender, Sender};
use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;
use zip::ZipArchive;
use warp::Filter;
use crate::models::{AuthManager, SharedAuthManager};
use crate::nbt::Tag;
use crate::region::Region;
use crate::server::base::Server;
use crate::server::common::Profile;
use crate::server::handler::{PacketHandler, SendError};
use crate::server::packets::c2s::handshake::HandshakeC2S;
use crate::server::packets::c2s::play::ChatC2S;
use crate::server::packets::c2s::status::{PingRequestC2S, StatusRequestC2S};
use crate::server::packets::packet::PacketS2C;
use crate::server::packets::stage::Stage;
use crate::server::text::{ChatColor, TextComponent};

#[derive(Parser)]
#[command(about="SwanCraft Map Distribution Server")]
pub struct Cli {
    #[clap(long,short)]
    pub path: String,
}

pub struct AuthPacketHandler {
    pub stage: Stage,
    pub channel: UnboundedSender<Box<dyn PacketS2C + Send>>,
    pub profile: Profile,
    pub manager: SharedAuthManager,
    pub stream: Sender<Option<Profile>>
}

impl AuthPacketHandler {
    fn new(manager: SharedAuthManager) -> AuthPacketHandler {
        AuthPacketHandler {
            stage: Stage::Handshake,
            channel: mpsc::unbounded_channel().0,  // to be set later
            profile: Profile {
                id: Uuid::from_u128(0u128),
                name: "Unknown".to_string(),
                properties: vec![]
            },
            manager,
            stream: channel(4).0  // placeholder
        }
    }
}

#[async_trait]
impl PacketHandler for AuthPacketHandler {
    fn set_channel(&mut self, sender: UnboundedSender<Box<dyn PacketS2C + Send>>) {
        self.channel = sender;
    }

    fn set_stage(&mut self, new_stage: Stage) {
        self.stage = new_stage;
    }

    fn get_stage(&self) -> &Stage {
        &self.stage
    }

    fn send_packet(&mut self, packet: Box<dyn PacketS2C + Send>) -> anyhow::Result<bool> {
        match self.channel.send(packet) {
            Ok(_) => Ok(true),
            Err(_) => Err(SendError::new().into())
        }
    }

    async fn on_handshake(&mut self, packet: HandshakeC2S) -> anyhow::Result<bool> {
        Ok(true)
    }

    async fn on_status_request(&mut self, packet: StatusRequestC2S) -> anyhow::Result<bool> {
        Ok(true)
    }

    async fn on_ping_request(&mut self, packet: PingRequestC2S) -> anyhow::Result<bool> {
        Ok(true)
    }

    async fn on_chat(&mut self, packet: ChatC2S) -> anyhow::Result<bool> {
        let mut manager_arc = self.manager.clone();
        let mut manager = manager_arc.lock().await;
        if !manager.has_code(&packet.message) {
            let mut msg1 = TextComponent::plain("This code does not exist! ");
            msg1.set_bold(true);
            msg1.set_color(ChatColor::Red);
            let mut msg2 = TextComponent::plain("Did you enter it correctly?");
            msg2.set_bold(false);
            msg2.set_color(ChatColor::DarkRed);
            msg1.add_component(msg2);
            self.send_game_message(msg1, false).unwrap();
        } else if manager.is_code_used(&packet.message) {
            let mut msg1 = TextComponent::plain("This code has already been used! ");
            msg1.set_bold(true);
            msg1.set_color(ChatColor::Red);
            let mut msg2 = TextComponent::plain("Please generate a new code and try again.");
            msg2.set_bold(false);
            msg2.set_color(ChatColor::DarkRed);
            msg1.add_component(msg2);
            self.send_game_message(msg1, false).unwrap();
        } else {
            manager.use_code(&packet.message);

            let profile = self.get_profile().await;
            info!("User {} ({}) authorized with code {}", profile.name, profile.id, packet.message);

            manager.get_sender(&packet.message).unwrap().send(Some(profile.clone())).await.unwrap();

            let mut msg1 = TextComponent::plain("Authorization successful! ");
            msg1.set_bold(true);
            msg1.set_color(ChatColor::Green);
            let mut msg2 = TextComponent::plain("You may return to the webmap.");
            msg2.set_bold(false);
            msg2.set_color(ChatColor::Gray);
            msg1.add_component(msg2);
            self.kick(msg1).unwrap();
        }
        Ok(true)
    }

    async fn set_profile(&mut self, profile: Profile) {
        self.profile = profile;
    }

    async fn get_profile(&mut self) -> &Profile {
        &self.profile
    }
}

#[tokio::main]
async fn main() {
    Cli::parse();

    if env::var_os("RUST_LOG").is_none() {
        env::set_var("RUST_LOG", "swandist=info");
    }
    pretty_env_logger::init();

    let mut manager = Arc::new(Mutex::new(AuthManager::new()));

    let api = filters::routes(manager.clone());

    let routes = api.with(warp::log("swandist"));

    tokio::spawn(async move {
        let mut server = Server::new();
        server.set_handler_factory(move || Box::new(AuthPacketHandler::new(manager.clone())));
        server.start("127.0.0.1:25565").await.expect("failed to start server");
    });

    warp::serve(routes).run(([127, 0, 0, 1], 7650)).await;
}
