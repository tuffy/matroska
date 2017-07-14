use std::io;
use std::string::FromUtf8Error;

use bitstream_io::{BitReader, BE};
use chrono::DateTime;
use chrono::offset::Utc;

//
//pub enum MKVElementType {
//    Signed(i64),
//    Unsigned(u64),
//    Float(f64),
//    String(String),
//    UTF8(String),
//    Date(i64), /*FIXME - turn this into date/time*/
//    Master(Vec<MKVElement>),
//    Binary(Vec<u8>),
//    Void(u64),
//    Unknown
//}
//
//impl fmt::Debug for MKVElementType {
//    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
//        match self {
//            &MKVElementType::Signed(ref x) => {write!(f, "Signed({:?})", x)}
//            &MKVElementType::Unsigned(ref x) => {write!(f, "Unsigned({:?})", x)}
//            &MKVElementType::Float(ref x) => {write!(f, "Float({:?})", x)}
//            &MKVElementType::String(ref x) => {write!(f, "String({:?})", x)}
//            &MKVElementType::UTF8(ref x) => {write!(f, "UTF8({:?})", x)}
//            &MKVElementType::Date(ref x) => {write!(f, "Date({:?})", x)}
//            &MKVElementType::Master(ref x) => {write!(f, "Master({:?})", x)}
//            &MKVElementType::Binary(ref x) => {
//                write!(f, "Binary({} bytes)", x.len())
//            }
//            &MKVElementType::Void(ref x) => {write!(f, "Void({} bytes)", x)}
//            &MKVElementType::Unknown => {write!(f, "Unknown")}
//        }
//    }
//}
//
//#[derive(Debug)]
//pub struct MKVElement {
//    pub id: u32,
//    pub size: u64, /*total size of element, including id/size*/
//    pub value: MKVElementType
//}
//
//impl MKVElement {
//    pub fn parse(r: &mut BitReader<BE>) -> Result<MKVElement,ReadMKVError> {
//        let (id, id_len) = read_element_id(r)?;
//        let (size, size_len) = read_element_size(r)?;
//
//        /*FIXME - generate different elements based on type*/
//        let value = match id {
//            0xFB | 0xFD | 0x75A2 | 0x537F => {
//                MKVElementType::Signed(read_int(r, size)?)
//            }
//
//            0x83 | 0x88 | 0x89 |
//            0x91 | 0x92 | 0x96 | 0x97 | 0x98 |
//            0x9A | 0x9B | 0x9C | 0x9D | 0x9F |
//            0xA7 | 0xAA | 0xAB |
//            0xB0 | 0xB2 | 0xB3 | 0xB9 | 0xBA |
//            0xC0 | 0xC6 | 0xC7 | 0xC9 | 0xCA |
//            0xCB | 0xCC | 0xCD | 0xCE | 0xCF |
//            0xD7 |
//            0xE5 | 0xE6 | 0xE7 | 0xEA | 0xEB | 0xED | 0xEE |
//            0xF0 | 0xF1 | 0xF7 | 0xFA |
//
//            0x4254 | 0x4285 | 0x4286 | 0x4287 | 0x42F2 | 0x42F3 |
//            0x42F7 | 0x4484 | 0x4598 | 0x45BC | 0x45BD | 0x45DB |
//            0x45DD | 0x4661 | 0x4662 | 0x46AE | 0x47E1 | 0x47E5 |
//            0x47E6 |
//            0x5031 | 0x5032 | 0x5033 | 0x535F | 0x5378 | 0x53AC |
//            0x53B8 | 0x53B9 | 0x53C0 | 0x54AA | 0x54B0 | 0x54B2 |
//            0x54B3 | 0x54BA | 0x54BB | 0x54CC | 0x54DD | 0x55AA |
//            0x55B1 | 0x55B2 | 0x55B3 | 0x55B4 | 0x55B5 | 0x55B6 |
//            0x55B7 | 0x55B8 | 0x55B9 | 0x55BA | 0x55BB | 0x55BC |
//            0x55BD | 0x55EE | 0x56AA | 0x56BB | 0x58D7 |
//            0x6264 | 0x63C3 | 0x63C4 | 0x63C5 | 0x63C6 | 0x63C9 |
//            0x66BF | 0x66FC | 0x68CA | 0x6922 | 0x6955 | 0x69BF |
//            0x69FC | 0x6DE7 | 0x6DF8 | 0x6EBC | 0x6FAB |
//            0x73C4 | 0x73C5 | 0x7446 | 0x7E8A | 0x7E9A |
//
//            0x234E7A | 0x23E383 | 0x2AD7B1 => {
//                MKVElementType::Unsigned(read_uint(r, size)?)
//            }
//
//            0xB5 |
//            0x4489 | 0x55D1 | 0x55D2 | 0x55D3 | 0x55D4 | 0x55D5 |
//            0x55D6 | 0x55D7 | 0x55D8 | 0x55D9 | 0x55DA | 0x78B5 |
//            0x23314F | 0x2383E3 | 0x2FB523 => {
//                MKVElementType::Float(read_float(r, size)?)
//            }
//
//            0x86 |
//            0x4282 | 0x437C | 0x437E | 0x447A | 0x4660 | 0x63CA |
//            0x22B59C | 0x26B240 | 0x3B4040 => {
//                /*FIXME - restrict this to ASCII encoding*/
//                MKVElementType::String(read_utf8(r, size)?)
//            }
//
//            0x85 |
//            0x4487 | 0x45A3 | 0x466E | 0x467E | 0x4D80 | 0x536E |
//            0x5654 | 0x5741 | 0x7384 | 0x7BA9 |
//            0x258688 | 0x3A9697 | 0x3C83AB | 0x3E83BB => {
//                MKVElementType::UTF8(read_utf8(r, size)?)
//            }
//
//            0x4461 => {MKVElementType::Date(read_date(r, size)?)}
//
//            0x80 | 0x8E | 0x8F | 0xA0 | 0xA6 | 0xAE | 0xB6 | 0xB7 | 0xBB |
//            0xC8 | 0xDB | 0xE0 | 0xE1 | 0xE2 | 0xE3 | 0xE4 | 0xE8 | 0xE9 |
//
//            0x45B9 | 0x4DBB | 0x5034 | 0x5035 | 0x55B0 | 0x55D0 | 0x5854 |
//            0x61A7 | 0x6240 | 0x63C0 | 0x6624 | 0x67C8 | 0x6911 | 0x6924 |
//            0x6944 | 0x6D80 | 0x7373 | 0x75A1 | 0x7E5B | 0x7E7B |
//            0x1043A770 | 0x114D9B74 | 0x1254C367 | 0x1549A966 | 0x1654AE6B |
//            0x18538067 | 0x1941A469 | 0x1A45DFA3 | 0x1B538667 | 0x1C53BB6B |
//            0x1F43B675 => {
//                let mut remaining_size = size;
//                let mut sub_elements = Vec::new();
//                while remaining_size > 0 {
//                    let sub_element = MKVElement::parse(r)?;
//                    if sub_element.size <= size {
//                        remaining_size -= sub_element.size;
//                        sub_elements.push(sub_element);
//                    } else {
//                        return Err(ReadMKVError::SubElementTooLarge);
//                    }
//                }
//                MKVElementType::Master(sub_elements)
//            }
//
//            0xA1 | 0xA2 | 0xA3 | 0xA4 | 0xA5 | 0xAF | 0xBF | 0xC1 | 0xC4 |
//            0x4255 | 0x4444 | 0x4485 | 0x450D | 0x465C | 0x4675 | 0x47E2 |
//            0x47E3 | 0x47E4 | 0x53AB | 0x63A2 | 0x6532 | 0x66A5 | 0x6933 |
//            0x69A5 | 0x6E67 | 0x73A4 | 0x7D7B | 0x7EA5 | 0x7EB5 |
//            0x2EB524 | 0x3CB923 | 0x3EB923 => {
//                MKVElementType::Binary(read_bin(r, size)?)
//            }
//
//            0xEC => {
//                let _ = read_bin(r, size)?;
//                MKVElementType::Void(size)
//            }
//
//            _ => {
//                let _ = read_bin(r, size)?;
//                MKVElementType::Unknown
//            }
//        };
//
//        Ok(MKVElement{id: id, size: id_len + size_len + size, value: value})
//    }
//}
//

#[derive(Debug)]
pub enum MKVError {
    Io(io::Error),
    UTF8(FromUtf8Error),
    InvalidID,
    InvalidSize,
    InvalidUint,
    InvalidFloat,
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

