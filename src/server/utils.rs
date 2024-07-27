use bytes::{Buf, BufMut, Bytes, BytesMut};
use uuid::Uuid;


// VarInt implementations based on https://github.com/valence-rs/valence/blob/main/crates/valence_protocol/src/var_int.rs
// (which is also based on https://github.com/as-com/varint-simd/blob/master/src/encode/mod.rs)
pub fn read_varint(buf: &mut Bytes) -> i32 {
    let mut val = 0;
    for i in 0..5 {
        let byte = buf.get_u8();
        val |= (i32::from(byte) & 0b01111111) << (i * 7);
        if byte & 0b10000000 == 0 {
            break;
        }
    }
    // FIXME: what if the varint is more than 5 bytes long?
    return val;
}

pub fn write_varint(buf: &mut BytesMut, value: i32) -> () {
    let x = value as u64;
    let stage1 = (x & 0x000000000000007f)
        | ((x & 0x0000000000003f80) << 1)
        | ((x & 0x00000000001fc000) << 2)
        | ((x & 0x000000000fe00000) << 3)
        | ((x & 0x00000000f0000000) << 4);

    let leading = stage1.leading_zeros();

    let unused_bytes = (leading - 1) >> 3;
    let bytes_needed = 8 - unused_bytes;

    let msbs = 0x8080808080808080;
    let msbmask = 0xffffffffffffffff >> (((8 - bytes_needed + 1) << 3) - 1);

    let merged = stage1 | (msbs & msbmask);
    let bytes = merged.to_le_bytes();

    buf.put(unsafe { bytes.get_unchecked(..bytes_needed as usize) });

    ()
}

pub fn write_varlong(buf: &mut BytesMut, val: i64) -> () {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::*;
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::*;

    // Break the number into 7-bit parts and spread them out into a vector
    let mut res = [0_u64; 2];
    {
        let x = val as u64;

        res[0] = unsafe { _pdep_u64(x, 0x7f7f7f7f7f7f7f7f) };
        res[1] = unsafe { _pdep_u64(x >> 56, 0x000000000000017f) }
    };
    let stage1: __m128i = unsafe { std::mem::transmute(res) };

    // Create a mask for where there exist values
    // This signed comparison works because all MSBs should be cleared at this point
    // Also handle the special case when num == 0
    let minimum =
        unsafe { _mm_set_epi8(0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xff_u8 as i8) };
    let exists = unsafe { _mm_or_si128(_mm_cmpgt_epi8(stage1, _mm_setzero_si128()), minimum) };
    let bits = unsafe { _mm_movemask_epi8(exists) };

    // Count the number of bytes used
    let bytes_needed = 32 - bits.leading_zeros() as u8; // lzcnt on supported CPUs

    // Fill that many bytes into a vector
    let ascend = unsafe { _mm_setr_epi8(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15) };
    let mask = unsafe { _mm_cmplt_epi8(ascend, _mm_set1_epi8(bytes_needed as i8)) };

    // Shift it down 1 byte so the last MSB is the only one set, and make sure only
    // the MSB is set
    let shift = unsafe { _mm_bsrli_si128(mask, 1) };
    let msbmask = unsafe { _mm_and_si128(shift, _mm_set1_epi8(128_u8 as i8)) };

    // Merge the MSB bits into the vector
    let merged = unsafe { _mm_or_si128(stage1, msbmask) };
    let bytes = unsafe { std::mem::transmute::<__m128i, [u8; 16]>(merged) };

    buf.put(unsafe { bytes.get_unchecked(..bytes_needed as usize) });

    ()
}

pub fn read_string(buf: &mut Bytes) -> String {
    let len = read_varint(buf) as usize;
    let text = buf.slice(0..len);
    buf.advance(len);
    // FIXME: handle invalid strings better
    String::from_utf8_lossy(&text[..]).to_string()
}

pub fn write_string(buf: &mut BytesMut, s: &str) -> () {
    write_varint(buf, s.len() as i32);
    buf.put(s.as_bytes());
}

pub fn read_uuid(buf: &mut Bytes) -> Uuid {
    let id = Uuid::from_slice(&buf.slice(0..16)[..]).unwrap();
    buf.advance(16);
    id
}

pub fn write_uuid(buf: &mut BytesMut, id: Uuid) {
    buf.put(&id.as_bytes()[..])
}
