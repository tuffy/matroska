// Copyright 2017-2020 Brian Langenberger
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::string::FromUtf8Error;
use std::{error, fmt, io};

use bitstream_io::BitRead;
use chrono::offset::Utc;
use chrono::DateTime;
use phf::{phf_set, Set};

pub type Result<T> = std::result::Result<T, MatroskaError>;

type BitReader<'a> = bitstream_io::BitReader<&'a mut dyn io::Read, bitstream_io::BigEndian>;

/// An EBML tree element
#[derive(Debug)]
pub struct Element {
    pub id: u32,
    pub size: u64, /*total size of element, including header*/
    pub val: ElementType,
}

static IDS_MASTER: Set<u32> = phf_set! {
    0x80u32, 0x8Eu32, 0x8Fu32, 0xA0u32, 0xA6u32, 0xAEu32, 0xB6u32,
    0xB7u32, 0xBBu32, 0xC8u32, 0xDBu32, 0xE0u32, 0xE1u32, 0xE2u32,
    0xE3u32, 0xE4u32, 0xE8u32, 0xE9u32, 0x45B9u32, 0x4DBBu32,
    0x5034u32, 0x5035u32, 0x55B0u32, 0x55D0u32, 0x5854u32, 0x61A7u32,
    0x6240u32, 0x63C0u32, 0x6624u32, 0x67C8u32, 0x6911u32, 0x6924u32,
    0x6944u32, 0x6D80u32, 0x7373u32, 0x75A1u32, 0x7E5Bu32, 0x7E7Bu32,
    0x1043_A770u32, 0x114D_9B74u32, 0x1254_C367u32, 0x1549_A966u32,
    0x1654_AE6Bu32, 0x1853_8067u32, 0x1941_A469u32, 0x1A45_DFA3u32,
    0x1B53_8667u32, 0x1C53_BB6Bu32, 0x1F43_B675u32
};

static IDS_INT: Set<u32> = phf_set! {
    0xFBu32, 0xFDu32, 0x537Fu32, 0x75A2u32
};

static IDS_UINT: Set<u32> = phf_set! {
    0x83u32, 0x88u32, 0x89u32, 0x91u32, 0x92u32, 0x96u32,
    0x97u32, 0x98u32, 0x9Au32, 0x9Bu32, 0x9Cu32, 0x9Du32,
    0x9Fu32, 0xA7u32, 0xAAu32, 0xABu32, 0xB0u32, 0xB2u32,
    0xB3u32, 0xB9u32, 0xBAu32, 0xC0u32, 0xC6u32, 0xC7u32,
    0xC9u32, 0xCAu32, 0xCBu32, 0xCCu32, 0xCDu32, 0xCEu32,
    0xCFu32, 0xD7u32, 0xE5u32, 0xE6u32, 0xE7u32, 0xEAu32,
    0xEBu32, 0xEDu32, 0xEEu32, 0xF0u32, 0xF1u32, 0xF7u32,
    0xFAu32, 0x4254u32, 0x4285u32, 0x4286u32, 0x4287u32,
    0x42F2u32, 0x42F3u32, 0x42F7u32, 0x4484u32, 0x4598u32,
    0x45BCu32, 0x45BDu32, 0x45DBu32, 0x45DDu32, 0x4661u32,
    0x4662u32, 0x46AEu32, 0x47E1u32, 0x47E5u32, 0x47E6u32,
    0x5031u32, 0x5032u32, 0x5033u32, 0x535Fu32, 0x5378u32,
    0x53ACu32, 0x53B8u32, 0x53B9u32, 0x53C0u32, 0x54AAu32,
    0x54B0u32, 0x54B2u32, 0x54B3u32, 0x54BAu32, 0x54BBu32,
    0x54CCu32, 0x54DDu32, 0x55AAu32, 0x55B1u32, 0x55B2u32,
    0x55B3u32, 0x55B4u32, 0x55B5u32, 0x55B6u32, 0x55B7u32,
    0x55B8u32, 0x55B9u32, 0x55BAu32, 0x55BBu32, 0x55BCu32,
    0x55BDu32, 0x55EEu32, 0x56AAu32, 0x56BBu32, 0x58D7u32,
    0x6264u32, 0x63C3u32, 0x63C4u32, 0x63C5u32, 0x63C6u32,
    0x63C9u32, 0x66BFu32, 0x66FCu32, 0x68CAu32, 0x6922u32,
    0x6955u32, 0x69BFu32, 0x69FCu32, 0x6DE7u32, 0x6DF8u32,
    0x6EBCu32, 0x6FABu32, 0x73C4u32, 0x73C5u32, 0x7446u32,
    0x7E8Au32, 0x7E9Au32, 0x23_4E7Au32, 0x23_E383u32, 0x2A_D7B1u32
};

static IDS_STRING: Set<u32> = phf_set! {
    0x86u32, 0x4282u32, 0x437Cu32, 0x437Du32, 0x437Eu32, 0x447Au32, 0x447Bu32,
    0x4660u32, 0x63CAu32, 0x22_B59Cu32, 0x22_B59Du32, 0x26_B240u32,
    0x3B_4040u32
};

static IDS_UTF8: Set<u32> = phf_set! {
    0x85u32, 0x4487u32, 0x45A3u32, 0x466Eu32, 0x467Eu32,
    0x4D80u32, 0x536Eu32, 0x5654u32, 0x5741u32, 0x7384u32,
    0x7BA9u32, 0x25_8688u32, 0x3A_9697u32, 0x3C_83ABu32, 0x3E_83BBu32
};

static IDS_BINARY: Set<u32> = phf_set! {
    0xA1u32, 0xA2u32, 0xA3u32, 0xA4u32, 0xA5u32, 0xAFu32,
    0xBFu32, 0xC1u32, 0xC4u32, 0xECu32, 0x4255u32, 0x4444u32,
    0x4485u32, 0x450Du32, 0x465Cu32, 0x4675u32, 0x47E2u32,
    0x47E3u32, 0x47E4u32, 0x53ABu32, 0x63A2u32, 0x6532u32,
    0x66A5u32, 0x6933u32, 0x69A5u32, 0x6E67u32, 0x73A4u32,
    0x7D7Bu32, 0x7EA5u32, 0x7EB5u32, 0x2E_B524u32, 0x3C_B923u32,
    0x3E_B923u32
};

static IDS_FLOAT: Set<u32> = phf_set! {
    0xB5u32, 0x4489u32, 0x55D1u32, 0x55D2u32, 0x55D3u32,
    0x55D4u32, 0x55D5u32, 0x55D6u32, 0x55D7u32, 0x55D8u32,
    0x55D9u32, 0x55DAu32, 0x78B5u32, 0x23_314Fu32, 0x23_83E3u32,
    0x2F_B523u32
};

impl Element {
    pub fn parse(r: &mut dyn io::Read) -> Result<Element> {
        let (id, size, header_len) = read_element_id_size(r)?;
        let val = Element::parse_body(r, id, size)?;
        Ok(Element {
            id,
            size: header_len + size,
            val,
        })
    }

    pub fn parse_body(r: &mut dyn io::Read, id: u32, size: u64) -> Result<ElementType> {
        match id {
            id if IDS_MASTER.contains(&id) => {
                Element::parse_master(r, size).map(ElementType::Master)
            }
            id if IDS_INT.contains(&id) => read_int(r, size).map(ElementType::Int),
            id if IDS_UINT.contains(&id) => read_uint(r, size).map(ElementType::UInt),
            id if IDS_STRING.contains(&id) => read_string(r, size).map(ElementType::String),
            id if IDS_UTF8.contains(&id) => read_utf8(r, size).map(ElementType::UTF8),
            id if IDS_BINARY.contains(&id) => read_bin(r, size).map(ElementType::Binary),
            id if IDS_FLOAT.contains(&id) => read_float(r, size).map(ElementType::Float),
            0x4461 => read_date(r, size).map(ElementType::Date),
            _ => read_bin(r, size).map(ElementType::Binary),
        }
    }

    pub fn parse_master(r: &mut dyn io::Read, mut size: u64) -> Result<Vec<Element>> {
        let mut elements = Vec::new();
        while size > 0 {
            let e = Element::parse(r)?;
            assert!(e.size <= size);
            size -= e.size;
            elements.push(e);
        }
        Ok(elements)
    }
}

#[derive(Debug)]
pub enum ElementType {
    Master(Vec<Element>),
    Int(i64),
    UInt(u64),
    String(String),
    UTF8(String),
    Binary(Vec<u8>),
    Float(f64),
    Date(DateTime<Utc>),
}

/// A possible error when parsing a Matroska file
#[derive(Debug)]
pub enum MatroskaError {
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
    InvalidDate,
}

impl fmt::Display for MatroskaError {
    fn fmt(&self, f: &mut fmt::Formatter) -> std::result::Result<(), fmt::Error> {
        match self {
            MatroskaError::Io(error) => error.fmt(f),
            MatroskaError::UTF8(error) => error.fmt(f),
            MatroskaError::InvalidID => write!(f, "invalid element ID"),
            MatroskaError::InvalidSize => write!(f, "invalid element size"),
            MatroskaError::InvalidUint => write!(f, "invalid unsigned integer"),
            MatroskaError::InvalidFloat => write!(f, "invalid float"),
            MatroskaError::InvalidDate => write!(f, "invalid date"),
        }
    }
}

impl error::Error for MatroskaError {
    fn cause(&self) -> Option<&dyn error::Error> {
        match self {
            MatroskaError::Io(error) => Some(error),
            MatroskaError::UTF8(error) => Some(error),
            _ => None,
        }
    }
}

pub fn read_element_id_size(reader: &mut dyn io::Read) -> Result<(u32, u64, u64)> {
    let mut r = BitReader::new(reader);
    let (id, id_len) = read_element_id(&mut r)?;
    let (size, size_len) = read_element_size(&mut r)?;
    Ok((id, size, id_len + size_len))
}

fn read_element_id<R: BitRead>(r: &mut R) -> Result<(u32, u64)> {
    match r.read_unary1() {
        Ok(0) => r
            .read::<u32>(7)
            .map_err(MatroskaError::Io)
            .map(|u| (0b1000_0000 | u, 1)),
        Ok(1) => r
            .read::<u32>(6 + 8)
            .map_err(MatroskaError::Io)
            .map(|u| ((0b0100_0000 << 8) | u, 2)),
        Ok(2) => r
            .read::<u32>(5 + 16)
            .map_err(MatroskaError::Io)
            .map(|u| ((0b0010_0000 << 16) | u, 3)),
        Ok(3) => r
            .read::<u32>(4 + 24)
            .map_err(MatroskaError::Io)
            .map(|u| ((0b0001_0000 << 24) | u, 4)),
        Ok(_) => Err(MatroskaError::InvalidID),
        Err(err) => Err(MatroskaError::Io(err)),
    }
}

fn read_element_size<R: BitRead>(r: &mut R) -> Result<(u64, u64)> {
    match r.read_unary1() {
        Ok(0) => r.read(7).map(|s| (s, 1)).map_err(MatroskaError::Io),
        Ok(1) => r.read(6 + 8).map(|s| (s, 2)).map_err(MatroskaError::Io),
        Ok(2) => r
            .read(5 + (2 * 8))
            .map(|s| (s, 3))
            .map_err(MatroskaError::Io),
        Ok(3) => r
            .read(4 + (3 * 8))
            .map(|s| (s, 4))
            .map_err(MatroskaError::Io),
        Ok(4) => r
            .read(3 + (4 * 8))
            .map(|s| (s, 5))
            .map_err(MatroskaError::Io),
        Ok(5) => r
            .read(2 + (5 * 8))
            .map(|s| (s, 6))
            .map_err(MatroskaError::Io),
        Ok(6) => r
            .read(1 + (6 * 8))
            .map(|s| (s, 7))
            .map_err(MatroskaError::Io),
        Ok(7) => r.read(7 * 8).map(|s| (s, 8)).map_err(MatroskaError::Io),
        Ok(_) => Err(MatroskaError::InvalidSize),
        Err(err) => Err(MatroskaError::Io(err)),
    }
}

pub fn read_int(r: &mut dyn io::Read, size: u64) -> Result<i64> {
    let mut r = BitReader::new(r);
    match size {
        0 => Ok(0),
        s @ 1..=8 => r.read_signed(s as u32 * 8).map_err(MatroskaError::Io),
        _ => Err(MatroskaError::InvalidUint),
    }
}

pub fn read_uint(r: &mut dyn io::Read, size: u64) -> Result<u64> {
    let mut r = BitReader::new(r);
    match size {
        0 => Ok(0),
        s @ 1..=8 => r.read(s as u32 * 8).map_err(MatroskaError::Io),
        _ => Err(MatroskaError::InvalidUint),
    }
}

pub fn read_float(r: &mut dyn io::Read, size: u64) -> Result<f64> {
    let mut r = BitReader::new(r);
    match size {
        4 => {
            let i: u32 = r.read(32).map_err(MatroskaError::Io)?;
            let f = f32::from_bits(i);
            Ok(f64::from(f))
        }
        8 => {
            let i: u64 = r.read(64).map_err(MatroskaError::Io)?;
            let f = f64::from_bits(i);
            Ok(f)
        }
        _ => Err(MatroskaError::InvalidFloat),
    }
}

pub fn read_string(r: &mut dyn io::Read, size: u64) -> Result<String> {
    /*FIXME - limit this to ASCII set*/
    read_bin(r, size).and_then(|bytes| String::from_utf8(bytes).map_err(MatroskaError::UTF8))
}

pub fn read_utf8(r: &mut dyn io::Read, size: u64) -> Result<String> {
    read_bin(r, size).and_then(|bytes| String::from_utf8(bytes).map_err(MatroskaError::UTF8))
}

pub fn read_date(r: &mut dyn io::Read, size: u64) -> Result<DateTime<Utc>> {
    if size == 8 {
        use chrono::Duration;
        use chrono::TimeZone;

        read_int(r, size).map(|d| Utc.ymd(2001, 1, 1).and_hms(0, 0, 0) + Duration::nanoseconds(d))
    } else {
        Err(MatroskaError::InvalidDate)
    }
}

pub fn read_bin(r: &mut dyn io::Read, size: u64) -> Result<Vec<u8>> {
    let mut buf = vec![0; size as usize];
    r.read_exact(&mut buf)
        .map(|()| buf)
        .map_err(MatroskaError::Io)
}
