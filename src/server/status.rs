use std::io::Cursor;
use base64::Engine;
use base64::engine::general_purpose;
use image::{DynamicImage, ImageFormat};
use image::imageops::FilterType;
use serde_derive::Serialize;
use crate::server::text::TextComponent;
use crate::server::version::ProtocolVersion;

#[derive(Serialize)]
pub struct StatusVersion {
    name: String,
    protocol: i32
}

#[derive(Serialize)]
pub struct PlayerSample {
    name: String,
    id: String
}

impl PlayerSample {
    pub fn new(name: String, uuid: String) -> PlayerSample {
        PlayerSample { name, id: uuid, }
    }
}

#[derive(Serialize)]
pub struct StatusPlayers {
    max: i32,
    online: i32,
    sample: Vec<PlayerSample>
}

#[derive(Serialize)]
pub struct StatusBuilder {
    version: StatusVersion,
    #[serde(skip_serializing_if = "Option::is_none")]
    players: Option<StatusPlayers>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<TextComponent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    favicon: Option<String>,
    #[serde(rename = "enforcesSecureChat")]
    secure_chat: bool,
    #[serde(rename = "previewsChat")]
    preview_chat: bool
}

impl StatusBuilder {
    pub fn new(version: ProtocolVersion) -> StatusBuilder {
        StatusBuilder {
            version: StatusVersion {
                name: version.get_name(),
                protocol: version.get_id()
            },
            players: Some(StatusPlayers {
                max: 0,
                online: 0,
                sample: Vec::new()
            }),
            description: None,
            favicon: None,
            secure_chat: false,
            preview_chat: false
        }
    }

    pub fn with_description(&mut self, description: TextComponent) -> &mut StatusBuilder {
        self.description = Some(description);
        self
    }

    pub fn with_plain_description(&mut self, description: &str) -> &mut StatusBuilder {
        let mut comp = TextComponent::new();
        comp.set_text(description);
        self.description = Some(comp);
        self
    }

    pub fn with_favicon(&mut self, icon: DynamicImage) -> &mut StatusBuilder {
        let mut bytes: Vec<u8> = Vec::new();
        let icon = icon.resize_exact(64, 64, FilterType::Lanczos3);
        // FIXME error handling
        icon.write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png).expect("invalid favicon");
        self.favicon = Some(format!("data:image/png;base64,{}", general_purpose::STANDARD.encode(bytes)));
        self
    }

    pub fn with_secure_chat(&mut self) -> &mut StatusBuilder {
        self.secure_chat = true;
        self
    }

    pub fn with_chat_preview(&mut self) -> &mut StatusBuilder {
        self.preview_chat = true;
        self
    }
    
    pub fn with_player_sample(&mut self, max: i32, online: i32, sample: Vec<PlayerSample>) -> &mut StatusBuilder {
        self.players = Some(StatusPlayers {
            max,
            online,
            sample
        });
        self
    }

    pub fn finish(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}
