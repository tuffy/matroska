// Copyright 2017-2020 Brian Langenberger
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A library for Matroska file metadata parsing functionality
//!
//! Implemented as a set of nested structs with public values
//! which one can use directly.
//!
//! ## Example
//! ```
//! use std::fs::File;
//! use matroska::Matroska;
//! let f = File::open("filename.mkv").unwrap();
//! let matroska = Matroska::open(f).unwrap();
//! println!("title : {:?}", matroska.info.title);
//! ```
//!
//! For additional information about the Matroska format, see the
//! official [specification](https://matroska.org)

#![warn(missing_docs)]

use std::collections::BTreeMap;
use std::fs::File;
use std::io;
use std::time::Duration;

mod ebml;
mod ids;

use chrono::offset::Utc;
use chrono::DateTime;

pub use ebml::MatroskaError;
use ebml::{Element, ElementType, MResult};

/// A Matroska file
#[derive(Debug)]
pub struct Matroska {
    /// The file's Info segment
    pub info: Info,
    /// The file's Tracks segment
    pub tracks: Vec<Track>,
    /// The file's Attachments segment
    pub attachments: Vec<Attachment>,
    /// The file's Chapters segment
    pub chapters: Vec<ChapterEdition>,
}

impl Matroska {
    fn new() -> Matroska {
        Matroska {
            info: Info::new(),
            tracks: Vec::new(),
            attachments: Vec::new(),
            chapters: Vec::new(),
        }
    }

    /// Parses contents of open Matroska file
    pub fn open(mut file: File) -> MResult<Matroska> {
        use std::io::Seek;
        use std::io::SeekFrom;

        let mut matroska = Matroska::new();

        let (mut id_0, mut size_0, _) = ebml::read_element_id_size(&mut file)?;
        while id_0 != ids::SEGMENT {
            file.seek(SeekFrom::Current(size_0 as i64))
                .map(|_| ())
                .map_err(MatroskaError::Io)?;
            let (id, size, _) = ebml::read_element_id_size(&mut file)?;
            id_0 = id;
            size_0 = size;
        }

        let segment_start = file.seek(SeekFrom::Current(0)).map_err(MatroskaError::Io)?;

        while size_0 > 0 {
            let (id_1, size_1, len) = ebml::read_element_id_size(&mut file)?;
            match id_1 {
                ids::SEEKHEAD => {
                    // if seektable encountered, populate file from that
                    let seektable = Seektable::parse(&mut file, size_1)?;
                    if let Some(pos) = seektable.get(ids::INFO) {
                        file.seek(SeekFrom::Start(pos + segment_start))
                            .map_err(MatroskaError::Io)?;
                        let (i, s, _) = ebml::read_element_id_size(&mut file)?;
                        assert_eq!(i, ids::INFO);
                        matroska.info = Info::parse(&mut file, s)?;
                    }
                    if let Some(pos) = seektable.get(ids::TRACKS) {
                        file.seek(SeekFrom::Start(pos + segment_start))
                            .map_err(MatroskaError::Io)?;
                        let (i, s, _) = ebml::read_element_id_size(&mut file)?;
                        assert_eq!(i, ids::TRACKS);
                        matroska.tracks = Track::parse(&mut file, s)?;
                    }
                    if let Some(pos) = seektable.get(ids::ATTACHMENTS) {
                        file.seek(SeekFrom::Start(pos + segment_start))
                            .map_err(MatroskaError::Io)?;
                        let (i, s, _) = ebml::read_element_id_size(&mut file)?;
                        assert_eq!(i, ids::ATTACHMENTS);
                        matroska.attachments = Attachment::parse(&mut file, s)?;
                    }
                    if let Some(pos) = seektable.get(ids::CHAPTERS) {
                        file.seek(SeekFrom::Start(pos + segment_start))
                            .map_err(MatroskaError::Io)?;
                        let (i, s, _) = ebml::read_element_id_size(&mut file)?;
                        assert_eq!(i, ids::CHAPTERS);
                        matroska.chapters = ChapterEdition::parse(&mut file, s)?;
                    }
                    return Ok(matroska);
                }
                // if no seektable, populate file from parts
                ids::INFO => {
                    matroska.info = Info::parse(&mut file, size_1)?;
                }
                ids::TRACKS => {
                    matroska.tracks = Track::parse(&mut file, size_1)?;
                }
                ids::ATTACHMENTS => {
                    matroska.attachments = Attachment::parse(&mut file, size_1)?;
                }
                ids::CHAPTERS => {
                    matroska.chapters = ChapterEdition::parse(&mut file, size_1)?;
                }
                _ => {
                    file.seek(SeekFrom::Current(size_1 as i64))
                        .map(|_| ())
                        .map_err(MatroskaError::Io)?;
                }
            }
            size_0 -= len;
            size_0 -= size_1;
        }

        Ok(matroska)
    }

    /// Returns all tracks with a type of "video"
    pub fn video_tracks(&self) -> Vec<&Track> {
        self.tracks
            .iter()
            .filter(|t| t.tracktype == Tracktype::Video)
            .collect()
    }

    /// Returns all tracks with a type of "audio"
    pub fn audio_tracks(&self) -> Vec<&Track> {
        self.tracks
            .iter()
            .filter(|t| t.tracktype == Tracktype::Audio)
            .collect()
    }

    /// Returns all tracks with a type of "subtitle"
    pub fn subtitle_tracks(&self) -> Vec<&Track> {
        self.tracks
            .iter()
            .filter(|t| t.tracktype == Tracktype::Subtitle)
            .collect()
    }
}

#[derive(Debug)]
struct Seektable {
    seek: BTreeMap<u32, u64>,
}

impl Seektable {
    fn new() -> Seektable {
        Seektable {
            seek: BTreeMap::new(),
        }
    }

    #[inline]
    fn get(&self, id: u32) -> Option<u64> {
        self.seek.get(&id).cloned()
    }

    fn parse(r: &mut dyn io::Read, size: u64) -> MResult<Seektable> {
        let mut seektable = Seektable::new();
        for e in Element::parse_master(r, size)? {
            if let Element {
                id: ids::SEEK,
                val: ElementType::Master(sub_elements),
                ..
            } = e
            {
                let seek = Seek::build(sub_elements);
                seektable.seek.insert(seek.id(), seek.position);
            }
        }
        Ok(seektable)
    }
}

#[derive(Debug)]
struct Seek {
    id: Vec<u8>,
    position: u64,
}

impl Seek {
    fn new() -> Seek {
        Seek {
            id: Vec::new(),
            position: 0,
        }
    }

    fn id(&self) -> u32 {
        self.id.iter().fold(0, |acc, i| (acc << 8) | u32::from(*i))
    }

    fn build(elements: Vec<Element>) -> Seek {
        let mut seek = Seek::new();
        for e in elements {
            match e {
                Element {
                    id: ids::SEEKID,
                    val: ElementType::Binary(id),
                    ..
                } => {
                    seek.id = id;
                }
                Element {
                    id: ids::SEEKPOSITION,
                    val: ElementType::UInt(position),
                    ..
                } => {
                    seek.position = position;
                }
                _ => {}
            }
        }
        seek
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
    pub writing_app: String,
}

impl Info {
    fn new() -> Info {
        Info {
            title: None,
            duration: None,
            date_utc: None,
            muxing_app: String::new(),
            writing_app: String::new(),
        }
    }

    fn parse(r: &mut dyn io::Read, size: u64) -> MResult<Info> {
        let mut info = Info::new();
        let mut timecode_scale = None;
        let mut duration = None;

        for e in Element::parse_master(r, size)? {
            match e {
                Element {
                    id: ids::TITLE,
                    val: ElementType::UTF8(title),
                    ..
                } => {
                    info.title = Some(title);
                }
                Element {
                    id: ids::TIMECODESCALE,
                    val: ElementType::UInt(scale),
                    ..
                } => {
                    timecode_scale = Some(scale);
                }
                Element {
                    id: ids::DURATION,
                    val: ElementType::Float(d),
                    ..
                } => duration = Some(d),
                Element {
                    id: ids::DATEUTC,
                    val: ElementType::Date(date),
                    ..
                } => info.date_utc = Some(date),
                Element {
                    id: ids::MUXINGAPP,
                    val: ElementType::UTF8(app),
                    ..
                } => {
                    info.muxing_app = app;
                }
                Element {
                    id: ids::WRITINGAPP,
                    val: ElementType::UTF8(app),
                    ..
                } => {
                    info.writing_app = app;
                }
                _ => {}
            }
        }

        if let Some(d) = duration {
            if let Some(t) = timecode_scale {
                info.duration = Some(Duration::from_nanos((d * t as f64) as u64))
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
    pub default_duration: Option<Duration>,
    /// A human-readable track name
    pub name: Option<String>,
    /// The track's language
    pub language: Option<String>,
    /// The track's codec's ID
    pub codec_id: String,
    /// The track's codec's human-readable name
    pub codec_name: Option<String>,
    /// The track's audio or video settings
    pub settings: Settings,
}

impl Track {
    fn new() -> Track {
        Track {
            number: 0,
            uid: 0,
            tracktype: Tracktype::Unknown,
            enabled: true,
            default: true,
            forced: false,
            interlaced: true,
            default_duration: None,
            name: None,
            language: None,
            codec_id: String::new(),
            codec_name: None,
            settings: Settings::None,
        }
    }

    fn parse(r: &mut dyn io::Read, size: u64) -> MResult<Vec<Track>> {
        Element::parse_master(r, size).map(|elements| {
            elements
                .into_iter()
                .filter_map(|e| match e {
                    Element {
                        id: ids::TRACKENTRY,
                        val: ElementType::Master(sub_elements),
                        ..
                    } => Some(Track::build_entry(sub_elements)),
                    _ => None,
                })
                .collect()
        })
    }

    fn build_entry(elements: Vec<Element>) -> Track {
        let mut track = Track::new();
        for e in elements {
            match e {
                Element {
                    id: ids::TRACKNUMBER,
                    val: ElementType::UInt(number),
                    ..
                } => {
                    track.number = number;
                }
                Element {
                    id: ids::TRACKUID,
                    val: ElementType::UInt(uid),
                    ..
                } => {
                    track.uid = uid;
                }
                Element {
                    id: ids::TRACKTYPE,
                    val: ElementType::UInt(tracktype),
                    ..
                } => {
                    track.tracktype = Tracktype::new(tracktype);
                }
                Element {
                    id: ids::FLAGENABLED,
                    val: ElementType::UInt(enabled),
                    ..
                } => {
                    track.enabled = enabled != 0;
                }
                Element {
                    id: ids::FLAGDEFAULT,
                    val: ElementType::UInt(default),
                    ..
                } => {
                    track.default = default != 0;
                }
                Element {
                    id: ids::FLAGFORCED,
                    val: ElementType::UInt(forced),
                    ..
                } => {
                    track.forced = forced != 0;
                }
                Element {
                    id: ids::FLAGLACING,
                    val: ElementType::UInt(lacing),
                    ..
                } => {
                    track.interlaced = lacing != 0;
                }
                Element {
                    id: ids::DEFAULTDURATION,
                    val: ElementType::UInt(duration),
                    ..
                } => {
                    track.default_duration = Some(Duration::from_nanos(duration));
                }
                Element {
                    id: ids::NAME,
                    val: ElementType::UTF8(name),
                    ..
                } => {
                    track.name = Some(name);
                }
                Element {
                    id: ids::LANGUAGE,
                    val: ElementType::String(language),
                    ..
                } => {
                    track.language = Some(language);
                }
                Element {
                    id: ids::CODEC_ID,
                    val: ElementType::String(codec_id),
                    ..
                } => {
                    track.codec_id = codec_id;
                }
                Element {
                    id: ids::CODEC_NAME,
                    val: ElementType::UTF8(codec_name),
                    ..
                } => {
                    track.codec_name = Some(codec_name);
                }
                Element {
                    id: ids::VIDEO,
                    val: ElementType::Master(sub_elements),
                    ..
                } => {
                    track.settings = Settings::Video(Video::build(sub_elements));
                }
                Element {
                    id: ids::AUDIO,
                    val: ElementType::Master(sub_elements),
                    ..
                } => {
                    track.settings = Settings::Audio(Audio::build(sub_elements));
                }
                _ => {}
            }
        }
        track
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
    Unknown,
}

impl Tracktype {
    fn new(tracktype: u64) -> Tracktype {
        match tracktype {
            0x01 => Tracktype::Video,
            0x02 => Tracktype::Audio,
            0x03 => Tracktype::Complex,
            0x10 => Tracktype::Logo,
            0x11 => Tracktype::Subtitle,
            0x12 => Tracktype::Buttons,
            0x20 => Tracktype::Control,
            _ => Tracktype::Unknown,
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
    Audio(Audio),
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
    pub display_height: Option<u64>,
}

impl Video {
    fn new() -> Video {
        Video {
            pixel_width: 0,
            pixel_height: 0,
            display_width: None,
            display_height: None,
        }
    }

    fn build(elements: Vec<Element>) -> Video {
        let mut video = Video::new();
        for e in elements {
            match e {
                Element {
                    id: ids::PIXELWIDTH,
                    val: ElementType::UInt(width),
                    ..
                } => {
                    video.pixel_width = width;
                }
                Element {
                    id: ids::PIXELHEIGHT,
                    val: ElementType::UInt(height),
                    ..
                } => {
                    video.pixel_height = height;
                }
                Element {
                    id: ids::DISPLAYWIDTH,
                    val: ElementType::UInt(width),
                    ..
                } => {
                    video.display_width = Some(width);
                }
                Element {
                    id: ids::DISPLAYHEIGHT,
                    val: ElementType::UInt(height),
                    ..
                } => video.display_height = Some(height),
                _ => {}
            }
        }
        video
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
    pub bit_depth: Option<u64>,
}

impl Audio {
    fn new() -> Audio {
        Audio {
            sample_rate: 0.0,
            channels: 0,
            bit_depth: None,
        }
    }

    fn build(elements: Vec<Element>) -> Audio {
        let mut audio = Audio::new();
        for e in elements {
            match e {
                Element {
                    id: ids::SAMPLINGFREQUENCY,
                    val: ElementType::Float(frequency),
                    ..
                } => {
                    audio.sample_rate = frequency;
                }
                Element {
                    id: ids::CHANNELS,
                    val: ElementType::UInt(channels),
                    ..
                } => {
                    audio.channels = channels;
                }
                Element {
                    id: ids::BITDEPTH,
                    val: ElementType::UInt(bit_depth),
                    ..
                } => {
                    audio.bit_depth = Some(bit_depth);
                }
                _ => {}
            }
        }
        audio
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
    pub data: Vec<u8>,
}

impl Attachment {
    fn new() -> Attachment {
        Attachment {
            description: None,
            name: String::new(),
            mime_type: String::new(),
            data: Vec::new(),
        }
    }

    fn parse(r: &mut dyn io::Read, size: u64) -> MResult<Vec<Attachment>> {
        Element::parse_master(r, size).map(|elements| {
            elements
                .into_iter()
                .filter_map(|e| match e {
                    Element {
                        id: ids::ATTACHEDFILE,
                        val: ElementType::Master(sub_elements),
                        ..
                    } => Some(Attachment::build_entry(sub_elements)),
                    _ => None,
                })
                .collect()
        })
    }

    fn build_entry(elements: Vec<Element>) -> Attachment {
        let mut attachment = Attachment::new();
        for e in elements {
            match e {
                Element {
                    id: ids::FILEDESCRIPTION,
                    val: ElementType::UTF8(description),
                    ..
                } => {
                    attachment.description = Some(description);
                }
                Element {
                    id: ids::FILENAME,
                    val: ElementType::UTF8(filename),
                    ..
                } => {
                    attachment.name = filename;
                }
                Element {
                    id: ids::FILEMIMETYPE,
                    val: ElementType::String(mime_type),
                    ..
                } => {
                    attachment.mime_type = mime_type;
                }
                Element {
                    id: ids::FILEDATA,
                    val: ElementType::Binary(data),
                    ..
                } => {
                    attachment.data = data;
                }
                _ => {}
            }
        }
        attachment
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
    pub chapters: Vec<Chapter>,
}

impl ChapterEdition {
    fn new() -> ChapterEdition {
        ChapterEdition {
            hidden: false,
            default: false,
            ordered: false,
            chapters: Vec::new(),
        }
    }

    fn parse(r: &mut dyn io::Read, size: u64) -> MResult<Vec<ChapterEdition>> {
        Element::parse_master(r, size).map(|elements| {
            elements
                .into_iter()
                .filter_map(|e| match e {
                    Element {
                        id: ids::EDITIONENTRY,
                        val: ElementType::Master(sub_elements),
                        ..
                    } => Some(ChapterEdition::build_entry(sub_elements)),
                    _ => None,
                })
                .collect()
        })
    }

    fn build_entry(elements: Vec<Element>) -> ChapterEdition {
        let mut chapteredition = ChapterEdition::new();
        for e in elements {
            match e {
                Element {
                    id: ids::EDITIONFLAGHIDDEN,
                    val: ElementType::UInt(hidden),
                    ..
                } => {
                    chapteredition.hidden = hidden != 0;
                }
                Element {
                    id: ids::EDITIONFLAGDEFAULT,
                    val: ElementType::UInt(default),
                    ..
                } => {
                    chapteredition.default = default != 0;
                }
                Element {
                    id: ids::EDITIONFLAGORDERED,
                    val: ElementType::UInt(ordered),
                    ..
                } => {
                    chapteredition.ordered = ordered != 0;
                }
                Element {
                    id: ids::CHAPTERATOM,
                    val: ElementType::Master(sub_elements),
                    ..
                } => {
                    chapteredition.chapters.push(Chapter::build(sub_elements));
                }
                _ => {}
            }
        }
        chapteredition
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
    pub display: Vec<ChapterDisplay>,
}

impl Chapter {
    fn new() -> Chapter {
        Chapter {
            time_start: Duration::default(),
            time_end: None,
            hidden: false,
            enabled: false,
            display: Vec::new(),
        }
    }

    fn build(elements: Vec<Element>) -> Chapter {
        let mut chapter = Chapter::new();
        for e in elements {
            match e {
                Element {
                    id: ids::CHAPTERTIMESTART,
                    val: ElementType::UInt(start),
                    ..
                } => {
                    chapter.time_start = Duration::from_nanos(start);
                }
                Element {
                    id: ids::CHAPTERTIMEEND,
                    val: ElementType::UInt(end),
                    ..
                } => {
                    chapter.time_end = Some(Duration::from_nanos(end));
                }
                Element {
                    id: ids::CHAPTERFLAGHIDDEN,
                    val: ElementType::UInt(hidden),
                    ..
                } => {
                    chapter.hidden = hidden != 0;
                }
                Element {
                    id: ids::CHAPTERFLAGENABLED,
                    val: ElementType::UInt(enabled),
                    ..
                } => {
                    chapter.enabled = enabled != 0;
                }
                Element {
                    id: ids::CHAPTERDISPLAY,
                    val: ElementType::Master(sub_elements),
                    ..
                } => {
                    chapter.display.push(ChapterDisplay::build(sub_elements));
                }
                _ => {}
            }
        }
        chapter
    }
}

/// The display string for a chapter point entry
#[derive(Debug)]
pub struct ChapterDisplay {
    /// The user interface string
    pub string: String,
    /// The string's language
    pub language: String,
}

impl ChapterDisplay {
    fn new() -> ChapterDisplay {
        ChapterDisplay {
            string: String::new(),
            language: String::new(),
        }
    }

    fn build(elements: Vec<Element>) -> ChapterDisplay {
        let mut display = ChapterDisplay::new();
        for e in elements {
            match e {
                Element {
                    id: ids::CHAPSTRING,
                    val: ElementType::UTF8(string),
                    ..
                } => {
                    display.string = string;
                }
                Element {
                    id: ids::CHAPLANGUAGE,
                    val: ElementType::String(language),
                    ..
                } => {
                    display.language = language;
                }
                _ => {}
            }
        }
        display
    }
}
