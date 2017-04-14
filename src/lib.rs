use std::io;

extern crate bitstream_io;
use bitstream_io::{BitReader, BE};

#[derive(Debug)]
pub enum ReadMKVError {
    Io(io::Error),
    InvalidID,
    InvalidSize
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
        Ok(0) => {r.read(7).map_err(ReadMKVError::Io)}
        Ok(1) => {r.read(6 + 8).map_err(ReadMKVError::Io)}
        Ok(2) => {r.read(5 + (2 * 8)).map_err(ReadMKVError::Io)}
        Ok(3) => {r.read(4 + (3 * 8)).map_err(ReadMKVError::Io)}
        Ok(4) => {r.read(3 + (4 * 8)).map_err(ReadMKVError::Io)}
        Ok(5) => {r.read(2 + (5 * 8)).map_err(ReadMKVError::Io)}
        Ok(6) => {r.read(1 + (6 * 8)).map_err(ReadMKVError::Io)}
        Ok(7) => {r.read(7 * 8).map_err(ReadMKVError::Io)}
        Ok(_) => {Err(ReadMKVError::InvalidSize)}
        Err(err) => {Err(ReadMKVError::Io(err))}
    }
}
