use std::collections::{HashMap, HashSet};
use std::io::Cursor;
use std::path::Path;
use bytes::{BufMut, BytesMut};
use itertools::Itertools;
use lazy_static::lazy_static;
use log::warn;
use serde_derive::{Serialize, Deserialize};
use serde_json::{Value, Map};
use crate::block::Block;
use crate::server::utils::write_varint;
use crate::server::version::ProtocolVersion;
use crate::{Region, Tag};

trait StringToStringMap {
    fn string_to_string_map(&self) -> HashMap<String, String>;
}

impl StringToStringMap for HashMap<String, Value> {
    fn string_to_string_map(&self) -> HashMap<String, String> {
        self.iter()
            .map(|(k, v)| {
                let v = match v.clone() {
                    e @ Value::Number(_) | e @ Value::Bool(_) => e.to_string(),
                    Value::String(s) => s,
                    _ => {
                        warn!(r#"Warning : Can not convert field : "{}'s value to String, It will be empty string."#, k);
                        "".to_string()
                    }
                };

                (k.clone(), v)
            })
            .collect()
    }
}

trait StringToStringVecMap {
    fn string_to_string_map(&self) -> HashMap<String, Vec<String>>;
}

impl StringToStringVecMap for HashMap<String, Value> {
    fn string_to_string_map(&self) -> HashMap<String, Vec<String>> {
        self.iter()
            .map(|(k, v)| {
                let v = match v.clone() {
                    Value::Array(s) => s.iter().map(|v| {
                        match v.clone() {
                            Value::String(st) => st,
                            _ => "".to_string()
                        }
                    }).collect(),
                    _ => vec!["".to_string()]
                };

                (k.clone(), v)
            })
            .collect()
    }
}

#[derive(Deserialize, Serialize)]
struct BlockState {
    id: i32,
    properties: Option<HashMap<String, String>>,
    default: Option<bool>
}

#[derive(Deserialize, Serialize)]
struct BlockType {
    #[serde(default = "HashMap::new")]
    properties: HashMap<String, Vec<String>>,
    states: Vec<BlockState>
}

type Blocks = Map<String, Value>;

static BLOCKS_JSON: &str = include_str!("server/blocks.json");

lazy_static! {
    static ref BLOCKS: Blocks = serde_json::from_str(&BLOCKS_JSON).unwrap();
}

#[derive(Debug, Clone)]
pub struct SubChunk {
    pub(crate) palette: Vec<Tag>,
    pub(crate) blocks: Vec<u16>,
    pub(crate) block_light: Option<Vec<u8>>,
    pub(crate) sky_light: Option<Vec<u8>>,
}

impl SubChunk {
    pub fn empty() -> SubChunk {
        SubChunk {
            palette: vec![Block::new("minecraft:air", HashMap::new()).data],
            blocks: vec![0u16; 4096],
            block_light: Some(vec![255u8; 2048]),  // 2 per block, all 15
            sky_light: Some(vec![255u8; 2048]),
        }
    }

    pub fn new(data: &Tag) -> SubChunk {
        let tag = data.clone();
        let mut block_states = None;
        let mut palette = None;
        let mut block_light = None;
        let mut sky_light = None;
        if let Tag::Compound(root) = data {
            if let Some(states_tag) = root.get("block_states") {
                if let Tag::Compound(states) = states_tag {
                    if let Some(data_tag) = states.get("data") {
                        if let Tag::LongArray(data) = data_tag {
                            block_states = Some(data);
                        }
                    }
                    if let Some(palette_tag) = states.get("palette") {
                        if let Tag::List(palette_list) = palette_tag {
                            palette = Some(palette_list);
                        }
                    }
                }
            } else {
                if let Some(states_tag) = root.get("BlockStates") {
                    if let Tag::LongArray(states) = states_tag {
                        block_states = Some(states);
                    }
                }
                if let Some(palette_tag) = root.get("Palette") {
                    if let Tag::List(palette_list) = palette_tag {
                        palette = Some(palette_list);
                    }
                }
            }
            if let Some(light_tag) = root.get("BlockLight") {
                if let Tag::ByteArray(light) = light_tag {
                    block_light = Some(light.clone());
                }
            }
            if let Some(light_tag) = root.get("SkyLight") {
                if let Tag::ByteArray(light) = light_tag {
                    sky_light = Some(light.clone());
                }
            }
        }

        let cloned_palette;
        if let Some(palette_some) = palette {
            cloned_palette = palette_some.clone();
        } else {
            cloned_palette = Vec::<Tag>::new();
        }

        let block_vals;
        if let Some(blocks) = block_states {
            block_vals = Self::decode_blocks(&cloned_palette, blocks);
        } else {
            block_vals = Vec::<u16>::new();
        }

        Self { palette: cloned_palette, blocks: block_vals, block_light, sky_light }
    }

    fn decode_state(mut val: u64, bits: u32, mask: u64, per_state: u32) -> Vec<u16> {
        let mut result = Vec::<u16>::new();
        for i in 0..per_state {
            result.push((val & mask) as u16);
            val >>= bits;
        }
        result
    }

    fn decode_blocks(palette: &Vec<Tag>, states: &Vec<i64>) -> Vec<u16> {
        let bits = (palette.len().ilog2() + 1).max(4);
        let mask = (1u32 << bits) - 1u32;
        let per_state = 64 / bits;
        let mut blocks = Vec::<u16>::new();
        for num in states {
            blocks.append(&mut Self::decode_state(*num as u64, bits, mask as u64, per_state));
        }
        blocks
    }

    pub fn get_block(&self, x: u8, y: u8, z: u8) -> Option<Block> {
        if let Some(id) = self.blocks.get(((x as u16) + (z as u16) * 16 + (y as u16) * 256) as usize) {
            if let Some(block) = self.palette.get(*id as usize) {
                return Some(Block::from_nbt(block));
            }
        }
        None
    }
}


#[derive(Debug, Clone)]
pub struct Chunk {
    subchunks: HashMap<i8, SubChunk>
}

impl Chunk {
    pub fn empty() -> Chunk {
        let mut subchunks = HashMap::new();
        for i in -4..24 {
            subchunks.insert(i, SubChunk::empty());
        }
        Chunk { subchunks }
    }

    pub fn new(data: Tag) -> Chunk {
        let mut subchunks = Vec::<Tag>::new();
        if let Tag::Compound(root) = &data {
            if let Some(sections_tag) = root.get("sections") {
                if let Tag::List(sections) = sections_tag {
                    subchunks = sections.clone();
                }
            } else if let Some(level_tag) = root.get("Level") {
                if let Tag::Compound(level) = level_tag {
                    if let Some(sections_tag) = level.get("Sections") {
                        if let Tag::List(sections) = sections_tag {
                            subchunks = sections.clone();
                        }
                    }
                }
            }
        }
        let mut subchunks_loaded = HashMap::new();
        for subchunk in subchunks {
            subchunks_loaded.insert(subchunk.get("Y").unwrap().as_byte().unwrap(), SubChunk::new(&subchunk));
        }
        Self { subchunks: subchunks_loaded }
    }

    pub fn get_subchunk(&self, y: i8) -> Option<&SubChunk> {
        if let Some(subchunk) = self.subchunks.get(&y) {
            return Some(subchunk);
        }
        None
    }

    pub fn get_block(&self, x: u8, y: u16, z: u8) -> Option<Block> {
        let subchunk = self.get_subchunk((y / 16) as i8);
        if let Some(subchunk) = subchunk {
            return subchunk.get_block(x, (y % 16) as u8, z);
        }
        None
    }

    pub fn serialize_to_chunk_packet(&self, output: &mut BytesMut, v: ProtocolVersion) {
        let mut buf = BytesMut::new();
        let mut skylight = 0;
        let mut skylight_data = HashMap::new();
        let mut skylight_empty = 0;
        let mut blocklight = 0;
        let mut blocklight_data = HashMap::new();
        let mut blocklight_empty = 0;
        for cy in -4..20 {
            let mut full_block_count = 0;
            let mut blocks = vec![0u16; 4096];

            // Light
            let section_opt = self.get_subchunk(cy);
            if let Some(section) = section_opt {
                if let Some(block_light) = &section.block_light {
                    blocklight |= 1u64 << ((cy as i64) + 4);
                    blocklight_data.insert(cy + 4, block_light.clone());
                } else {
                    blocklight_empty |= 1u64 << ((cy as i64) + 4);
                }
                if let Some(sky_light) = &section.sky_light {
                    skylight |= 1u64 << ((cy as i64) + 4);
                    skylight_data.insert(cy + 4, sky_light.clone());
                } else {
                    skylight_empty |= 1u64 << ((cy as i64) + 4);
                }

                for y in 0..16 {
                    for z in 0..16 {
                        for x in 0..16 {
                            let idx = (((y as u16) * 16 + (z as u16)) * 16) + (x as u16);
                            let block = section.get_block(x, y, z).unwrap_or_else(|| {
                                let mut tag = HashMap::new();
                                tag.insert("Name".to_owned(), Tag::String("minecraft:air".to_owned()));
                                Block { data: Tag::Compound(tag) }
                            });
                            let block_name = block.name().cloned().unwrap_or_else(|| "minecraft:air".to_string());
                            if block_name != "minecraft:air" {
                                full_block_count += 1;
                            }

                            let block_def = BLOCKS.get(&block_name);
                            let mut block_id = 0;
                            if block_name != "minecraft:air" {
                                if let Some(block_type_val) = block_def {
                                    let block_type: BlockType = serde_json::from_value(block_type_val.clone()).unwrap();
                                    let mut default = None;
                                    for cond in block_type.states {
                                        if let Some(def) = cond.default {
                                            if def {
                                                default = Some(cond.id);
                                            }
                                        }

                                        let mut pm = Vec::new();
                                        if let Some(props) = cond.properties {
                                            for (k, v) in props {
                                                if let Some(p) = block.get_property(&k) {
                                                    pm.push(*(p.as_string().cloned().unwrap_or_else(|_| match p.as_byte().unwrap_or_else(|_| -1i8) {
                                                        0 => "true",
                                                        1 => "false",
                                                        _ => "???"
                                                    }.to_owned())) == v);
                                                } else {
                                                    pm.push(false);
                                                }
                                            }
                                        }

                                        if pm.len() > 0 && pm.iter().all(|i| *i) {
                                            block_id = cond.id;
                                            break;
                                        };
                                    }

                                    if block_id == 0 {
                                        block_id = default.unwrap_or_else(|| 0);
                                    }
                                }
                                blocks[idx as usize] = block_id as u16;
                            }
                        }
                    }
                }

                buf.put_u16(full_block_count);

                if *blocks.iter().max().unwrap() == 0u16 {
                    buf.put_bytes(0, 3);  // bpe, air, empty array
                } else {
                    let mut palette: Vec<&u16> = blocks.iter().unique().sorted().collect();
                    // println!("palette: {:?}", palette);
                    let bpe = ((palette.len() as f32).log2().ceil() as u32).max(4);
                    println!("{} for {}", bpe, palette.len());
                    println!("{:?}", palette);
                    // println!("bpe={bpe} for {}", palette.len());
                    if bpe > 15 {
                        panic!("tried to serialize a chunk with a bpe > 15");
                    }
                    let elems_per_num = 64 / bpe;
                    // println!("elems_per_num={elems_per_num}");
                    let num_elems = (4096f32 / (elems_per_num as f32)).ceil() as u32;
                    // println!("num_elems={num_elems}");
                    let mut data = vec![0u64; num_elems as usize];
                    for i in 0..num_elems {
                        let mut e = 0;
                        for j in 0..elems_per_num {
                            if i * elems_per_num + j >= 4096 {
                                break;
                            }

                            e |= ((palette.iter().position(|&b| *b == blocks[(i * elems_per_num + j) as usize]).unwrap() as u64) << (bpe * j));
                        }
                        data[i as usize] = e;
                    }

                    buf.put_u8(bpe as u8);
                    write_varint(&mut buf, palette.len() as i32);
                    for p in palette {
                        write_varint(&mut buf, *p as i32);
                    }
                    write_varint(&mut buf, data.len() as i32);
                    for l in data {
                        buf.put_u64(l);
                    }
                }

                buf.put_u8(0);  // biomes NYI, TODO
                write_varint(&mut buf, 39);  // plains
                write_varint(&mut buf, 0);  // empty array
            } else {
                blocklight_empty |= 1u64 << ((cy as u64) + 4);
                skylight_empty |= 1u64 << ((cy as u64) + 4);
            }
        }

        write_varint(output, buf.len() as i32);
        output.put(buf);

        write_varint(output, 0);  // no block entities

        if v <= ProtocolVersion::V1_19_4 {
            output.put_u8(1);  // trust edges
        }

        write_varint(output, 1);  // 64 bits skylight mask
        output.put_u64(skylight);
        write_varint(output, 1);  // 64 bits blocklight mask
        output.put_u64(blocklight);
        write_varint(output, 1);  // 64 bits skylight empty mask
        output.put_u64(skylight_empty);
        write_varint(output, 1);  // 64 bits blocklight empty mask
        output.put_u64(blocklight_empty);

        write_varint(output, skylight_data.len() as i32);
        for i in 0..24 {
            let val = skylight_data.get(&i);
            if let Some(v) = val {
                write_varint(output, v.len() as i32);
                output.put(&v[..]);
            }
        }
        write_varint(output, blocklight_data.len() as i32);
        for i in 0..24 {
            let val = blocklight_data.get(&i);
            if let Some(v) = val {
                write_varint(output, v.len() as i32);
                output.put(&v[..]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use std::path::Path;
    use bytes::BytesMut;
    use crate::Region;
    use crate::server::version::ProtocolVersion;

    #[test]
    fn test_chunk_data() {
        let mut reg = include_bytes!("../server/world/region/r.0.0.mca");
        let mut region = Region::load(Cursor::new(&mut reg));
        let mut output = BytesMut::new();
        for x in 0..4 {
            for z in 0..4 {
                let chunk = region.get_chunk(x, z).unwrap();
                chunk.serialize_to_chunk_packet(&mut output, ProtocolVersion::V1_20_4);
                // println!("{:?}", output);
            }
        }
        std::fs::write(Path::new("r.0.0.chunks"), output).unwrap();
    }
}