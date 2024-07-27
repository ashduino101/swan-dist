use std::collections::HashMap;
use colorgrad::Color;
use serde_derive::Serialize;
use crate::Tag;

const FORMAT_CHAR: &str = "§";

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ClickEventType {
    OpenUrl,
    RunCommand,
    SuggestCommand,
    ChangePage,
    CopyToClipboard
}

#[derive(Debug, Serialize, Clone)]
pub struct ClickEvent {
    action: ClickEventType,
    value: String
}

#[derive(Debug, Serialize, Clone)]
pub struct ShowItemEvent {
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tag: Option<String>  // sNBT
}

#[derive(Debug, Serialize, Clone)]
pub struct ShowEntityEvent {
    #[serde(rename = "type")]
    r#type: String,
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "snake_case", tag = "action", content = "contents")]
pub enum HoverEvent {
    ShowText(TextComponent),
    ShowItem(ShowItemEvent),
    ShowEntity(ShowEntityEvent)
}

impl HoverEvent {
    pub fn show_text(text: TextComponent) -> HoverEvent {
        HoverEvent::ShowText(text)
    }

    pub fn show_item(id: String, count: Option<i32>, nbt: Option<String>) -> HoverEvent {
        HoverEvent::ShowItem(ShowItemEvent { id, count, tag: nbt })
    }

    pub fn show_entity(r#type: String, id: String, name: Option<String>) -> HoverEvent {
        HoverEvent::ShowEntity(ShowEntityEvent { r#type, id, name })
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct Score {
    name: String,
    objective: String
}

#[derive(Debug, Clone)]
pub enum ChatColor {
    Black,
    DarkBlue,
    DarkGreen,
    DarkCyan,
    DarkRed,
    Purple,
    Gold,
    Gray,
    DarkGray,
    Blue,
    Green,
    Aqua,
    Red,
    LightPurple,
    Yellow,
    White,
    Custom(String)
}

impl ChatColor {
    pub fn as_format_code(&self) -> String {
        if let ChatColor::Custom(hex) = self {
            return format!("#{}", hex);
        }
        format!("{}{}", FORMAT_CHAR, match self {
            ChatColor::Black => "0",
            ChatColor::DarkBlue => "1",
            ChatColor::DarkGreen => "2",
            ChatColor::DarkCyan => "3",
            ChatColor::DarkRed => "4",
            ChatColor::Purple => "5",
            ChatColor::Gold => "6",
            ChatColor::Gray => "7",
            ChatColor::DarkGray => "8",
            ChatColor::Blue => "9",
            ChatColor::Green => "a",
            ChatColor::Aqua => "b",
            ChatColor::Red => "c",
            ChatColor::LightPurple => "d",
            ChatColor::Yellow => "e",
            _ => "f"
        })
    }

    pub fn get_name(&self) -> String {
        if let ChatColor::Custom(hex) = self {
            return format!("#{}", hex);
        }
        match self {
            ChatColor::Black => "black",
            ChatColor::DarkBlue => "dark_blue",
            ChatColor::DarkGreen => "dark_green",
            ChatColor::DarkCyan => "dark_aqua",
            ChatColor::DarkRed => "dark_red",
            ChatColor::Purple => "dark_purple",
            ChatColor::Gold => "gold",
            ChatColor::Gray => "gray",
            ChatColor::DarkGray => "dark_gray",
            ChatColor::Blue => "blue",
            ChatColor::Green => "green",
            ChatColor::Aqua => "aqua",
            ChatColor::Red => "red",
            ChatColor::LightPurple => "light_purple",
            ChatColor::Yellow => "yellow",
            _ => "white"
        }.to_owned()
    }

    pub fn as_rgba(&self) -> (u8, u8, u8, u8) {
        match self {
            ChatColor::Black => (0x00, 0x00, 0x00, 0xff),
            ChatColor::DarkBlue => (0x00, 0x00, 0xaa, 0xff),
            ChatColor::DarkGreen => (0x00, 0xaa, 0x00, 0xff),
            ChatColor::DarkCyan => (0x00, 0xaa, 0xaa, 0xff),
            ChatColor::DarkRed => (0xaa, 0x00, 0x00, 0xff),
            ChatColor::Purple => (0xaa, 0x00, 0xaa, 0xff),
            ChatColor::Gold => (0xff, 0xaa, 0x00, 0xff),
            ChatColor::Gray => (0xaa, 0xaa, 0xaa, 0xff),
            ChatColor::DarkGray => (0x55, 0x55, 0x55, 0xff),
            ChatColor::Blue => (0x55, 0x55, 0xff, 0xff),
            ChatColor::Green => (0x55, 0xff, 0x55, 0xff),
            ChatColor::Aqua => (0x55, 0xff, 0xff, 0xff),
            ChatColor::Red => (0xff, 0x55, 0x55, 0xff),
            ChatColor::LightPurple => (0xff, 0x55, 0xff, 0xff),
            ChatColor::Yellow => (0xff, 0xff, 0x55, 0xff),
            ChatColor::White => (0xff, 0xff, 0xff, 0xff),
            ChatColor::Custom(h) => (
                u8::from_str_radix(&h[0..2], 16).unwrap(),
                u8::from_str_radix(&h[2..4], 16).unwrap(),
                u8::from_str_radix(&h[4..6], 16).unwrap(),
                0xff
                )
        }
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct TextComponent {
    #[serde(rename = "type")]
    r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    extra: Option<Vec<Box<TextComponent>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bold: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    italic: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    font: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    underlined: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    strikethrough: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    obfuscated: Option<bool>,
    // Events
    #[serde(skip_serializing_if = "Option::is_none", rename = "clickEvent")]
    click_event: Option<ClickEvent>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "hoverEvent")]
    hover_event: Option<Box<HoverEvent>>,
    // Translate
    #[serde(skip_serializing_if = "Option::is_none")]
    translate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    with: Option<Vec<Box<TextComponent>>>,
    // Keybind
    #[serde(skip_serializing_if = "Option::is_none")]
    keybind: Option<String>,
    // Score
    #[serde(skip_serializing_if = "Option::is_none")]
    score: Option<Score>,
    // Selector
    #[serde(skip_serializing_if = "Option::is_none")]
    selector: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    separator: Option<Box<TextComponent>>,  // also used in NBT
    // NBT
    #[serde(skip_serializing_if = "Option::is_none")]
    nbt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    interpret: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    block: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    entity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    storage: Option<String>
}

impl TextComponent {
    pub fn new() -> TextComponent {
        TextComponent {
            r#type: "text".to_owned(),
            extra: None,
            text: None,
            color: None,
            bold: None,
            italic: None,
            font: None,
            underlined: None,
            strikethrough: None,
            obfuscated: None,
            click_event: None,
            hover_event: None,
            translate: None,
            with: None,
            keybind: None,
            score: None,
            selector: None,
            separator: None,
            nbt: None,
            interpret: None,
            block: None,
            entity: None,
            storage: None
        }
    }

    pub fn plain(text: &str) -> TextComponent {
        let mut component = TextComponent::new();
        component.set_text(text);
        component
    }

    /// Adds a sibling component to this component.
    pub fn add_component(&mut self, component: TextComponent) {
        if self.extra.is_none() {
            self.extra = Some(Vec::new())
        }
        if let Some(ref mut extra) = &mut self.extra {
            extra.push(Box::new(component));
        }
    }

    /// Prepends a sibling component to this component.
    pub fn prepend_component(&mut self, component: TextComponent) {
        if self.extra.is_none() {
            self.extra = Some(Vec::new())
        }
        if let Some(ref mut extra) = &mut self.extra {
            extra.insert(0, Box::new(component));
        }
    }

    /// Sets the content of this component.
    pub fn set_text(&mut self, text: &str) {
        self.text = Some(text.to_owned())
    }

    pub fn set_bold(&mut self, bold: bool) {
        self.bold = Some(bold)
    }

    pub fn set_italic(&mut self, italic: bool) {
        self.bold = Some(italic)
    }

    pub fn set_underlined(&mut self, underlined: bool) {
        self.underlined = Some(underlined)
    }

    pub fn set_strikethrough(&mut self, strikethrough: bool) {
        self.strikethrough = Some(strikethrough)
    }

    pub fn set_obfuscated(&mut self, obfuscated: bool) {
        self.obfuscated = Some(obfuscated)
    }

    /// Sets the static color of the text.
    pub fn set_color(&mut self, color: ChatColor) {
        self.color = Some(color.get_name());
    }

    /// Colors the current text to a gradient, adding the necessary components. (1.16+)
    pub fn set_gradient(&mut self, points: &[ChatColor]) {
        if self.text.is_none() {
            return;
        }
        let points: Vec<Color> = points.iter().map(|p| {
            let rgba = p.as_rgba();
            Color::from_rgba8(rgba.0, rgba.1, rgba.2, rgba.3)
        }).collect();

        let g = colorgrad::CustomGradient::new()
            .colors(&points)
            .build().unwrap();

        let text = self.text.clone().unwrap();
        let mut i = 0f64;
        let mut components = Vec::new();
        for ch in text.chars() {
            let idx = i / text.len() as f64;
            let color = g.at(idx);
            let mut component = TextComponent::new();  // will inherit properties by default
            component.set_text(ch.to_string().as_str());
            component.set_color(ChatColor::Custom(String::from(&color.to_hex_string()[1..])));
            components.push(Box::new(component));
            i += 1.0;
        }
        self.extra = Some(components);
        self.text = Some("".to_owned());
    }

    pub fn set_hover_event(&mut self, event: HoverEvent) {
        self.hover_event = Some(Box::new(event));
    }

    pub fn to_nbt(&self) -> Tag {
        let mut root = HashMap::new();
        root.insert("type".to_owned(), Tag::String(self.r#type.clone()));
        if let Some(extra) = &self.extra {
            root.insert("extra".to_owned(), Tag::List(extra.iter().map(|t| t.to_nbt()).collect()));
        }

        if let Some(color) = &self.color {
            root.insert("color".to_owned(), Tag::String(color.clone()));
        }

        if let Some(text) = &self.text {
            root.insert("text".to_owned(), Tag::String(text.clone()));
        }

        if let Some(bold) = self.bold {
            root.insert("bold".to_owned(), Tag::Byte(if bold { 1 } else { 0 }));
        }
        if let Some(italic) = self.italic {
            root.insert("italic".to_owned(), Tag::Byte(if italic { 1 } else { 0 }));
        }
        if let Some(underlined) = self.underlined {
            root.insert("underlined".to_owned(), Tag::Byte(if underlined { 1 } else { 0 }));
        }
        if let Some(strikethrough) = self.strikethrough {
            root.insert("strikethrough".to_owned(), Tag::Byte(if strikethrough { 1 } else { 0 }));
        }
        if let Some(obfuscated) = self.obfuscated {
            root.insert("obfuscated".to_owned(), Tag::Byte(if obfuscated { 1 } else { 0 }));
        }

        Tag::Compound(root)
    }
}