// Copyright 2017-2022 Brian Langenberger
// Copyright 2022 Ririsoft <riri@ririsoft.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
use std::{fs::File, path::PathBuf, time::Duration};

use matroska::{Settings, TagValue, Tracktype};

#[test]
fn info() {
    let f = File::open(PathBuf::from("tests").join("samples").join("bbb.mkv")).unwrap();
    let m = matroska::Matroska::open(f).unwrap();
    assert_eq!(m.info.title, Some("Big Buck Bunny".into()));
    assert_eq!(m.info.duration, Some(Duration::from_millis(1015)));

    assert_eq!(m.tracks.len(), 2);

    let video_track = m
        .tracks
        .iter()
        .find(|t| t.tracktype == Tracktype::Video)
        .unwrap();
    match &video_track.settings {
        Settings::Video(settings) => {
            assert_eq!(settings.pixel_width, 320);
            assert_eq!(settings.pixel_height, 180);
        }
        _ => panic!("unexpected track settings"),
    }

    let audio_track = m
        .tracks
        .iter()
        .find(|t| t.tracktype == Tracktype::Audio)
        .unwrap();
    match &audio_track.settings {
        Settings::Audio(settings) => {
            assert_eq!(settings.channels, 2);
            assert_eq!(settings.sample_rate, 48000.0);
        }
        _ => panic!("unexpected track settings"),
    }

    let year_value = m
        .tags
        .iter()
        .find_map(|t| {
            t.simple.iter().find_map(|t| match t.name.as_str() {
                "DATE" => t.value.clone(),
                _ => None,
            })
        })
        .expect("tag DATE not found");
    match year_value {
        TagValue::String(val) => assert_eq!(val, "2012"),
        _ => panic!("invalid tag value"),
    }
}
