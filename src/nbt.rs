use std::collections::HashMap;
use bytes::{Buf, BufMut, Bytes, BytesMut};
use log::kv::Source;
use crate::server::version::ProtocolVersion;

#[derive(Clone, Copy)]
#[derive(PartialEq)]
pub enum TagType {
    End = 0x00,
    Byte = 0x01,
    Short = 0x02,
    Int = 0x03,
    Long = 0x04,
    Float = 0x05,
    Double = 0x06,
    ByteArray = 0x07,
    String = 0x08,
    List = 0x09,
    Compound = 0x0A,
    IntArray = 0x0B,
    LongArray = 0x0C,

    Invalid = -1
}

impl TagType {
    fn from_id(tag: u8) -> TagType {
        match tag {
            0 => TagType::End,
            1 => TagType::Byte,
            2 => TagType::Short,
            3 => TagType::Int,
            4 => TagType::Long,
            5 => TagType::Float,
            6 => TagType::Double,
            7 => TagType::ByteArray,
            8 => TagType::String,
            9 => TagType::List,
            10 => TagType::Compound,
            11 => TagType::IntArray,
            12 => TagType::LongArray,
            _ => TagType::Invalid
        }
    }
}

#[derive(Debug)]
pub struct TagError { }

#[derive(Debug, Clone)]
pub enum Tag {
    End,
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    ByteArray(Vec<u8>),
    String(String),
    List(Vec<Tag>),
    Compound(HashMap<String, Tag>),
    IntArray(Vec<i32>),
    LongArray(Vec<i64>),

    Invalid
}

impl Tag {
    pub fn parse(data: &mut Bytes) -> Self {
        Self::parse_nbt(data, false)
    }

    pub fn parse_network(data: &mut Bytes, v: ProtocolVersion) -> Self {
        Self::parse_nbt(data, v >= ProtocolVersion::V1_20_2)
    }

    fn parse_string(data: &mut Bytes) -> String {
        let length = data.get_u16() as usize;
        let b = data.slice(0..length);
        data.advance(length);
        String::from_utf8(b.to_vec()).expect("failed to parse string")
    }

    fn parse_tag(tag_type: TagType, data: &mut Bytes) -> Tag {
        match tag_type {
            TagType::End => Tag::End,
            TagType::Byte => Tag::Byte(data.get_i8()),
            TagType::Short => Tag::Short(data.get_i16()),
            TagType::Int => Tag::Int(data.get_i32()),
            TagType::Long => Tag::Long(data.get_i64()),
            TagType::Float => Tag::Float(data.get_f32()),
            TagType::Double => Tag::Double(data.get_f64()),
            TagType::ByteArray => {
                let size = data.get_i32() as usize;
                let r = Tag::ByteArray(data.slice(0..size).to_vec());
                data.advance(size);
                r
            },
            TagType::String => Tag::String(Self::parse_string(data)),
            TagType::List => {
                let tag_type = TagType::from_id(data.get_u8());
                let size = data.get_i32();
                let mut value = Vec::<Tag>::new();
                for _ in 0..size {
                    value.push(Self::parse_tag(tag_type, data));
                }
                Tag::List(value)
            },
            TagType::Compound => {
                let mut value = HashMap::new();
                loop {
                    let tag_type = TagType::from_id(data.get_u8());
                    if tag_type == TagType::End {
                        break;
                    }
                    let name = Self::parse_string(data);
                    let tag = Self::parse_tag(tag_type, data);
                    value.insert(name, tag);
                }
                Tag::Compound(value)
            },
            TagType::IntArray => {
                let size = data.get_i32();
                let mut value = Vec::<i32>::new();
                for _ in 0..size {
                    value.push(data.get_i32());
                }
                Tag::IntArray(value)
            },
            TagType::LongArray => {
                let size = data.get_i32();
                let mut value = Vec::<i64>::new();
                for _ in 0..size {
                    value.push(data.get_i64());
                }
                Tag::LongArray(value)
            }

            _ => Tag::Invalid
        }
    }

    fn parse_nbt(data: &mut Bytes, no_root_name: bool) -> Tag {
        let root = TagType::from_id(data.get_u8());
        if !no_root_name {
            Self::parse_string(data);
        }
        Self::parse_tag(root, data)
    }

    fn get_type_id(&self) -> u8 {
        match self {
            Tag::End => 0,
            Tag::Byte(_) => 1,
            Tag::Short(_) => 2,
            Tag::Int(_) => 3,
            Tag::Long(_) => 4,
            Tag::Float(_) => 5,
            Tag::Double(_) => 6,
            Tag::ByteArray(_) => 7,
            Tag::String(_) => 8,
            Tag::List(_) => 9,
            Tag::Compound(_) => 10,
            Tag::IntArray(_) => 11,
            Tag::LongArray(_) => 12,
            _ => 0
        }
    }

    fn serialize_internal(&self, buf: &mut BytesMut) {
        match self {
            Tag::End => {},
            Tag::Byte(v) => buf.put_i8(*v),
            Tag::Short(v) => buf.put_i16(*v),
            Tag::Int(v) => buf.put_i32(*v),
            Tag::Long(v) => buf.put_i64(*v),
            Tag::Float(v) => buf.put_f32(*v),
            Tag::Double(v) => buf.put_f64(*v),
            Tag::ByteArray(v) => {
                buf.put_i32(v.len() as i32);
                buf.put(&v[..]);
            },
            Tag::String(v) => {
                buf.put_u16(v.len() as u16);
                buf.put(v.as_bytes());
            },
            Tag::List(v) => {
                buf.put_u8(v.get(0).unwrap_or_else(|| &Tag::End).get_type_id());
                buf.put_i32(v.len() as i32);
                for tag in v {
                    tag.serialize_internal(buf);
                }
            },
            Tag::Compound(c) => {
                for (k, v) in c {
                    buf.put_u8(v.get_type_id());
                    buf.put_u16(k.len() as u16);
                    buf.put(k.as_bytes());
                    v.serialize_internal(buf);
                }
                buf.put_u8(0);  // end tag
            },
            Tag::IntArray(v) => {
                buf.put_i32(v.len() as i32);
                for val in v {
                    buf.put_i32(*val);
                }
            },
            Tag::LongArray(v) => {
                buf.put_i32(v.len() as i32);
                for val in v {
                    buf.put_i64(*val);
                }
            }
            _ => {}
        }
    }

    pub fn serialize(&self, buf: &mut BytesMut, is_network_1_20_2_plus: bool) {
        buf.put_u8(self.get_type_id());
        if !is_network_1_20_2_plus {
            buf.put_u16(0);  // empty string
        }
        self.serialize_internal(buf);
    }

    pub fn traverse(&self, path: &str) -> Option<&Tag> {
        let parts: Vec<&str> = path.split("/").collect();
        let mut cur = self;
        for (i, part) in parts.iter().enumerate() {
            if let Tag::Compound(cur_tag) = cur {
                if let Some(tag) = cur_tag.get(*part) {
                    if i == (parts.len() - 1) {
                        return Some(tag);
                    } else {
                        cur = tag;
                    }
                } else {
                    break;
                }
            }
        }
        None
    }

    pub fn as_byte(&self) -> Result<i8, TagError> {
        if let Tag::Byte(v) = self { Ok(*v) } else { Err(TagError {}) }
    }

    pub fn as_short(&self) -> Result<i16, TagError> {
        if let Tag::Short(v) = self { Ok(*v) } else { Err(TagError {}) }
    }

    pub fn as_int(&self) -> Result<i32, TagError> {
        if let Tag::Int(v) = self { Ok(*v) } else { Err(TagError {}) }
    }

    pub fn as_long(&self) -> Result<i64, TagError> {
        if let Tag::Long(v) = self { Ok(*v) } else { Err(TagError {}) }
    }

    pub fn as_float(&self) -> Result<f32, TagError> {
        if let Tag::Float(v) = self { Ok(*v) } else { Err(TagError {}) }
    }

    pub fn as_double(&self) -> Result<f64, TagError> {
        if let Tag::Double(v) = self { Ok(*v) } else { Err(TagError {}) }
    }

    pub fn as_byte_array(&self) -> Result<&Vec<u8>, TagError> {
        if let Tag::ByteArray(v) = self { Ok(v) } else { Err(TagError {}) }
    }

    pub fn as_string(&self) -> Result<&String, TagError> {
        if let Tag::String(v) = self { Ok(v) } else { Err(TagError {}) }
    }

    pub fn as_list(&self) -> Result<&Vec<Tag>, TagError> {
        if let Tag::List(v) = self { Ok(v) } else { Err(TagError {}) }
    }

    pub fn as_compound(&self) -> Result<&HashMap<String, Tag>, TagError> {
        if let Tag::Compound(v) = self { Ok(v) } else { Err(TagError {}) }
    }

    pub fn as_int_array(&self) -> Result<&Vec<i32>, TagError> {
        if let Tag::IntArray(v) = self { Ok(v) } else { Err(TagError {}) }
    }

    pub fn as_long_array(&self) -> Result<&Vec<i64>, TagError> {
        if let Tag::LongArray(v) = self { Ok(v) } else { Err(TagError {}) }
    }

    pub fn get(&self, key: &str) -> Result<&Tag, TagError> {
        self.as_compound()?.get(key).ok_or(TagError {})
    }
}

#[test]
fn test_nbt() {
    let mut map = HashMap::new();
    map.insert("byte".to_owned(), Tag::Byte(0));
    map.insert("short".to_owned(), Tag::Short(3453));
    map.insert("int".to_owned(), Tag::Int(34543346));
    map.insert("long".to_owned(), Tag::Long(43624578963498));
    map.insert("float".to_owned(), Tag::Float(0.34545));
    map.insert("double".to_owned(), Tag::Double(0.437853467834));
    map.insert("bytearray".to_owned(), Tag::ByteArray(vec![0u8, 1, 2, 3]));
    map.insert("string".to_owned(), Tag::String("abcdefg".to_owned()));
    map.insert("list".to_owned(), Tag::List(vec![Tag::Int(0), Tag::Int(6)]));
    let mut nested = HashMap::new();
    nested.insert("a".to_owned(), Tag::Int(0));
    nested.insert("b".to_owned(), Tag::Int(1));
    nested.insert("c".to_owned(), Tag::Int(2));
    map.insert("compound".to_owned(), Tag::Compound(nested));
    map.insert("intarray".to_owned(), Tag::IntArray(vec![1, 2, 3, 4, 5, 6, 7, 8]));
    map.insert("longarray".to_owned(), Tag::LongArray(vec![3425673454634, 346568485667542, 869273876787, 237846328437]));
    let tag = Tag::Compound(map);
    println!("{:?}", tag);
    let mut buf = BytesMut::new();
    tag.serialize(&mut buf, false);
    println!("{:?}", buf);
    let new_tag = Tag::parse(&mut Bytes::from(buf));
    println!("{:?}", new_tag);
}