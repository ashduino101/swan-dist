use std::collections::HashMap;
use crate::Tag;

#[derive(Debug, Clone)]
pub struct Block {
    pub(crate) data: Tag
}

impl Block {
    pub fn new(name: &str, properties: HashMap<String, String>) -> Block {
        let mut data = HashMap::new();
        data.insert("Name".to_owned(), Tag::String(name.to_owned()));
        let mut props = HashMap::new();
        for (key, p) in properties {
            props.insert(key, Tag::String(p));
        }
        data.insert("Properties".to_owned(), Tag::Compound(props));

        Block { data: Tag::Compound(data) }
    }

    pub fn from_nbt(nbt: &Tag) -> Block {
        Self { data: nbt.clone() }
    }

    pub fn name(&self) -> Option<&String> {
        if let Tag::Compound(root) = &self.data {
            if let Some(name_tag) = root.get("Name") {
                if let Tag::String(name) = name_tag {
                    return Some(name);
                }
            }
        }
        None
    }

    pub fn get_property(&self, name: &str) -> Option<Tag> {
        let val = self.data.get("Properties").cloned().unwrap_or_else(|_| Tag::Compound(HashMap::new())).get(name).cloned();
        if let Ok(v) = val {
            Some(v)
        } else {
            None
        }
    }

}
