use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use byteorder::{BigEndian, ReadBytesExt};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use flate2::read::{GzDecoder, ZlibDecoder};
use crate::chunk::Chunk;
use crate::Tag;

#[derive(Debug, Copy, Clone)]
struct ChunkInfo {
    offset: u32,
    sectors: u8
}

impl ChunkInfo {
    pub fn new(offset: u32, sectors: u8) -> ChunkInfo {
        ChunkInfo { offset, sectors }
    }

    pub fn get_index(chunk_x: i32, chunk_z: i32) -> usize {
        ((chunk_z << 5) + chunk_x) as usize
    }
}

#[derive(Debug, Clone)]
struct RegionHeader {
    chunks: Vec<ChunkInfo>,
    timestamps: Vec<u32>
}

impl RegionHeader {
    pub fn new() -> RegionHeader {
        RegionHeader {
            chunks: vec![ChunkInfo { offset: 0, sectors: 0 }; 1024],
            timestamps: vec![0u32; 1024]
        }
    }
}

pub struct Region<R: Read + Seek> {
    file: R,
    header: RegionHeader
}

impl<R: Read + Seek> Region<R> {
    pub fn open(path: &str) -> Region<File> {
        let f = File::open(path).expect("could not open region file for reading");
        Region::<File>::load(f)
    }

    pub fn load(mut file: R) -> Region<R> {
        file.rewind().expect("failed to seek");
        let mut header_buf = [0u8; 8192];
        file.read(&mut header_buf).expect("failed to read");
        let mut header = Bytes::from(header_buf.to_vec());
        let mut chunks = Vec::<ChunkInfo>::new();
        for _ in 0..1024 {
            chunks.push(ChunkInfo { offset: header.get_uint(3) as u32, sectors: header.get_u8() });
        }
        let mut timestamps = Vec::<u32>::new();
        for _ in 0..1024 {
            timestamps.push(header.get_u32());
        }

        Self {
            file,
            header: RegionHeader {
                chunks,
                timestamps
            }
        }
    }

    pub fn get_timestamp(&self, chunk_x: i32, chunk_z: i32) -> Option<&u32> {
        self.header.timestamps.get(ChunkInfo::get_index(chunk_x, chunk_z))
    }

    /// Gets the raw chunk data, without performing decompression
    pub fn get_chunk_raw(&mut self, chunk_x: i32, chunk_z: i32) -> Option<Vec<u8>> {
        let index = ChunkInfo::get_index(chunk_x, chunk_z);
        let info_some = self.header.chunks.get(index);
        if let Some(info) = info_some {
            if info.offset == 0 || info.sectors == 0 {
                return None;
            }
            self.file.seek(SeekFrom::Start((info.offset * 4096) as u64)).expect("cannot seek");
            let length = self.file.read_u32::<BigEndian>().expect("cannot read");

            let mut raw = Vec::<u8>::new();
            raw.resize((length + 1) as usize, 0u8);

            self.file.read_exact(&mut raw).expect("cannot read");

            Some(raw)
        } else {
            None
        }
    }

    pub fn get_chunk_data(&mut self, chunk_x: i32, chunk_z: i32) -> Option<Bytes> {
        let index = ChunkInfo::get_index(chunk_x, chunk_z);
        let info_some = self.header.chunks.get(index);
        if let Some(info) = info_some {
            if info.offset == 0 || info.sectors == 0 {
                return None;
            }
            self.file.seek(SeekFrom::Start((info.offset * 4096) as u64)).expect("cannot seek");
            let length = self.file.read_u32::<BigEndian>().expect("cannot read");
            let comp_method = self.file.read_u8().expect("cannot read");

            let mut raw = Vec::<u8>::new();
            raw.resize(length as usize, 0u8);

            self.file.read_exact(&mut raw).expect("cannot read");

            let mut data = match comp_method {
                1 => {  // GZip
                    let mut dec = GzDecoder::new(&raw[..]);
                    let mut out = Vec::<u8>::new();
                    dec.read_to_end(&mut out).expect("TODO: panic message");
                    Bytes::from(out)
                },
                2 => {  // Zlib
                    let mut dec = ZlibDecoder::new(&raw[..]);
                    let mut out = Vec::<u8>::new();
                    dec.read_to_end(&mut out).expect("TODO: panic message");
                    Bytes::from(out)
                },
                3 => {  // Uncompressed
                    Bytes::from(raw)
                },
                _ => panic!("invalid compression method")
            };

            Some(data)
        } else { None }
    }

    pub fn get_chunk_nbt(&mut self, chunk_x: i32, chunk_z: i32) -> Option<Tag> {
        match self.get_chunk_data(chunk_x, chunk_z) {
            Some(mut c) => Some(Tag::parse(&mut c)),
            _ => None
        }
    }

    pub fn get_chunk(&mut self, chunk_x: i32, chunk_z: i32) -> Option<Chunk> {
        if let Some(tag) = self.get_chunk_nbt(chunk_x, chunk_z) {
            Some(Chunk::new(tag))
        } else {
            None
        }
    }
}

pub struct RegionWriter {
    data: BytesMut,
    current_sector: usize,
    header: RegionHeader
}

impl RegionWriter {
    pub fn new() -> RegionWriter {
        RegionWriter {
            data: BytesMut::new(),  // The chunk data, not the entire region file!
            current_sector: 0,
            header: RegionHeader::new()
        }
    }

    pub fn inner(&self) -> &BytesMut {
        &self.data
    }
    pub fn inner_mut(&mut self) -> &mut BytesMut {
        &mut self.data
    }

    /// Set the raw data of a chunk, where `data` is 1 byte compression type + n bytes data
    pub fn set_chunk_raw(&mut self, chunk_x: i32, chunk_z: i32, data: Vec<u8>) {
        let offset = ChunkInfo::get_index(chunk_x, chunk_z);
        let mut buf = BytesMut::new();
        buf.put_u32((data.len() - 1) as u32);  // 1 byte is the compression type
        buf.put(&data[..]);

        let full_len = data.len() + 4;  // Size of entire sector
        let num_sectors = ((full_len as f32) / 4096f32).ceil() as usize;
        let pad = (num_sectors * 4096) - full_len;

        let sector_offset = self.current_sector;
        self.data.put(buf);
        self.data.put_bytes(0, pad);

        self.current_sector += num_sectors;

        self.header.chunks[offset] = ChunkInfo::new(
            (sector_offset + 2) as u32,  // header is 2 sectors
            num_sectors as u8
        );
    }

    pub fn set_chunk_timestamp(&mut self, chunk_x: i32, chunk_z: i32, timestamp: u32) {
        self.header.timestamps[ChunkInfo::get_index(chunk_x, chunk_z)] =  timestamp;
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut buf = BytesMut::new();
        for chunk in &self.header.chunks {
            buf.put_uint(chunk.offset as u64, 3);
            buf.put_u8(chunk.sectors);
        }
        for timestamp in &self.header.timestamps {
            buf.put_u32(*timestamp);
        }

        buf.extend_from_slice(&self.data[..]);

        buf.into()
    }
}
