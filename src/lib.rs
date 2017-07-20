//! A library for MKV file metadata parsing functionality
//!
//! Implemented as a set of nested structs with public values
//! which one can use directly.
//!
//! ## Example
//! ```
//! use std::fs::File;
//! use mkv::MKV;
//! let f = File::open("filename.mkv").unwrap();
//! let mkv = MKV::open(f).unwrap();
//! println!("title : {:?}", mkv.info.title);
//! ```
//!
//! For additional information about the MKV format, see the
//! official [specification](https://matroska.org)

#![warn(missing_docs)]

use std::io;
use std::fs::File;
use std::collections::BTreeMap;

extern crate bitstream_io;
extern crate chrono;

mod ebml;
mod ids;

use chrono::{DateTime, Duration};
use chrono::offset::Utc;

pub use ebml::MKVError;

/// An MKV file
#[derive(Debug)]
pub struct MKV {
    /// The file's Info segment
    pub info: Info,
    /// The file's Tracks segment
    pub tracks: Vec<Track>,
    /// The file's Attachments segment
    pub attachments: Vec<Attachment>,
    /// The file's Chapters segment
    pub chapters: Vec<ChapterEdition>
}

impl MKV {
    fn new() -> MKV {
        MKV{info: Info::new(),
            tracks: Vec::new(),
            attachments: Vec::new(),
            chapters: Vec::new()}
    }

    /// Parses contents of open MKV file
    pub fn open(mut file: File) -> Result<MKV,MKVError> {
        use std::io::Seek;
        use std::io::SeekFrom;

        let mut mkv = MKV::new();

        let (mut id_0, mut size_0, _) = ebml::read_element_id_size(&mut file)?;
        while id_0 != ids::SEGMENT {
            file.seek(SeekFrom::Current(size_0 as i64))
                .map(|_| ())
                .map_err(MKVError::Io)?;
            let (id, size, _) = ebml::read_element_id_size(&mut file)?;
            id_0 = id;
            size_0 = size;
        }

        let segment_start = file.seek(SeekFrom::Current(0))
                                .map_err(MKVError::Io)?;

        while size_0 > 0 {
            let (id_1, size_1, len) = ebml::read_element_id_size(&mut file)?;
            match id_1 {
                ids::SEEKHEAD => {
                    let seektable = Seektable::parse(&mut file, size_1)?;
                    if let Some(pos) = seektable.get(ids::INFO) {
                        file.seek(SeekFrom::Start(pos + segment_start))
                            .map_err(MKVError::Io)?;
                        let (i, s, _) = ebml::read_element_id_size(&mut file)?;
                        assert_eq!(i, ids::INFO);
                        mkv.info = Info::parse(&mut file, s)?;
                    }
                    if let Some(pos) = seektable.get(ids::TRACKS) {
                        file.seek(SeekFrom::Start(pos + segment_start))
                            .map_err(MKVError::Io)?;
                        let (i, s, _) = ebml::read_element_id_size(&mut file)?;
                        assert_eq!(i, ids::TRACKS);
                        mkv.tracks = Track::parse(&mut file, s)?;
                    }
                    if let Some(pos) = seektable.get(ids::ATTACHMENTS) {
                        file.seek(SeekFrom::Start(pos + segment_start))
                            .map_err(MKVError::Io)?;
                        let (i, s, _) = ebml::read_element_id_size(&mut file)?;
                        assert_eq!(i, ids::ATTACHMENTS);
                        mkv.attachments = Attachment::parse(&mut file, s)?;
                    }
                    if let Some(pos) = seektable.get(ids::CHAPTERS) {
                        file.seek(SeekFrom::Start(pos + segment_start))
                            .map_err(MKVError::Io)?;
                        let (i, s, _) = ebml::read_element_id_size(&mut file)?;
                        assert_eq!(i, ids::CHAPTERS);
                        mkv.chapters = ChapterEdition::parse(&mut file, s)?;
                    }
                    return Ok(mkv)
                }
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
                    file.seek(SeekFrom::Current(size_1 as i64))
                        .map(|_| ())
                        .map_err(MKVError::Io)?;
                }
            }
            size_0 -= len;
            size_0 -= size_1;
        }

        Ok(mkv)
    }

    /// Returns all tracks with a type of "video"
    pub fn video_tracks(&self) -> Vec<&Track> {
        self.tracks.iter()
                   .filter(|t| t.tracktype == Tracktype::Video)
                   .collect()
    }

    /// Returns all tracks with a type of "audio"
    pub fn audio_tracks(&self) -> Vec<&Track> {
        self.tracks.iter()
                   .filter(|t| t.tracktype == Tracktype::Audio)
                   .collect()
    }

    /// Returns all tracks with a type of "subtitle"
    pub fn subtitle_tracks(&self) -> Vec<&Track> {
        self.tracks.iter()
                   .filter(|t| t.tracktype == Tracktype::Subtitle)
                   .collect()
    }
}

#[derive(Debug)]
struct Seektable {
    seek: BTreeMap<u32,u64>
}

impl Seektable {
    fn new() -> Seektable {
        Seektable{seek: BTreeMap::new()}
    }

    #[inline]
    fn get(&self, id: u32) -> Option<u64> {
        self.seek.get(&id).map(|&i| i)
    }

    fn parse(r: &mut io::Read, mut size: u64) -> Result<Seektable,MKVError> {
        let mut seektable = Seektable::new();
        while size > 0 {
            let (i, s, len) = ebml::read_element_id_size(r)?;
            match i {
                ids::SEEK => {
                    let seek = Seek::parse(r, s)?;
                    seektable.seek.insert(seek.id(), seek.position);
                }
                _ => {ebml::skip(r, s)?;}
            }
            size -= s;
            size -= len;
        }
        Ok(seektable)
    }
}

#[derive(Debug)]
struct Seek {
    id: Vec<u8>,
    position: u64
}

impl Seek {
    fn new() -> Seek {
        Seek{id: Vec::new(), position: 0}
    }

    fn id(&self) -> u32 {
        self.id.iter().fold(0, |acc, i| (acc << 8) | *i as u32)
    }

    fn parse(r: &mut io::Read, mut size: u64) -> Result<Seek,MKVError> {
        let mut seek = Seek::new();
        while size > 0 {
            let (i, s, len) = ebml::read_element_id_size(r)?;
            match i {
                ids::SEEKID => {seek.id = ebml::read_bin(r, s)?;}
                ids::SEEKPOSITION => {seek.position = ebml::read_uint(r, s)?;}
                _ => {ebml::skip(r, s)?;}
            }
            size -= s;
            size -= len;
        }
        Ok(seek)
    }
}

/// An Info segment with information pertaining to the entire file
#[derive(Debug)]
pub struct Info {
    /// The file's title
    pub title: Option<String>,
    /// The file's duration
    pub duration: Option<Duration>,
    /// Production date
    pub date_utc: Option<DateTime<Utc>>,
    /// The muxing application or library
    pub muxing_app: String,
    /// The writing application
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

    fn parse(r: &mut io::Read, size: u64) -> Result<Info,MKVError> {
        let mut info = Info::new();
        let mut timecode_scale = None;
        let mut duration = None;

        if let ebml::ElementType::Master(elements) =
            ebml::Element::parse_body(r, ids::INFO, size)? {
            for e in elements {
                match e {
                    ebml::Element{id: ids::TITLE, size: _,
                                  val: ebml::ElementType::UTF8(title)} => {
                        info.title = Some(title);
                    }
                    ebml::Element{id: ids::TIMECODESCALE, size: _,
                                  val: ebml::ElementType::UInt(scale)} => {
                        timecode_scale = Some(scale);
                    }
                    ebml::Element{id: ids::DURATION, size: _,
                                  val: ebml::ElementType::Float(d)} => {
                        duration = Some(d)
                    }
                    ebml::Element{id: ids::DATEUTC, size: _,
                                  val: ebml::ElementType::Date(date)} => {
                        info.date_utc = Some(date)
                    }
                    ebml::Element{id: ids::MUXINGAPP, size: _,
                                  val: ebml::ElementType::UTF8(app)} => {
                        info.muxing_app = app;
                    }
                    ebml::Element{id: ids::WRITINGAPP, size: _,
                                  val: ebml::ElementType::UTF8(app)} => {
                        info.writing_app = app;
                    }
                    _ => {}
                }
            }
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

/// A TrackEntry segment in the Tracks segment container
#[derive(Debug)]
pub struct Track {
    /// The track number, starting from 1
    pub number: u64,
    /// The track's UID
    pub uid: u64,
    /// The track's type
    pub tracktype: Tracktype,
    /// If the track is usable
    pub enabled: bool,
    /// If the track should be active if no other preferences found
    pub default: bool,
    /// If the track *must* be active during playback
    pub forced: bool,
    /// If the track contains blocks using lacing
    pub interlaced: bool,
    /// Duration of each frame
    pub defaultduration: Option<Duration>,
    /// Value to add to the block's timestamp
    pub offset: Option<i64>,
    /// A human-readable track name
    pub name: Option<String>,
    /// The track's language
    pub language: Option<String>,
    /// The track's codec's ID
    pub codec_id: String,
    /// The track's codec's human-readable name
    pub codec_name: Option<String>,
    /// The track's audio or video settings
    pub settings: Settings
}

impl Track {
    fn new() -> Track {
        Track{number: 0,
              uid: 0,
              tracktype: Tracktype::Unknown,
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

    fn parse(r: &mut io::Read, mut size: u64) -> Result<Vec<Track>,MKVError> {
        let mut tracks = Vec::new();

        while size > 0 {
            let (i, s, len) = ebml::read_element_id_size(r)?;
            if i == ids::TRACKENTRY {
                tracks.push(Track::parse_entry(r, s)?);
            } else {
                ebml::skip(r, s)?;
            }

            size -= len;
            size -= s;
        }
        Ok(tracks)
    }

    fn parse_entry(r: &mut io::Read, mut size: u64) -> Result<Track,MKVError> {
        let mut track = Track::new();
        while size > 0 {
            let (i, s, len) = ebml::read_element_id_size(r)?;

            match i {
                ids::TRACKNUMBER => {
                    track.number = ebml::read_uint(r, s)?;
                }
                ids::TRACKUID => {
                    track.uid = ebml::read_uint(r, s)?;
                }
                ids::TRACKTYPE => {
                    track.tracktype = Tracktype::new(ebml::read_uint(r, s)?);
                }
                ids::FLAGENABLED => {
                    track.enabled = ebml::read_uint(r, s)? != 0;
                }
                ids::FLAGDEFAULT => {
                    track.default = ebml::read_uint(r, s)? != 0;
                }
                ids::FLAGFORCED => {
                    track.forced = ebml::read_uint(r, s)? != 0;
                }
                ids::FLAGLACING => {
                    track.interlaced = ebml::read_uint(r, s)? != 0;
                }
                ids::DEFAULTDURATION => {
                    track.defaultduration =
                        Some(Duration::nanoseconds(
                            ebml::read_uint(r, s)? as i64));
                }
                ids::TRACKOFFSET => {
                    track.offset = Some(ebml::read_int(r, s)?);
                }
                ids::NAME => {
                    track.name = Some(ebml::read_utf8(r, s)?);
                }
                ids::LANGUAGE => {
                    track.language = Some(ebml::read_string(r, s)?);
                }
                ids::CODEC_ID => {
                    track.codec_id = ebml::read_string(r, s)?;
                }
                ids::CODEC_NAME => {
                    track.codec_name = Some(ebml::read_utf8(r, s)?);
                }
                ids::VIDEO => {
                    track.settings = Settings::Video(Video::parse(r, s)?);
                }
                ids::AUDIO => {
                    track.settings = Settings::Audio(Audio::parse(r, s)?);
                }
                _ => {ebml::skip(r, s)?;}
            }

            size -= len;
            size -= s;
        }
        Ok(track)
    }
}

/// The type of a given track
#[derive(Debug, PartialEq, Eq)]
pub enum Tracktype {
    /// A video track
    Video,
    /// An audio track
    Audio,
    /// A complex track
    Complex,
    /// A logo track
    Logo,
    /// A subtitle track
    Subtitle,
    /// A buttons track
    Buttons,
    /// A controls track
    Control,
    /// An unknown track type
    Unknown
}

impl Tracktype {
    fn new(tracktype: u64) -> Tracktype {
        match tracktype {
            0x01 => {Tracktype::Video}
            0x02 => {Tracktype::Audio}
            0x03 => {Tracktype::Complex}
            0x10 => {Tracktype::Logo}
            0x11 => {Tracktype::Subtitle}
            0x12 => {Tracktype::Buttons}
            0x20 => {Tracktype::Control}
            _    => {Tracktype::Unknown}
        }
    }
}

/// The settings a track may have
#[derive(Debug)]
pub enum Settings {
    /// No settings (for non audio/video tracks)
    None,
    /// Video settings
    Video(Video),
    /// Audio settings
    Audio(Audio)
}


/// A video track's specifications
#[derive(Debug)]
pub struct Video {
    /// Width of encoded video frames in pixels
    pub pixel_width: u64,
    /// Height of encoded video frames in pixels
    pub pixel_height: u64,
    /// Width of video frames to display
    pub display_width: Option<u64>,
    /// Height of video frames to display
    pub display_height: Option<u64>
}

impl Video {
    fn new() -> Video {
        Video{pixel_width: 0,
              pixel_height: 0,
              display_width: None,
              display_height: None}
    }

    fn parse(r: &mut io::Read, mut size: u64) -> Result<Video,MKVError> {
        let mut video = Video::new();

        while size > 0 {
            let (i, s, len) = ebml::read_element_id_size(r)?;

            match i {
                ids::PIXELWIDTH => {
                    video.pixel_width = ebml::read_uint(r, s)?;
                }
                ids::PIXELHEIGHT => {
                    video.pixel_height = ebml::read_uint(r, s)?;
                }
                ids::DISPLAYWIDTH => {
                    video.display_width = Some(ebml::read_uint(r, s)?);
                }
                ids::DISPLAYHEIGHT => {
                    video.display_height = Some(ebml::read_uint(r, s)?);
                }
                _ => {ebml::skip(r, s)?;}
            }

            size -= len;
            size -= s;
        }

        Ok(video)
    }
}

/// An audio track's specifications
#[derive(Debug)]
pub struct Audio {
    /// The sample rate in Hz
    pub sample_rate: f64,
    /// The number of audio channels
    pub channels: u64,
    /// The bit depth of each sample
    pub bit_depth: Option<u64>
}

impl Audio {
    fn new() -> Audio {
        Audio{sample_rate: 0.0,
              channels: 0,
              bit_depth: None}
    }

    fn parse(r: &mut io::Read, mut size: u64) -> Result<Audio,MKVError> {
        let mut audio = Audio::new();

        while size > 0 {
            let (i, s, len) = ebml::read_element_id_size(r)?;

            match i {
                ids::SAMPLINGFREQUENCY => {
                    audio.sample_rate = ebml::read_float(r, s)?;
                }
                ids::CHANNELS => {
                    audio.channels = ebml::read_uint(r, s)?;
                }
                ids::BITDEPTH => {
                    audio.bit_depth = Some(ebml::read_uint(r, s)?);
                }
                _ => {ebml::skip(r, s)?;}
            }

            size -= len;
            size -= s;
        }

        Ok(audio)
    }
}

/// An attached file (often used for cover art)
#[derive(Debug)]
pub struct Attachment {
    /// A human-friendly name for the file
    pub description: Option<String>,
    /// The file's name
    pub name: String,
    /// The file's MIME type
    pub mime_type: String,
    /// The file's raw data
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
        Result<Vec<Attachment>,MKVError> {
        let mut attachments = Vec::new();

        while size > 0 {
            let (i, s, len) = ebml::read_element_id_size(r)?;

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
        Result<Attachment,MKVError> {
        let mut attachment = Attachment::new();

        while size > 0 {
            let (i, s, len) = ebml::read_element_id_size(r)?;

            match i {
                ids::FILEDESCRIPTION => {
                    attachment.description = Some(ebml::read_utf8(r, s)?);
                }
                ids::FILENAME => {
                    attachment.name = ebml::read_utf8(r, s)?;
                }
                ids::FILEMIMETYPE => {
                    attachment.mime_type = ebml::read_string(r, s)?;
                }
                ids::FILEDATA => {
                    attachment.data = ebml::read_bin(r, s)?;
                }
                _ => {ebml::skip(r, s)?;}
            }

            size -= len;
            size -= s;
        }

        Ok(attachment)
    }
}

/// A complete set of chapters
#[derive(Debug)]
pub struct ChapterEdition {
    /// Whether the chapters should be hidden in the user interface
    pub hidden: bool,
    /// Whether the chapters should be the default
    pub default: bool,
    /// Whether the order to play chapters is enforced
    pub ordered: bool,
    /// The individual chapter entries
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
        Result<Vec<ChapterEdition>,MKVError> {
        let mut chaptereditions = Vec::new();

        while size > 0 {
            let (i, s, len) = ebml::read_element_id_size(r)?;

            if i == ids::EDITIONENTRY {
                chaptereditions.push(ChapterEdition::parse_entry(r, s)?);
            } else {
                ebml::skip(r, s)?;
            }

            size -= s;
            size -= len;
        }

        Ok(chaptereditions)
    }

    fn parse_entry(r: &mut io::Read, mut size: u64) ->
        Result<ChapterEdition,MKVError> {

        let mut chapteredition = ChapterEdition::new();

        while size > 0 {
            let (i, s, len) = ebml::read_element_id_size(r)?;

            match i {
                ids::EDITIONFLAGHIDDEN => {
                    chapteredition.hidden = ebml::read_uint(r, s)? != 0;
                }
                ids::EDITIONFLAGDEFAULT => {
                    chapteredition.default = ebml::read_uint(r, s)? != 0;
                }
                ids::EDITIONFLAGORDERED => {
                    chapteredition.ordered = ebml::read_uint(r, s)? != 0;
                }
                ids::CHAPTERATOM => {
                    chapteredition.chapters.push(Chapter::parse(r, s)?)
                }
                _ => {ebml::skip(r, s)?;}
            }

            size -= s;
            size -= len;
        }

        Ok(chapteredition)
    }
}

/// An individual chapter point
#[derive(Debug)]
pub struct Chapter {
    /// Timestamp of the start of the chapter
    pub time_start: Duration,
    /// Timestamp of the end of the chapter
    pub time_end: Option<Duration>,
    /// Whether the chapter point should be hidden in the user interface
    pub hidden: bool,
    /// Whether the chapter point should be enabled in the user interface
    pub enabled: bool,
    /// Contains all strings to use for displaying chapter
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

    fn parse(r: &mut io::Read, mut size: u64) -> Result<Chapter,MKVError> {
        let mut chapter = Chapter::new();

        while size > 0 {
            let (i, s, len) = ebml::read_element_id_size(r)?;

            match i {
                ids::CHAPTERTIMESTART => {
                    chapter.time_start =
                        Duration::nanoseconds(
                            ebml::read_uint(r, s)? as i64);
                }
                ids::CHAPTERTIMEEND => {
                    chapter.time_end =
                        Some(Duration::nanoseconds(
                            ebml::read_uint(r, s)? as i64));
                }
                ids::CHAPTERFLAGHIDDEN => {
                    chapter.hidden = ebml::read_uint(r, s)? != 0;
                }
                ids::CHAPTERFLAGENABLED => {
                    chapter.enabled = ebml::read_uint(r, s)? != 0;
                }
                ids::CHAPTERDISPLAY => {
                    chapter.display.push(ChapterDisplay::parse(r, s)?);
                }
                _ => {ebml::skip(r, s)?;}
            }

            size -= s;
            size -= len;
        }

        Ok(chapter)
    }
}

/// The display string for a chapter point entry
#[derive(Debug)]
pub struct ChapterDisplay {
    /// The user interface string
    pub string: String,
    /// The string's language
    pub language: String
}

impl ChapterDisplay {
    fn new() -> ChapterDisplay {
        ChapterDisplay{string: String::new(), language: String::new()}
    }

    fn parse(r: &mut io::Read, mut size: u64) ->
        Result<ChapterDisplay,MKVError> {
        let mut display = ChapterDisplay::new();

        while size > 0 {
            let (i, s, len) = ebml::read_element_id_size(r)?;

            match i {
                ids::CHAPSTRING => {
                    display.string = ebml::read_utf8(r, s)?;
                }
                ids::CHAPLANGUAGE => {
                    display.language = ebml::read_string(r, s)?;
                }
                _ => {ebml::skip(r, s)?;}
            }

            size -= s;
            size -= len;
        }

        Ok(display)
    }
}
