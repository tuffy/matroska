use std::io;
use std::fs::File;

extern crate bitstream_io;
extern crate chrono;

mod ebml;
mod ids;

use chrono::{DateTime, Duration};
use chrono::offset::Utc;

#[derive(Debug)]
pub struct MKV {
    pub info: Info,
    pub tracks: Vec<Track>,
    pub attachments: Vec<Attachment>,
    pub chapters: Vec<ChapterEdition>
}

#[derive(Debug)]
pub enum ReadMKVError {
    Io(io::Error)
}

impl MKV {
    pub fn new() -> MKV {
        MKV{info: Info::new(),
            tracks: Vec::new(),
            attachments: Vec::new(),
            chapters: Vec::new()}
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
            let (id, size, _) = ebml::read_element_id_size(&mut file).unwrap();
            id_0 = id;
            size_0 = size;
        }

        // pull out useful pieces from Segment
        while size_0 > 0 {
            let (id_1, size_1, len) =
                ebml::read_element_id_size(&mut file).unwrap();
            /*FIXME - implement extraction*/
            match id_1 {
                ids::INFO => {
                    mkv.info = Info::parse(&mut file, size_1)?;
                }
                ids::TRACKS => {
                    mkv.tracks = Track::parse(&mut file, size_1)?;
                }
                ids::ATTACHMENTS => {
                    mkv.attachments = Attachment::parse(&mut file, size_1)?;
                }
                ids::CHAPTERS => {
                    mkv.chapters = ChapterEdition::parse(&mut file, size_1)?;
                }
                _ => {
                    //println!("level1 : {:X} {}", id_1, size_1);
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
    pub duration: Option<Duration>,
    pub date_utc: Option<DateTime<Utc>>,
    pub muxing_app: String,
    pub writing_app: String
}

impl Info {
    fn new() -> Info {
        Info{title: None,
             duration: None,
             date_utc: None,
             muxing_app: String::new(),
             writing_app: String::new()}
    }

    fn parse(r: &mut io::Read, mut size: u64) -> Result<Info,ReadMKVError> {
        let mut info = Info::new();
        let mut timecode_scale = None;
        let mut duration = None;

        while size > 0 {
            /*FIXME - improve error handling*/
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
                    info.muxing_app = ebml::read_utf8(r, s).unwrap();
                }
                ids::WRITINGAPP => {
                    info.writing_app = ebml::read_utf8(r, s).unwrap();
                }
                _ => {ebml::skip(r, s).unwrap();}
            }
            size -= len;
            size -= s;
        }

        if let Some(d) = duration {
            if let Some(t) = timecode_scale {
                info.duration = Some(
                    Duration::nanoseconds((d * t as f64) as i64))
            }
        }
        Ok(info)
    }
}

#[derive(Debug)]
pub struct Video {
    pub pixel_width: u64,
    pub pixel_height: u64,
    pub display_width: Option<u64>,
    pub display_height: Option<u64>
}

impl Video {
    fn new() -> Video {
        Video{pixel_width: 0,
              pixel_height: 0,
              display_width: None,
              display_height: None}
    }

    fn parse(r: &mut io::Read, mut size: u64) -> Result<Video,ReadMKVError> {
        let mut video = Video::new();

        while size > 0 {
            let (i, s, len) = ebml::read_element_id_size(r).unwrap();

            match i {
                ids::PIXELWIDTH => {
                    video.pixel_width = ebml::read_uint(r, s).unwrap();
                }
                ids::PIXELHEIGHT => {
                    video.pixel_height = ebml::read_uint(r, s).unwrap();
                }
                ids::DISPLAYWIDTH => {
                    video.display_width = Some(ebml::read_uint(r, s).unwrap());
                }
                ids::DISPLAYHEIGHT => {
                    video.display_height = Some(ebml::read_uint(r, s).unwrap());
                }
                _ => {ebml::skip(r, s).unwrap();}
            }

            size -= len;
            size -= s;
        }

        Ok(video)
    }
}

#[derive(Debug)]
pub struct Audio {
    pub sample_rate: f64,
    pub channels: u64,
    pub bit_depth: Option<u64>
}

impl Audio {
    fn new() -> Audio {
        Audio{sample_rate: 0.0,
              channels: 0,
              bit_depth: None}
    }

    fn parse(r: &mut io::Read, mut size: u64) -> Result<Audio,ReadMKVError> {
        let mut audio = Audio::new();

        while size > 0 {
            let (i, s, len) = ebml::read_element_id_size(r).unwrap();

            match i {
                ids::SAMPLINGFREQUENCY => {
                    audio.sample_rate = ebml::read_float(r, s).unwrap();
                }
                ids::CHANNELS => {
                    audio.channels = ebml::read_uint(r, s).unwrap();
                }
                ids::BITDEPTH => {
                    audio.bit_depth = Some(ebml::read_uint(r, s).unwrap());
                }
                _ => {ebml::skip(r, s).unwrap();}
            }

            size -= len;
            size -= s;
        }

        Ok(audio)
    }
}

#[derive(Debug)]
pub enum Settings {
    None,
    Video(Video),
    Audio(Audio)
}

#[derive(Debug)]
pub struct Track {
    pub number: u64,
    pub uid: u64,
    pub tracktype: u64, /*FIXME - make enum?*/
    pub enabled: bool,
    pub default: bool,
    pub forced: bool,
    pub interlaced: bool,
    pub defaultduration: Option<Duration>,
    pub offset: Option<i64>,
    pub name: Option<String>,
    pub language: Option<String>,
    pub codec_id: String,
    pub codec_name: Option<String>,
    pub settings: Settings
}

impl Track {
    fn new() -> Track {
        Track{number: 0,
              uid: 0,
              tracktype: 0,
              enabled: true,
              default: true,
              forced: false,
              interlaced: true,
              defaultduration: None,
              offset: None,
              name: None,
              language: None,
              codec_id: String::new(),
              codec_name: None,
              settings: Settings::None}
    }

    fn parse(r: &mut io::Read, mut size: u64) ->
        Result<Vec<Track>,ReadMKVError> {
        let mut tracks = Vec::new();

        while size > 0 {
            let (i, s, len) = ebml::read_element_id_size(r).unwrap();
            if i == ids::TRACKENTRY {
                tracks.push(Track::parse_entry(r, s)?);
            } else {
                ebml::skip(r, s).unwrap();
            }

            size -= len;
            size -= s;
        }
        Ok(tracks)
    }

    fn parse_entry(r: &mut io::Read, mut size: u64) ->
        Result<Track,ReadMKVError> {
        let mut track = Track::new();
        while size > 0 {
            let (i, s, len) = ebml::read_element_id_size(r).unwrap();

            match i {
                ids::TRACKNUMBER => {
                    track.number = ebml::read_uint(r, s).unwrap();
                }
                ids::TRACKUID => {
                    track.uid = ebml::read_uint(r, s).unwrap();
                }
                ids::TRACKTYPE => {
                    track.tracktype = ebml::read_uint(r, s).unwrap();
                }
                ids::FLAGENABLED => {
                    track.enabled = ebml::read_uint(r, s).unwrap() != 0;
                }
                ids::FLAGDEFAULT => {
                    track.default = ebml::read_uint(r, s).unwrap() != 0;
                }
                ids::FLAGFORCED => {
                    track.forced = ebml::read_uint(r, s).unwrap() != 0;
                }
                ids::FLAGLACING => {
                    track.interlaced = ebml::read_uint(r, s).unwrap() != 0;
                }
                ids::DEFAULTDURATION => {
                    track.defaultduration =
                        Some(Duration::nanoseconds(
                            ebml::read_uint(r, s).unwrap() as i64));
                }
                ids::TRACKOFFSET => {
                    track.offset = Some(ebml::read_int(r, s).unwrap());
                }
                ids::NAME => {
                    track.name = Some(ebml::read_utf8(r, s).unwrap());
                }
                ids::LANGUAGE => {
                    track.language = Some(ebml::read_string(r, s).unwrap());
                }
                ids::CODEC_ID => {
                    track.codec_id = ebml::read_string(r, s).unwrap();
                }
                ids::CODEC_NAME => {
                    track.codec_name = Some(ebml::read_utf8(r, s).unwrap());
                }
                ids::VIDEO => {
                    track.settings = Settings::Video(Video::parse(r, s)?);
                }
                ids::AUDIO => {
                    track.settings = Settings::Audio(Audio::parse(r, s)?);
                }
                _ => {
                    ebml::skip(r, s).unwrap();
                }
            }

            size -= len;
            size -= s;
        }
        Ok(track)
    }
}

#[derive(Debug)]
pub struct Attachment {
    pub description: Option<String>,
    pub name: String,
    pub mime_type: String,
    pub data: Vec<u8>
}

impl Attachment {
    fn new() -> Attachment {
        Attachment{description: None,
                   name: String::new(),
                   mime_type: String::new(),
                   data: Vec::new()}
    }

    fn parse(r: &mut io::Read, mut size: u64) ->
        Result<Vec<Attachment>,ReadMKVError> {
        let mut attachments = Vec::new();

        while size > 0 {
            let (i, s, len) = ebml::read_element_id_size(r).unwrap();

            if i == ids::ATTACHEDFILE {
                attachments.push(Attachment::parse_entry(r, s)?);
            } else {
                let _ = ebml::skip(r, s);
            }

            size -= len;
            size -= s;
        }

        Ok(attachments)
    }

    fn parse_entry(r: &mut io::Read, mut size: u64) ->
        Result<Attachment,ReadMKVError> {
        let mut attachment = Attachment::new();

        while size > 0 {
            let (i, s, len) = ebml::read_element_id_size(r).unwrap();

            match i {
                ids::FILEDESCRIPTION => {
                    attachment.description =
                        Some(ebml::read_utf8(r, s).unwrap());
                }
                ids::FILENAME => {
                    attachment.name = ebml::read_utf8(r, s).unwrap();
                }
                ids::FILEMIMETYPE => {
                    attachment.mime_type = ebml::read_string(r, s).unwrap();
                }
                ids::FILEDATA => {
                    attachment.data = ebml::read_bin(r, s).unwrap();
                }
                _ => {
                    let _ = ebml::skip(r, s).unwrap();
                }
            }

            size -= len;
            size -= s;
        }

        Ok(attachment)
    }
}

#[derive(Debug)]
pub struct ChapterEdition {
    pub hidden: bool,
    pub default: bool,
    pub ordered: bool,
    pub chapters: Vec<Chapter>
}

impl ChapterEdition {
    fn new() -> ChapterEdition {
        ChapterEdition{hidden: false,
                       default: false,
                       ordered: false,
                       chapters: Vec::new()}
    }

    fn parse(r: &mut io::Read, mut size: u64) ->
        Result<Vec<ChapterEdition>,ReadMKVError> {
        let mut chaptereditions = Vec::new();

        while size > 0 {
            let (i, s, len) = ebml::read_element_id_size(r).unwrap();

            if i == ids::EDITIONENTRY {
                chaptereditions.push(ChapterEdition::parse_entry(r, s)?);
            } else {
                ebml::skip(r, s).unwrap();
            }

            size -= s;
            size -= len;
        }

        Ok(chaptereditions)
    }

    fn parse_entry(r: &mut io::Read, mut size: u64) ->
        Result<ChapterEdition,ReadMKVError> {

        let mut chapteredition = ChapterEdition::new();

        while size > 0 {
            let (i, s, len) = ebml::read_element_id_size(r).unwrap();

            match i {
                ids::EDITIONFLAGHIDDEN => {
                    chapteredition.hidden =
                        ebml::read_uint(r, s).unwrap() != 0;
                }
                ids::EDITIONFLAGDEFAULT => {
                    chapteredition.default =
                        ebml::read_uint(r, s).unwrap() != 0;
                }
                ids::EDITIONFLAGORDERED => {
                    chapteredition.ordered =
                        ebml::read_uint(r, s).unwrap() != 0;
                }
                ids::CHAPTERATOM => {
                    chapteredition.chapters.push(Chapter::parse(r, s)?)
                }
                _ => {
                    ebml::skip(r, s).unwrap();
                }
            }

            size -= s;
            size -= len;
        }

        Ok(chapteredition)
    }
}

#[derive(Debug)]
pub struct Chapter {
    pub time_start: Duration,
    pub time_end: Option<Duration>,
    pub hidden: bool,
    pub enabled: bool,
    pub display: Vec<ChapterDisplay>
}

impl Chapter {
    fn new() -> Chapter {
        Chapter{time_start: Duration::nanoseconds(0),
                time_end: None,
                hidden: false,
                enabled: false,
                display: Vec::new()}
    }

    fn parse(r: &mut io::Read, mut size: u64) -> Result<Chapter,ReadMKVError> {
        let mut chapter = Chapter::new();

        while size > 0 {
            let (i, s, len) = ebml::read_element_id_size(r).unwrap();

            match i {
                ids::CHAPTERTIMESTART => {
                    chapter.time_start =
                        Duration::nanoseconds(
                            ebml::read_uint(r, s).unwrap() as i64);
                }
                ids::CHAPTERTIMEEND => {
                    chapter.time_end =
                        Some(Duration::nanoseconds(
                            ebml::read_uint(r, s).unwrap() as i64));
                }
                ids::CHAPTERFLAGHIDDEN => {
                    chapter.hidden = ebml::read_uint(r, s).unwrap() != 0;
                }
                ids::CHAPTERFLAGENABLED => {
                    chapter.enabled = ebml::read_uint(r, s).unwrap() != 0;
                }
                ids::CHAPTERDISPLAY => {
                    chapter.display.push(ChapterDisplay::parse(r, s)?);
                }
                _ => {
                    ebml::skip(r, s).unwrap();
                }
            }

            size -= s;
            size -= len;
        }

        Ok(chapter)
    }
}

#[derive(Debug)]
pub struct ChapterDisplay {
    pub string: String,
    pub language: String
}

impl ChapterDisplay {
    fn new() -> ChapterDisplay {
        ChapterDisplay{string: String::new(), language: String::new()}
    }

    fn parse(r: &mut io::Read, mut size: u64) ->
        Result<ChapterDisplay,ReadMKVError> {
        let mut display = ChapterDisplay::new();

        while size > 0 {
            let (i, s, len) = ebml::read_element_id_size(r).unwrap();

            match i {
                ids::CHAPSTRING => {
                    display.string = ebml::read_utf8(r, s).unwrap();
                }
                ids::CHAPLANGUAGE => {
                    display.language = ebml::read_string(r, s).unwrap();
                }
                _ => {
                    ebml::skip(r, s).unwrap();
                }
            }

            size -= s;
            size -= len;
        }

        Ok(display)
    }
}
