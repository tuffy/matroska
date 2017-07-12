use std::io;

extern crate bitstream_io;
use bitstream_io::{BitReader, BE};

#[derive(Debug)]
pub enum ReadMKVError {
    Io(io::Error),
    InvalidID,
    InvalidSize,
    InvalidUint,
    InvalidFloat,
    InvalidDate,
    UTF8(std::string::FromUtf8Error)
}

pub fn read_element_id(r: &mut io::Read) -> Result<u32,ReadMKVError> {
    let mut r = BitReader::<BE>::new(r);

    match r.read_unary1() {
        Ok(0) => {
            r.read::<u32>(7)
             .map_err(ReadMKVError::Io)
             .map(|u| 0b10000000 | u)
        }
        Ok(1) => {
            r.read::<u32>(6 + 8)
             .map_err(ReadMKVError::Io)
             .map(|u| (0b01000000 << 8) | u)
        }
        Ok(2) => {
            r.read::<u32>(5 + 16)
             .map_err(ReadMKVError::Io)
             .map(|u| (0b00100000 << 16) | u)
        }
        Ok(3) => {
            r.read::<u32>(4 + 24)
             .map_err(ReadMKVError::Io)
             .map(|u| (0b00010000 << 24) | u)
        }
        Ok(_) => {Err(ReadMKVError::InvalidID)}
        Err(err) => {Err(ReadMKVError::Io(err))}
    }
}

pub fn read_element_size(r: &mut io::Read) -> Result<u64,ReadMKVError> {
    let mut r = BitReader::<BE>::new(r);

    match r.read_unary1() {
        Ok(0) => {r.read(7 + (0 * 8)).map_err(ReadMKVError::Io)}
        Ok(1) => {r.read(6 + (1 * 8)).map_err(ReadMKVError::Io)}
        Ok(2) => {r.read(5 + (2 * 8)).map_err(ReadMKVError::Io)}
        Ok(3) => {r.read(4 + (3 * 8)).map_err(ReadMKVError::Io)}
        Ok(4) => {r.read(3 + (4 * 8)).map_err(ReadMKVError::Io)}
        Ok(5) => {r.read(2 + (5 * 8)).map_err(ReadMKVError::Io)}
        Ok(6) => {r.read(1 + (6 * 8)).map_err(ReadMKVError::Io)}
        Ok(7) => {r.read(0 + (7 * 8)).map_err(ReadMKVError::Io)}
        Ok(_) => {Err(ReadMKVError::InvalidSize)}
        Err(err) => {Err(ReadMKVError::Io(err))}
    }
}

pub fn read_int(r: &mut io::Read, size: u64) -> Result<i64,ReadMKVError> {
    match size {
        0 => {Ok(0)}
        s @ 1...8 => {
            let mut r = BitReader::<BE>::new(r);
            r.read_signed(s as u32 * 8).map_err(ReadMKVError::Io)
        }
        _ => {Err(ReadMKVError::InvalidUint)}
    }
}

pub fn read_uint(r: &mut io::Read, size: u64) -> Result<u64,ReadMKVError> {
    match size {
        0 => {Ok(0)}
        s @ 1...8 => {
            let mut r = BitReader::<BE>::new(r);
            r.read(s as u32 * 8).map_err(ReadMKVError::Io)
        }
        _ => {Err(ReadMKVError::InvalidUint)}
    }
}

pub fn read_float(r: &mut io::Read, size: u64) -> Result<f64,ReadMKVError> {
    match size {
        0 => {Ok(0.0)}
        4 => {read_bin(r, 4).map(|_| 0.0) /*FIXME*/}
        8 => {read_bin(r, 8).map(|_| 0.0) /*FIXME*/}
        10 => {read_bin(r, 10).map(|_| 0.0) /*FIXME*/}
        _ => {Err(ReadMKVError::InvalidFloat)}
    }
}

pub fn read_utf8(r: &mut io::Read, size: u64) -> Result<String,ReadMKVError> {
    read_bin(r, size).and_then(
        |bytes| String::from_utf8(bytes).map_err(ReadMKVError::UTF8))
}

/*FIXME - have this return proper date value*/
pub fn read_date(r: &mut io::Read, size: u64) -> Result<(),ReadMKVError> {
    if size == 8 {
        read_int(r, size).map(|_| ())
    } else {
        Err(ReadMKVError::InvalidDate)
    }
}

pub fn read_bin(r: &mut io::Read, size: u64) -> Result<Vec<u8>,ReadMKVError> {
    /*FIXME - need to read this in chunks*/
    let mut buf = Vec::with_capacity(size as usize);
    buf.resize(size as usize, 0);
    r.read_exact(&mut buf).map(|()| buf).map_err(ReadMKVError::Io)
}

pub struct LimitedReader<'a> {
    reader: &'a mut io::Read,
    length: usize
}

impl<'a> LimitedReader<'a> {
    #[inline]
    pub fn new(reader: &mut io::Read, length: u64) -> LimitedReader {
        LimitedReader{reader: reader, length: length as usize}
    }

    #[inline]
    pub fn empty(&self) -> bool {
        self.length == 0
    }
}

impl<'a> io::Read for LimitedReader<'a> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize,io::Error> {
        use std::cmp::min;

        let to_read = min(self.length, buf.len());
        self.length -= to_read;
        self.reader.read(&mut buf[0..to_read])
    }
}
