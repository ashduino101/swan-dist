
#[derive(Debug, Clone)]
pub enum ChatVisibility {
    Full,
    System,
    Hidden
}

impl ChatVisibility {
    pub fn from_i32(val: i32) -> ChatVisibility {
        match val {
            0 => ChatVisibility::Full,
            1 => ChatVisibility::System,
            _ => ChatVisibility::Hidden,  // 2 or default, just in case
        }
    }
}


#[derive(Debug, Clone)]
pub enum Arm {
    Left,
    Right
}

impl Arm {
    pub fn from_i32(val: i32) -> Arm {
        match val {
            0 => Arm::Left,
            _ => Arm::Right,
        }
    }
}
