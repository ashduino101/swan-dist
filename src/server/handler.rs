use std::error::Error;
use std::fmt::{Display, Formatter, Write, Debug};
use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;
use crate::server::common::Profile;
use crate::server::packets::c2s::handshake::HandshakeC2S;
use crate::server::packets::c2s::play::ChatC2S;
use crate::server::packets::packet::PacketS2C;
use crate::server::packets::c2s::status::{PingRequestC2S, StatusRequestC2S};
use crate::server::packets::s2c::config::ConfigDisconnectS2C;
use crate::server::packets::s2c::login::LoginDisconnectS2C;
use crate::server::packets::s2c::play::{GameMessageS2C, PlayDisconnectS2C};
use crate::server::packets::s2c::status::{PingResponseS2C, StatusResponseS2C};
use crate::server::packets::stage::Stage;
use crate::server::status::StatusBuilder;
use crate::server::text::TextComponent;

#[async_trait]
pub trait PacketHandler {
    /// Called by the server when it sets the channel to send packets to
    fn set_channel(&mut self, sender: UnboundedSender<Box<dyn PacketS2C + Send>>);
    /// Transition to a different stage
    fn set_stage(&mut self, new_stage: Stage);
    /// Get the current stage
    fn get_stage(&self) -> &Stage;
    fn send_packet(&mut self, packet: Box<dyn PacketS2C + Send>) -> anyhow::Result<bool>;

    // Handshake
    /// Called whenever a user attempts to handshake
    async fn on_handshake(&mut self, packet: HandshakeC2S) -> anyhow::Result<bool> { Ok(true) }
    // Status
    /// Called whenever a status request is sent
    async fn on_status_request(&mut self, packet: StatusRequestC2S) -> anyhow::Result<bool> { Ok(true) }
    /// Called whenever a ping request is sent
    async fn on_ping_request(&mut self, packet: PingRequestC2S) -> anyhow::Result<bool> { Ok(true) }
    // Play
    /// Called whenever a chat message is sent by the user
    async fn on_chat(&mut self, packet: ChatC2S) -> anyhow::Result<bool> { Ok(true) }

    /// Set the user's profile
    async fn set_profile(&mut self, profile: Profile);
    /// Get the user's profile
    async fn get_profile(&mut self) -> &Profile;

    // Implemented by default
    fn kick(&mut self, reason: TextComponent) -> anyhow::Result<bool> {
        match self.get_stage() {
            Stage::Login => self.send_packet(Box::new(LoginDisconnectS2C { reason })),
            Stage::Config => self.send_packet(Box::new(ConfigDisconnectS2C { reason })),
            Stage::Play => self.send_packet(Box::new(PlayDisconnectS2C { reason })),
            _ => Ok(false)
        }
    }

    fn send_game_message(&mut self, text: TextComponent, overlay: bool) -> anyhow::Result<bool> {
        self.send_packet(Box::new(GameMessageS2C { text, overlay }))
    }
}

pub struct DefaultPacketHandler {
    pub stage: Stage,
    // For sending packets
    pub channel: UnboundedSender<Box<dyn PacketS2C + Send>>,
    // User profile
    pub profile: Profile
}

impl DefaultPacketHandler {
    pub fn new() -> DefaultPacketHandler {
        DefaultPacketHandler {
            stage: Stage::Handshake,
            channel: mpsc::unbounded_channel().0,  // placeholder channel
            profile: Profile {
                id: Uuid::from_u128(0u128),
                name: "Unknown".to_string(),
                properties: vec![]
            }
        }
    }

    pub fn set_channel(&mut self, sender: UnboundedSender<Box<dyn PacketS2C + Send>>) {
        self.channel = sender;
    }
}

#[derive(Debug, Copy, Clone)]
pub struct SendError {

}

impl SendError {
    pub fn new()  -> SendError {
        SendError {

        }
    }
}

impl Display for SendError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // TODO: verbosity
        f.write_str("Packet send error")
    }
}

impl Error for SendError {

}

#[async_trait]
impl PacketHandler for DefaultPacketHandler {
    // Setup
    fn set_channel(&mut self, sender: UnboundedSender<Box<dyn PacketS2C + Send>>) {
        self.channel = sender;
    }

    fn set_stage(&mut self, new_stage: Stage) {
        self.stage = new_stage;
    }

    fn get_stage(&self) -> &Stage {
        &self.stage
    }

    // Network actions
    fn send_packet(&mut self, packet: Box<dyn PacketS2C + Send>) -> anyhow::Result<bool> {
        match self.channel.send(packet) {
            Ok(_) => Ok(true),
            Err(_) => Err(SendError::new().into())
        }
    }

    async fn set_profile(&mut self, profile: Profile) {
        self.profile = profile;
    }

    async fn get_profile(&mut self) -> &Profile {
        &self.profile
    }
}
