#[derive(Debug, Copy, Clone, Hash, PartialEq)]
pub enum Stage {
    Handshake,
    Status,
    Login,
    Config,  // New in 1.20.2
    Play,
    Transfer,
    Invalid
}

impl Stage {
    pub fn from_id(id: i32) -> Stage {
        match id {
            1 => Stage::Status,
            2 => Stage::Login,
            3 => Stage::Transfer,  // New in 1.21
            _ => Stage::Invalid
        }
    }
}
