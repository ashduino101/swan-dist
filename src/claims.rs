use serde_derive::{Serialize, Deserialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claim {
    pub(crate) x1: i32,
    pub(crate) z1: i32,
    pub(crate) x2: i32,
    pub(crate) z2: i32,
    pub(crate) timestamp: u64
}

pub fn get_claims(_: Uuid) -> Vec<Claim> {
    // TODO: this is a demo value
    vec![Claim {x1: 0, z1: 13, x2: 17, z2: 54, timestamp: 0}, Claim {x1: -20, z1: -30, x2: -4, z2: -7, timestamp: 0}]
}