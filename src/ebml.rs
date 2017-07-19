use std::io;
use std::string::FromUtf8Error;

use bitstream_io::{BitReader, BE};
use chrono::DateTime;
use chrono::offset::Utc;

/// Some form of error when parsing MKV file
#[derive(Debug)]
pub enum MKVError {
    /// An I/O error
    Io(io::Error),
    /// An error decoding a UTF-8 string
    UTF8(FromUtf8Error),
    /// An invalid element ID
    InvalidID,
    /// An invalid element size
    InvalidSize,
    /// An invalid unsigned integer
    InvalidUint,
    /// An invalid floating point value
    InvalidFloat,
    /// An invalid date value
    InvalidDate
}

pub fn read_element_id_size(reader: &mut io::Read) ->
    Result<(u32,u64,u64),MKVError> {
    let mut r = BitReader::<BE>::new(reader);
    let (id, id_len) = read_element_id(&mut r)?;
    let (size, size_len) = read_element_size(&mut r)?;
    Ok((id, size, id_len + size_len))
}

fn read_element_id(r: &mut BitReader<BE>) -> Result<(u32,u64),MKVError> {
    match r.read_unary1() {
        Ok(0) => {
            r.read::<u32>(7)
             .map_err(MKVError::Io)
             .map(|u| (0b10000000 | u, 1))
        }
        Ok(1) => {
            r.read::<u32>(6 + 8)
             .map_err(MKVError::Io)
             .map(|u| ((0b01000000 << 8) | u, 2))
        }
        Ok(2) => {
            r.read::<u32>(5 + 16)
             .map_err(MKVError::Io)
             .map(|u| ((0b00100000 << 16) | u, 3))
        }
        Ok(3) => {
            r.read::<u32>(4 + 24)
             .map_err(MKVError::Io)
             .map(|u| ((0b00010000 << 24) | u, 4))
        }
        Ok(_) => {Err(MKVError::InvalidID)}
        Err(err) => {Err(MKVError::Io(err))}
    }
}

fn read_element_size(r: &mut BitReader<BE>) -> Result<(u64,u64),MKVError> {
    match r.read_unary1() {
        Ok(0) => {r.read(7 + (0 * 8)).map(|s| (s, 1)).map_err(MKVError::Io)}
        Ok(1) => {r.read(6 + (1 * 8)).map(|s| (s, 2)).map_err(MKVError::Io)}
        Ok(2) => {r.read(5 + (2 * 8)).map(|s| (s, 3)).map_err(MKVError::Io)}
        Ok(3) => {r.read(4 + (3 * 8)).map(|s| (s, 4)).map_err(MKVError::Io)}
        Ok(4) => {r.read(3 + (4 * 8)).map(|s| (s, 5)).map_err(MKVError::Io)}
        Ok(5) => {r.read(2 + (5 * 8)).map(|s| (s, 6)).map_err(MKVError::Io)}
        Ok(6) => {r.read(1 + (6 * 8)).map(|s| (s, 7)).map_err(MKVError::Io)}
        Ok(7) => {r.read(0 + (7 * 8)).map(|s| (s, 8)).map_err(MKVError::Io)}
        Ok(_) => {Err(MKVError::InvalidSize)}
        Err(err) => {Err(MKVError::Io(err))}
    }
}

pub fn read_int(r: &mut io::Read, size: u64) -> Result<i64,MKVError> {
    let mut r = BitReader::<BE>::new(r);
    match size {
        0 => {Ok(0)}
        s @ 1...8 => {r.read_signed(s as u32 * 8).map_err(MKVError::Io)}
        _ => {Err(MKVError::InvalidUint)}
    }
}

pub fn read_uint(r: &mut io::Read, size: u64) -> Result<u64,MKVError> {
    let mut r = BitReader::<BE>::new(r);
    match size {
        0 => {Ok(0)}
        s @ 1...8 => {r.read(s as u32 * 8).map_err(MKVError::Io)}
        _ => {Err(MKVError::InvalidUint)}
    }
}

pub fn read_float(r: &mut io::Read, size: u64) -> Result<f64,MKVError> {
    use std::mem;

    let mut r = BitReader::<BE>::new(r);
    match size {
        4 => {
            let i: u32 = r.read(32).map_err(MKVError::Io)?;
            let f: f32 = unsafe {mem::transmute(i)};
            Ok(f as f64)
        }
        8 => {
            let i: u64 = r.read(64).map_err(MKVError::Io)?;
            let f: f64 = unsafe {mem::transmute(i)};
            Ok(f)
        }
        _ => {Err(MKVError::InvalidFloat)}
    }
}

pub fn read_string(r: &mut io::Read, size: u64) -> Result<String,MKVError> {
    /*FIXME - limit this to ASCII set*/
    read_bin(r, size).and_then(
        |bytes| String::from_utf8(bytes).map_err(MKVError::UTF8))
}

pub fn read_utf8(r: &mut io::Read, size: u64) -> Result<String,MKVError> {
    read_bin(r, size).and_then(
        |bytes| String::from_utf8(bytes).map_err(MKVError::UTF8))
}

pub fn read_date(r: &mut io::Read, size: u64) -> Result<DateTime<Utc>,MKVError> {
    if size == 8 {
        use chrono::Duration;
        use chrono::TimeZone;

        read_int(r, size)
        .map(|d|
            Utc.ymd(2001, 1, 1)
               .and_hms(0, 0, 0) + Duration::nanoseconds(d))
    } else {
        Err(MKVError::InvalidDate)
    }
}

pub fn read_bin(r: &mut io::Read, size: u64) -> Result<Vec<u8>,MKVError> {
    /*FIXME - need to read this in chunks*/
    let mut buf = Vec::with_capacity(size as usize);
    buf.resize(size as usize, 0);
    r.read_exact(&mut buf).map(|()| buf).map_err(MKVError::Io)
}

pub fn skip(r: &mut io::Read, size: u64) -> Result<(),MKVError> {
    read_bin(r, size).map(|_| ())
}
