use std::io;
use std::fs::File;

extern crate bitstream_io;
extern crate chrono;

mod ebml;
mod ids;

use chrono::DateTime;
use chrono::offset::Utc;

#[derive(Debug)]
pub struct MKV {
    pub info: Info,
    //pub tracks_video: Vec<Video>,
    //pub tracks_audio: Vec<Audio>,
    //pub tracks_subtitle: Vec<Subtitle>,
    //pub chapters: Vec<Chapter>,
    //pub tags: Vec<Tag>
}

#[derive(Debug)]
pub enum ReadMKVError {
    Io(io::Error)
}

impl MKV {
    pub fn new() -> MKV {
        MKV{info: Info::new()}
    }

    pub fn open(mut file: File) -> Result<MKV,ReadMKVError> {
        use std::io::Seek;
        use std::io::SeekFrom;

        let mut mkv = MKV::new();

        // look for first Segment in stream
        /*FIXME - clean this up*/
        let (mut id_0, mut size_0, _) =
            ebml::read_element_id_size(&mut file).unwrap();
        while id_0 != ids::SEGMENT {
            file.seek(SeekFrom::Current(size_0 as i64)).map(|_| ()).unwrap();
            let t = ebml::read_element_id_size(&mut file).unwrap();
            id_0 = t.0;
            size_0 = t.1;
        }

        // pull out useful pieces from Segment
        while size_0 > 0 {
            let (id_1, size_1, len) =
                ebml::read_element_id_size(&mut file).unwrap();
            println!("level1 : {:X} {}", id_1, size_1);
            /*FIXME - implement extraction*/
            match id_1 {
                ids::INFO => {
                    mkv.info = Info::parse(&mut file, size_1)?;
                }
                _ => {
                    file.seek(SeekFrom::Current(size_1 as i64))
                        .map(|_| ())
                        .unwrap();
                }
            }
            size_0 -= len;
            size_0 -= size_1;
        }

        Ok(mkv)
    }
}

#[derive(Debug)]
pub struct Info {
    pub title: Option<String>,
    pub duration: Option<f64>,
    pub date_utc: Option<DateTime<Utc>>,
    pub muxing_app: Option<String>,
    pub writing_app: Option<String>
}

impl Info {
    pub fn new() -> Info {
        Info{title: None,
             duration: None,
             date_utc: None,
             muxing_app: None,
             writing_app: None}
    }

    pub fn parse(r: &mut io::Read, mut size: u64) -> Result<Info,ReadMKVError> {
        let mut info = Info::new();
        let mut timecode_scale = None;
        let mut duration = None;

        while size > 0 {
            let (i, s, len) = ebml::read_element_id_size(r).unwrap();
            match i {
                ids::TITLE => {
                    info.title = Some(ebml::read_utf8(r, s).unwrap());
                }
                ids::TIMECODESCALE => {
                    timecode_scale = Some(ebml::read_uint(r, s).unwrap());
                }
                ids::DURATION => {
                    duration = Some(ebml::read_float(r, s).unwrap());
                }
                ids::DATEUTC => {
                    info.date_utc = Some(ebml::read_date(r, s).unwrap());
                }
                ids::MUXINGAPP => {
                    info.muxing_app = Some(ebml::read_utf8(r, s).unwrap());
                }
                ids::WRITINGAPP => {
                    info.writing_app = Some(ebml::read_utf8(r, s).unwrap());
                }
                _ => {ebml::read_bin(r, s).unwrap();}
            }
            size -= len;
            size -= s;
        }

        if let Some(d) = duration {
            if let Some(t) = timecode_scale {
                info.duration = Some((d * t as f64) / 1_000_000_000.0)
            }
        }
        Ok(info)
    }
}

//pub struct Video {
//    /*FIXME*/
//}
//
//pub struct Audio {
//    /*FIXME*/
//}
//
//pub struct Subtitle {
//    /*FIXME*/
//}
//
//pub struct Chapter {
//    /*FIXME*/
//}
//
//pub struct Tag {
//    /*FIXME*/
//}
