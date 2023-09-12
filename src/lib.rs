// Copyright 2017-2022 Brian Langenberger
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
//! ## Example 1
//! ```no_run
//! let matroska = matroska::open("file.mkv").unwrap();
//! println!("title : {:?}", matroska.info.title);
//! ```
//!
//! ## Example 2
//! ```no_run
//! use matroska::Info;
//! if let Ok(Some(Info { duration, ..})) = matroska::get_from::<_, Info>("file.mkv") {
//!     println!("duration : {:?}", duration);
//! }
//! ```
//!
//! For additional information about the Matroska format, see the
//! official [specification](https://matroska.org)

#![warn(missing_docs)]
#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::io;
use std::time::Duration;

mod ebml;
mod ids;

pub use ebml::{DateTime, MatroskaError};
use ebml::{Element, ElementType, Result};

/// A possible error when reading or parsing a Matroska file
pub type Error = MatroskaError;

/// A Matroska file
#[derive(Debug, Clone)]
pub struct Matroska {
    /// The file's Info segment
    pub info: Info,
    /// The file's Tracks segment
    pub tracks: Vec<Track>,
    /// The file's Attachments segment
    pub attachments: Vec<Attachment>,
    /// The file's Chapters segment
    pub chapters: Vec<ChapterEdition>,
    /// The file's Tags segment
    pub tags: Vec<Tag>,
}

impl Matroska {
    fn new() -> Matroska {
        Matroska {
            info: Info::new(),
            tracks: Vec::new(),
            attachments: Vec::new(),
            chapters: Vec::new(),
            tags: Vec::new(),
        }
    }

    /// Parses contents of open Matroska file
    pub fn open<R: io::Read + io::Seek>(mut file: R) -> Result<Matroska> {
        use std::io::SeekFrom;

        let mut matroska = Matroska::new();

        let (mut id_0, mut size_0, _) = ebml::read_element_id_size(&mut file)?;
        while id_0 != ids::SEGMENT {
            file.seek(SeekFrom::Current(size_0 as i64)).map(|_| ())?;
            let (id, size, _) = ebml::read_element_id_size(&mut file)?;
            id_0 = id;
            size_0 = size;
        }

        let segment_start = file.stream_position()?;

        while size_0 > 0 {
            let (id_1, size_1, len) = ebml::read_element_id_size(&mut file)?;
            match id_1 {
                ids::SEEKHEAD => {
                    // if seektable encountered, populate file from that
                    let seektable = Seektable::parse(&mut file, segment_start, size_1)?;

                    if let Some(pos) = seektable.get(ids::INFO)? {
                        file.seek(SeekFrom::Start(pos))?;
                        let (i, s, _) = ebml::read_element_id_size(&mut file)?;
                        assert_eq!(i, ids::INFO);
                        matroska.info = Info::parse(&mut file, s)?;
                    }
                    if let Some(pos) = seektable.get(ids::TRACKS)? {
                        file.seek(SeekFrom::Start(pos))?;
                        let (i, s, _) = ebml::read_element_id_size(&mut file)?;
                        assert_eq!(i, ids::TRACKS);
                        matroska.tracks = Track::parse(&mut file, s)?;
                    }
                    if let Some(pos) = seektable.get(ids::ATTACHMENTS)? {
                        file.seek(SeekFrom::Start(pos))?;
                        let (i, s, _) = ebml::read_element_id_size(&mut file)?;
                        assert_eq!(i, ids::ATTACHMENTS);
                        matroska.attachments = Attachment::parse(&mut file, s)?;
                    }
                    if let Some(pos) = seektable.get(ids::CHAPTERS)? {
                        file.seek(SeekFrom::Start(pos))?;
                        let (i, s, _) = ebml::read_element_id_size(&mut file)?;
                        assert_eq!(i, ids::CHAPTERS);
                        matroska.chapters = ChapterEdition::parse(&mut file, s)?;
                    }
                    if let Some(pos) = seektable.get(ids::TAGS)? {
                        file.seek(SeekFrom::Start(pos))?;
                        let (i, s, _) = ebml::read_element_id_size(&mut file)?;
                        assert_eq!(i, ids::TAGS);
                        matroska.tags = Tag::parse(&mut file, s)?;
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
                ids::TAGS => {
                    matroska.tags = Tag::parse(&mut file, size_1)?;
                }
                _ => {
                    file.seek(SeekFrom::Current(size_1 as i64)).map(|_| ())?;
                }
            }
            size_0 -= len;
            size_0 -= size_1;
        }

        Ok(matroska)
    }

    /// Returns a single item from the Matroska file such as Info
    #[deprecated(since = "0.21.0", note = "use matroska::get() function instead")]
    pub fn get<R, P>(file: R) -> Result<Option<P::Output>>
    where
        R: io::Read + io::Seek,
        P: Parseable,
    {
        get::<R, P>(file)
    }

    /// Returns all tracks with a type of "video"
    pub fn video_tracks(&self) -> impl Iterator<Item = &Track> {
        self.tracks.iter().filter(|t| t.is_video())
    }

    /// Returns all tracks with a type of "audio"
    pub fn audio_tracks(&self) -> impl Iterator<Item = &Track> {
        self.tracks.iter().filter(|t| t.is_audio())
    }

    /// Returns all tracks with a type of "subtitle"
    pub fn subtitle_tracks(&self) -> impl Iterator<Item = &Track> {
        self.tracks.iter().filter(|t| t.is_subtitle())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Seektable {
    offset: u64, // The file offset of the Seektable
    seek: BTreeMap<u32, u64>,
}

impl Seektable {
    fn new(offset: u64) -> Seektable {
        Seektable {
            offset,
            seek: BTreeMap::new(),
        }
    }

    #[inline]
    fn get(&self, id: u32) -> Result<Option<u64>> {
        if let Some(position) = self.seek.get(&id) {
            if let Some(offset) = self.offset.checked_add(*position) {
                Ok(Some(offset))
            } else {
                Err(MatroskaError::InvalidSeekHead { id })
            }
        } else {
            Ok(None)
        }
    }

    fn parse<R>(r: &mut R, segment_start: u64, mut size: u64) -> Result<Seektable>
    where
        R: io::Read + io::Seek,
    {
        let mut seektable = Seektable::new(segment_start);
        loop {
            for e in Element::parse_master(r, size, Some(ids::SEGMENT))? {
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

            match seektable.seek.remove(&ids::SEEKHEAD) {
                Some(next_table) => {
                    r.seek(io::SeekFrom::Start(next_table + segment_start))?;
                    let (id, new_size, _) = ebml::read_element_id_size(r)?;
                    assert!(id == ids::SEEKHEAD);
                    size = new_size;
                }
                None => break Ok(seektable),
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

/// An element which can be parsed from the Matroska stream
pub trait Parseable {
    /// What to parse from the stream, such as ourself or a `Vec` of ourselves
    type Output;

    /// Our Matroska element ID
    const ID: u32;

    /// Performs the actual parsing
    fn parse<R: io::Read>(r: &mut R, size: u64) -> Result<Self::Output>;
}

/// An Info segment with information pertaining to the entire file
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Info {
    /// The file's UID
    pub uid: Option<Vec<u8>>,
    /// Unique ID of the previous segment
    pub prev_uid: Option<Vec<u8>>,
    /// Unique ID of the next segment
    pub next_uid: Option<Vec<u8>>,
    /// Unique IDs of the families this segment belongs to
    pub family_uids: Vec<Vec<u8>>,
    /// The file's title
    pub title: Option<String>,
    /// The file's duration
    pub duration: Option<Duration>,
    /// Production date
    pub date_utc: Option<DateTime>,
    /// The muxing application or library
    pub muxing_app: String,
    /// The writing application
    pub writing_app: String,
}

impl Info {
    fn new() -> Info {
        Info {
            uid: None,
            prev_uid: None,
            next_uid: None,
            family_uids: Vec::new(),
            title: None,
            duration: None,
            date_utc: None,
            muxing_app: String::new(),
            writing_app: String::new(),
        }
    }
}

impl Parseable for Info {
    type Output = Info;

    const ID: u32 = ids::INFO;

    fn parse<R: io::Read>(r: &mut R, size: u64) -> Result<Info> {
        let mut info = Info::new();
        let mut timecode_scale = 1000000;
        let mut duration = None;

        for e in Element::parse_master(r, size, Some(ids::INFO))? {
            match e {
                Element {
                    id: ids::SEGMENTUID,
                    val: ElementType::Binary(uid),
                    ..
                } => {
                    info.uid = Some(uid);
                }
                Element {
                    id: ids::PREVUID,
                    val: ElementType::Binary(uid),
                    ..
                } => {
                    info.prev_uid = Some(uid);
                }
                Element {
                    id: ids::NEXTUID,
                    val: ElementType::Binary(uid),
                    ..
                } => {
                    info.next_uid = Some(uid);
                }
                Element {
                    id: ids::SEGMENTFAMILY,
                    val: ElementType::Binary(uid),
                    ..
                } => {
                    info.family_uids.push(uid);
                }
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
                    timecode_scale = scale;
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
            info.duration = Some(Duration::from_nanos((d * timecode_scale as f64) as u64))
        }

        Ok(info)
    }
}

/// A TrackEntry segment in the Tracks segment container
#[derive(Debug, Clone, PartialEq)]
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

    /// If the track is suitable for users with hearing impairments
    pub hearing_impaired: Option<bool>,

    /// If the track is suitable for users with visual impairments
    pub visual_impaired: Option<bool>,

    /// If the track contains textual descriptions of video content
    pub text_descriptions: Option<bool>,

    /// If the track is in the content's original language
    pub original: Option<bool>,

    /// If the track contains commentary
    pub commentary: Option<bool>,

    /// If the track contains blocks using lacing
    pub interlaced: bool,

    /// Duration of each frame
    pub default_duration: Option<Duration>,

    /// A human-readable track name
    pub name: Option<String>,

    /// The track's language
    pub language: Option<Language>,

    /// The track's codec's ID
    pub codec_id: String,

    /// Private data known only to the codec
    pub codec_private: Option<Vec<u8>>,

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
            hearing_impaired: None,
            visual_impaired: None,
            text_descriptions: None,
            original: None,
            commentary: None,
            interlaced: true,
            default_duration: None,
            name: None,
            language: None,
            codec_id: String::new(),
            codec_private: None,
            codec_name: None,
            settings: Settings::None,
        }
    }

    /// returns `true` if track is video
    #[inline]
    pub fn is_video(&self) -> bool {
        matches!(self.tracktype, Tracktype::Video)
    }

    /// returns `true` if track is audio
    #[inline]
    pub fn is_audio(&self) -> bool {
        matches!(self.tracktype, Tracktype::Audio)
    }

    /// returns `true` if track is subtitle
    #[inline]
    pub fn is_subtitle(&self) -> bool {
        matches!(self.tracktype, Tracktype::Subtitle)
    }

    fn build_entry(elements: Vec<Element>) -> Track {
        let mut track = Track::new();
        for e in elements {
            // although the official specification lists
            // the hearing impaired, visual impaired, text descriptions,
            // original and commentary flags as unsigned ints,
            // mpvpropedit sets them as binary, so I will support both

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
                    id: ids::FLAGHEARINGIMPAIRED,
                    val: ElementType::Binary(hearing_impaired),
                    ..
                } => {
                    track.hearing_impaired = hearing_impaired.first().map(|b| *b != 0);
                }
                Element {
                    id: ids::FLAGHEARINGIMPAIRED,
                    val: ElementType::UInt(hearing_impaired),
                    ..
                } => {
                    track.hearing_impaired = Some(hearing_impaired != 0);
                }
                Element {
                    id: ids::FLAGVISUALIMPAIRED,
                    val: ElementType::Binary(visual_impaired),
                    ..
                } => {
                    track.visual_impaired = visual_impaired.first().map(|b| *b != 0);
                }
                Element {
                    id: ids::FLAGVISUALIMPAIRED,
                    val: ElementType::UInt(visual_impaired),
                    ..
                } => {
                    track.visual_impaired = Some(visual_impaired != 0);
                }
                Element {
                    id: ids::FLAGTEXTDESCRIPTIONS,
                    val: ElementType::Binary(text_descriptions),
                    ..
                } => {
                    track.text_descriptions = text_descriptions.first().map(|b| *b != 0);
                }
                Element {
                    id: ids::FLAGTEXTDESCRIPTIONS,
                    val: ElementType::UInt(text_descriptions),
                    ..
                } => {
                    track.text_descriptions = Some(text_descriptions != 0);
                }
                Element {
                    id: ids::FLAGORIGINAL,
                    val: ElementType::Binary(original),
                    ..
                } => {
                    track.original = original.first().map(|b| *b != 0);
                }
                Element {
                    id: ids::FLAGORIGINAL,
                    val: ElementType::UInt(original),
                    ..
                } => {
                    track.original = Some(original != 0);
                }
                Element {
                    id: ids::FLAGCOMMENTARY,
                    val: ElementType::Binary(commentary),
                    ..
                } => {
                    track.commentary = commentary.first().map(|b| *b != 0);
                }
                Element {
                    id: ids::FLAGCOMMENTARY,
                    val: ElementType::UInt(commentary),
                    ..
                } => {
                    track.commentary = Some(commentary != 0);
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
                    if !matches!(track.language, Some(Language::IETF(_))) {
                        track.language = Some(Language::ISO639(language));
                    }
                }
                Element {
                    id: ids::LANGUAGE_IETF,
                    val: ElementType::String(language),
                    ..
                } => {
                    track.language = Some(Language::IETF(language));
                }
                Element {
                    id: ids::CODEC_ID,
                    val: ElementType::String(codec_id),
                    ..
                } => {
                    track.codec_id = codec_id;
                }
                Element {
                    id: ids::CODEC_PRIVATE,
                    val: ElementType::Binary(private_data),
                    ..
                } => {
                    track.codec_private = Some(private_data);
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

impl Parseable for Track {
    type Output = Vec<Track>;

    const ID: u32 = ids::TRACKS;

    fn parse<R: io::Read>(r: &mut R, size: u64) -> Result<Vec<Track>> {
        Element::parse_master(r, size, Some(ids::TRACKENTRY)).map(|elements| {
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
}

/// The type of a given track
#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
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
#[derive(Debug, Clone, PartialEq)]
pub enum Settings {
    /// No settings (for non audio/video tracks)
    None,
    /// Video settings
    Video(Video),
    /// Audio settings
    Audio(Audio),
}

/// A video track's specifications
#[derive(Debug, Clone, PartialEq)]
pub struct Video {
    /// Width of encoded video frames in pixels
    pub pixel_width: u64,
    /// Height of encoded video frames in pixels
    pub pixel_height: u64,
    /// Width of video frames to display
    pub display_width: Option<u64>,
    /// Height of video frames to display
    pub display_height: Option<u64>,
    /// Whether video is interlaced
    pub interlaced: Option<bool>,
    /// Stereo video mode
    pub stereo: Option<StereoMode>,
    /// Gamma
    pub gamma: Option<f64>,
}

impl Video {
    fn new() -> Video {
        Video {
            pixel_width: 0,
            pixel_height: 0,
            display_width: None,
            display_height: None,
            interlaced: None,
            stereo: None,
            gamma: None,
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
                Element {
                    id: ids::INTERLACED,
                    val: ElementType::UInt(interlaced),
                    ..
                } => {
                    video.interlaced = match interlaced {
                        1 => Some(true),
                        2 => Some(false),
                        _ => None,
                    }
                }
                Element {
                    id: ids::GAMMA,
                    val: ElementType::Float(gamma),
                    ..
                } => {
                    video.gamma = Some(gamma);
                }
                Element {
                    id: ids::STEREOMODE,
                    val: ElementType::UInt(stereo),
                    ..
                } => {
                    video.stereo = match stereo {
                        0 => Some(StereoMode::Mono),
                        1 => Some(StereoMode::SideBySide(EyeOrder::LeftFirst)),
                        2 => Some(StereoMode::TopBottom(EyeOrder::RightFirst)),
                        3 => Some(StereoMode::TopBottom(EyeOrder::LeftFirst)),
                        4 => Some(StereoMode::Checkboard(EyeOrder::RightFirst)),
                        5 => Some(StereoMode::Checkboard(EyeOrder::LeftFirst)),
                        6 => Some(StereoMode::RowInterleaved(EyeOrder::RightFirst)),
                        7 => Some(StereoMode::RowInterleaved(EyeOrder::LeftFirst)),
                        8 => Some(StereoMode::ColumnInterleaved(EyeOrder::RightFirst)),
                        9 => Some(StereoMode::ColumnInterleaved(EyeOrder::LeftFirst)),
                        10 => Some(StereoMode::Anaglyph(StereoColors::CyanRed)),
                        11 => Some(StereoMode::SideBySide(EyeOrder::RightFirst)),
                        12 => Some(StereoMode::Anaglyph(StereoColors::GreenMagenta)),
                        13 => Some(StereoMode::Interlaced(EyeOrder::LeftFirst)),
                        14 => Some(StereoMode::Interlaced(EyeOrder::RightFirst)),
                        _ => None,
                    }
                }
                _ => {}
            }
        }
        video
    }
}

/// How a video track may be displayed in stereo mode
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum StereoMode {
    /// mono
    Mono,
    /// side-by-side
    SideBySide(EyeOrder),
    /// top - bottom
    TopBottom(EyeOrder),
    /// checkboard
    Checkboard(EyeOrder),
    /// row interleaved
    RowInterleaved(EyeOrder),
    /// column interleaved
    ColumnInterleaved(EyeOrder),
    /// anaglyph
    Anaglyph(StereoColors),
    /// interlaced
    Interlaced(EyeOrder),
}

impl std::fmt::Display for StereoMode {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        match self {
            StereoMode::Mono => write!(f, "mono"),
            StereoMode::SideBySide(o) => write!(f, "side by side ({o})"),
            StereoMode::TopBottom(o) => write!(f, "top - bottom ({o})"),
            StereoMode::Checkboard(o) => write!(f, "checkboard ({o})"),
            StereoMode::RowInterleaved(o) => write!(f, "row interleaved ({o})"),
            StereoMode::ColumnInterleaved(o) => write!(f, "column interleaved ({o})"),
            StereoMode::Anaglyph(c) => write!(f, "anaglyph ({c})"),
            StereoMode::Interlaced(o) => write!(f, "interlaced ({o})"),
        }
    }
}

/// Which eye is displayed first
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub enum EyeOrder {
    /// left eye is displayed first
    LeftFirst,
    /// right eye is displayed first
    RightFirst,
}

impl std::fmt::Display for EyeOrder {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        match self {
            EyeOrder::LeftFirst => write!(f, "left eye is first"),
            EyeOrder::RightFirst => write!(f, "right eye is first"),
        }
    }
}

/// Which colors are used for anaglyph stereo 3D
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub enum StereoColors {
    /// cyan/red
    CyanRed,
    /// green/magenta
    GreenMagenta,
}

impl std::fmt::Display for StereoColors {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        match self {
            StereoColors::CyanRed => write!(f, "cyan/red"),
            StereoColors::GreenMagenta => write!(f, "green/magenta"),
        }
    }
}

/// An audio track's specifications
#[derive(Debug, Clone, PartialEq)]
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
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

impl Parseable for Attachment {
    type Output = Vec<Attachment>;

    const ID: u32 = ids::ATTACHMENTS;

    fn parse<R: io::Read>(r: &mut R, size: u64) -> Result<Vec<Attachment>> {
        Element::parse_master(r, size, Some(ids::ATTACHEDFILE)).map(|elements| {
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
}

/// A complete set of chapters
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ChapterEdition {
    /// The edition's UID
    pub uid: Option<u64>,
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
            uid: None,
            hidden: false,
            default: false,
            ordered: false,
            chapters: Vec::new(),
        }
    }

    fn build_entry(elements: Vec<Element>) -> ChapterEdition {
        let mut chapteredition = ChapterEdition::new();
        for e in elements {
            match e {
                Element {
                    id: ids::EDITIONUID,
                    val: ElementType::UInt(uid),
                    ..
                } => {
                    chapteredition.uid = Some(uid);
                }
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

impl Parseable for ChapterEdition {
    type Output = Vec<ChapterEdition>;

    const ID: u32 = ids::CHAPTERS;

    fn parse<R: io::Read>(r: &mut R, size: u64) -> Result<Vec<ChapterEdition>> {
        Element::parse_master(r, size, Some(ids::EDITIONENTRY)).map(|elements| {
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
}

/// An individual chapter point
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Chapter {
    /// The chapter's UID
    pub uid: u64,
    /// Timestamp of the start of the chapter
    pub time_start: Duration,
    /// Timestamp of the end of the chapter
    pub time_end: Option<Duration>,
    /// Whether the chapter point should be hidden in the user interface
    pub hidden: bool,
    /// Whether the chapter point should be enabled in the user interface
    pub enabled: bool,
    /// Unique ID of the segment to be played during this chapter
    pub segment_uid: Option<Vec<u8>>,
    /// Unique ID of the edition to play from the linked segment
    pub segment_edition_uid: Option<u64>,
    /// Contains all strings to use for displaying chapter
    pub display: Vec<ChapterDisplay>,
}

impl Chapter {
    fn new() -> Chapter {
        Chapter {
            uid: 0,
            time_start: Duration::default(),
            time_end: None,
            hidden: false,
            enabled: false,
            segment_uid: None,
            segment_edition_uid: None,
            display: Vec::new(),
        }
    }

    fn build(elements: Vec<Element>) -> Chapter {
        let mut chapter = Chapter::new();
        for e in elements {
            match e {
                Element {
                    id: ids::CHAPTERUID,
                    val: ElementType::UInt(uid),
                    ..
                } => {
                    chapter.uid = uid;
                }
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
                    id: ids::CHAPTERSEGMENTUID,
                    val: ElementType::Binary(uid),
                    ..
                } => {
                    chapter.segment_uid = Some(uid);
                }
                Element {
                    id: ids::CHAPTERSEGMENTEDITIONUID,
                    val: ElementType::UInt(uid),
                    ..
                } => {
                    chapter.segment_edition_uid = Some(uid);
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
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ChapterDisplay {
    /// The user interface string
    pub string: String,
    /// The string's language
    pub language: Language,
}

impl ChapterDisplay {
    fn new() -> ChapterDisplay {
        ChapterDisplay {
            string: String::new(),
            language: Language::ISO639(String::new()),
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
                    if !matches!(display.language, Language::IETF(_)) {
                        display.language = Language::ISO639(language);
                    }
                }
                Element {
                    id: ids::CHAPLANGUAGE_IETF,
                    val: ElementType::String(language),
                    ..
                } => {
                    display.language = Language::IETF(language);
                }
                _ => {}
            }
        }
        display
    }
}

/// An attached tag
#[derive(Debug, Clone)]
pub struct Tag {
    /// which elements the metadata's tag applies to
    pub targets: Option<Target>,
    /// general information about the target
    pub simple: Vec<SimpleTag>,
}

impl Tag {
    fn new() -> Tag {
        Tag {
            targets: None,
            simple: Vec::new(),
        }
    }

    fn build_entry(elements: Vec<Element>) -> Tag {
        let mut tag = Tag::new();
        for e in elements {
            match e {
                Element {
                    id: ids::TARGETS,
                    val: ElementType::Master(sub_elements),
                    ..
                } => {
                    tag.targets = Some(Target::build_entry(sub_elements));
                }
                Element {
                    id: ids::SIMPLETAG,
                    val: ElementType::Master(sub_elements),
                    ..
                } => {
                    tag.simple.push(SimpleTag::build_entry(sub_elements));
                }
                _ => {}
            }
        }
        tag
    }
}

impl Parseable for Tag {
    type Output = Vec<Tag>;

    const ID: u32 = ids::TAGS;

    fn parse<R: io::Read>(r: &mut R, size: u64) -> Result<Vec<Tag>> {
        Element::parse_master(r, size, Some(ids::TAG)).map(|elements| {
            elements
                .into_iter()
                .filter_map(|e| match e {
                    Element {
                        id: ids::TAG,
                        val: ElementType::Master(sub_elements),
                        ..
                    } => Some(Tag::build_entry(sub_elements)),
                    _ => None,
                })
                .collect()
        })
    }
}

/// Which elements the metadata's tag applies to
#[derive(Debug, Clone)]
pub struct Target {
    /// Logical level of target
    pub target_type_value: Option<TargetTypeValue>,
    /// Informational string of target level
    pub target_type: Option<String>,
    /// Unique IDs of track(s) the tag belongs to
    pub track_uids: Vec<u64>,
    /// Unique IDs of edition entry(s) the tag belongs to
    pub edition_uids: Vec<u64>,
    /// Unique IDs of chapter(s) the tag belongs to
    pub chapter_uids: Vec<u64>,
    /// Unique IDs of attachment(s) the tag belongs to
    pub attachment_uids: Vec<u64>,
}

/// The type of value the tag is for
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub enum TargetTypeValue {
    /// collection
    Collection,
    /// edition / issue / volume / opus / season / sequel
    Season,
    /// album / opera / concert / movie / episode / concert
    Episode,
    /// part / session
    Part,
    /// track / song / chapter
    Chapter,
    /// subtrack / part / movement / scene
    Scene,
    /// shot
    Shot,
    /// none of the define value types
    Unknown,
}

impl TargetTypeValue {
    /// Returns type value as static string
    pub fn as_str(&self) -> &'static str {
        match self {
            TargetTypeValue::Collection => "collection",
            TargetTypeValue::Season => "season",
            TargetTypeValue::Episode => "episode",
            TargetTypeValue::Part => "part",
            TargetTypeValue::Chapter => "chapter",
            TargetTypeValue::Scene => "scene",
            TargetTypeValue::Shot => "shot",
            TargetTypeValue::Unknown => "unknown",
        }
    }
}

impl std::fmt::Display for TargetTypeValue {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "{}", self.as_str())
    }
}

impl From<u64> for TargetTypeValue {
    fn from(val: u64) -> Self {
        match val {
            70 => TargetTypeValue::Collection,
            60 => TargetTypeValue::Season,
            50 => TargetTypeValue::Episode,
            40 => TargetTypeValue::Part,
            30 => TargetTypeValue::Chapter,
            20 => TargetTypeValue::Scene,
            10 => TargetTypeValue::Shot,
            _ => TargetTypeValue::Unknown,
        }
    }
}

impl Target {
    fn new() -> Target {
        Target {
            target_type_value: None,
            target_type: None,
            track_uids: Vec::new(),
            edition_uids: Vec::new(),
            chapter_uids: Vec::new(),
            attachment_uids: Vec::new(),
        }
    }

    fn build_entry(elements: Vec<Element>) -> Target {
        let mut target = Target::new();
        for e in elements {
            match e {
                Element {
                    id: ids::TARGETTYPEVALUE,
                    val: ElementType::UInt(number),
                    ..
                } => {
                    target.target_type_value = Some(number.into());
                }
                Element {
                    id: ids::TARGETTYPE,
                    val: ElementType::String(string),
                    ..
                } => {
                    target.target_type = Some(string);
                }
                Element {
                    id: ids::TAG_TRACK_UID,
                    val: ElementType::UInt(number),
                    ..
                } => {
                    target.track_uids.push(number);
                }
                Element {
                    id: ids::TAG_EDITION_UID,
                    val: ElementType::UInt(number),
                    ..
                } => {
                    target.edition_uids.push(number);
                }
                Element {
                    id: ids::TAG_CHAPTER_UID,
                    val: ElementType::UInt(number),
                    ..
                } => {
                    target.chapter_uids.push(number);
                }
                Element {
                    id: ids::TAG_ATTACHMENT_UID,
                    val: ElementType::UInt(number),
                    ..
                } => {
                    target.attachment_uids.push(number);
                }
                _ => {}
            }
        }
        target
    }
}

/// General information about the target
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SimpleTag {
    /// The tag's name
    pub name: String,
    /// The tag's language
    pub language: Option<Language>,
    /// Whether this is the default/original language to use
    pub default: bool,
    /// The tag's value
    pub value: Option<TagValue>,
}

impl SimpleTag {
    fn new() -> SimpleTag {
        SimpleTag {
            name: String::new(),
            language: None,
            default: false,
            value: None,
        }
    }

    fn build_entry(elements: Vec<Element>) -> SimpleTag {
        let mut tag = SimpleTag::new();
        for e in elements {
            match e {
                Element {
                    id: ids::TAGNAME,
                    val: ElementType::UTF8(string),
                    ..
                } => {
                    tag.name = string;
                }
                Element {
                    id: ids::TAGLANGUAGE,
                    val: ElementType::String(string),
                    ..
                } => {
                    if !matches!(tag.language, Some(Language::IETF(_))) {
                        tag.language = Some(Language::ISO639(string));
                    }
                }
                Element {
                    id: ids::TAGLANGUAGE_IETF,
                    val: ElementType::String(string),
                    ..
                } => {
                    tag.language = Some(Language::IETF(string));
                }
                Element {
                    id: ids::TAGDEFAULT,
                    val: ElementType::UInt(default),
                    ..
                } => {
                    tag.default = default != 0;
                }
                Element {
                    id: ids::TAGSTRING,
                    val: ElementType::UTF8(string),
                    ..
                } => {
                    tag.value = Some(TagValue::String(string));
                }
                Element {
                    id: ids::TAGBINARY,
                    val: ElementType::Binary(binary),
                    ..
                } => {
                    tag.value = Some(TagValue::Binary(binary));
                }
                _ => {}
            }
        }
        tag
    }
}

/// Which form of language is in use
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Language {
    /// Language formatted as ISO-639
    ISO639(String),
    /// Lanuage formatted as IETF
    IETF(String),
}

/// A tag's value
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TagValue {
    /// Tag's value as string
    String(String),
    /// Tag's value as binary
    Binary(Vec<u8>),
}

/// Returns a single item from open Matroska file such as `Info`
pub fn get<R, P>(mut file: R) -> Result<Option<P::Output>>
where
    R: io::Read + io::Seek,
    P: Parseable,
{
    use std::io::SeekFrom;

    let (mut id_0, mut size_0, _) = ebml::read_element_id_size(&mut file)?;
    while id_0 != ids::SEGMENT {
        file.seek(SeekFrom::Current(size_0 as i64)).map(|_| ())?;
        let (id, size, _) = ebml::read_element_id_size(&mut file)?;
        id_0 = id;
        size_0 = size;
    }

    let segment_start = file.stream_position()?;

    while size_0 > 0 {
        let (id_1, size_1, len) = ebml::read_element_id_size(&mut file)?;
        match id_1 {
            ids::SEEKHEAD => {
                // if seektable encountered, find part from that
                let seektable = Seektable::parse(&mut file, segment_start, size_1)?;

                if let Some(pos) = seektable.get(P::ID)? {
                    file.seek(SeekFrom::Start(pos))?;
                    let (i, s, _) = ebml::read_element_id_size(&mut file)?;
                    assert_eq!(i, P::ID);
                    return P::parse(&mut file, s).map(Some);
                }
            }
            // if no seektable, try to find part separately
            id if id == P::ID => {
                return P::parse(&mut file, size_1).map(Some);
            }
            _ => {
                file.seek(SeekFrom::Current(size_1 as i64)).map(|_| ())?;
            }
        }
        size_0 -= len;
        size_0 -= size_1;
    }

    Ok(None)
}

/// Returns a single item from Matroska file on disk, such as `Info`
pub fn get_from<P, R>(path: P) -> Result<Option<R::Output>>
where
    P: AsRef<std::path::Path>,
    R: Parseable,
{
    std::fs::File::open(path)
        .map(std::io::BufReader::new)
        .map_err(MatroskaError::Io)
        .and_then(get::<_, R>)
}

/// Opens Matroska file on disk
pub fn open<P: AsRef<std::path::Path>>(path: P) -> Result<Matroska> {
    std::fs::File::open(path)
        .map(std::io::BufReader::new)
        .map_err(MatroskaError::Io)
        .and_then(Matroska::open)
}
