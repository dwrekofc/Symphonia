// Symphonia
// Copyright (c) 2019-2026 The Project Symphonia Developers.
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Read-only scalar inspection over the existing MP4 atom parser.
use crate::atoms::hdlr::HandlerType;
use crate::atoms::{AtomIterator, AtomType, MoovAtom};
use crate::IsoMp4Reader;
use std::io::{Seek, SeekFrom};
use symphonia_core::codecs::CodecParameters;
use symphonia_core::errors::Result;
use symphonia_core::io::{MediaSource, MediaSourceStream};

#[derive(Debug)]
pub struct Inspection {
    pub file_bytes: Option<u64>,
    pub movies: Vec<MovieInfo>,
}
#[derive(Debug)]
pub struct MovieInfo {
    pub timescale: u32,
    pub fragmented: bool,
    pub tracks: Vec<TrackInfo>,
}
#[derive(Debug)]
pub struct EditInfo {
    pub duration: u64,
    pub media_time: i64,
    pub rate_integer: i16,
    pub rate_fraction: i16,
}
#[derive(Debug)]
pub struct TrackInfo {
    pub id: u32,
    pub handler: [u8; 4],
    pub codec_params: CodecParameters,
    pub media_timescale: u32,
    pub media_duration: u64,
    pub edits: Option<Vec<EditInfo>>,
    pub sample_durations: Vec<(u32, u32)>,
    pub sample_count: u32,
}
impl IsoMp4Reader {
    /// Inspect a seekable stream from the beginning without decoding or writing content.
    /// Uses exactly the same atom readers as normal demuxing. This is neutral metadata,
    /// not a declaration that every edit/profile is interpreted by the decoder.
    pub fn inspect(mut source: MediaSourceStream) -> Result<Inspection> {
        source.seek(SeekFrom::Start(0))?;
        let len = source.byte_len();
        let mut it = AtomIterator::new_root(source, len);
        let mut movies = Vec::new();
        while let Some(header) = it.next()? {
            if header.atype != AtomType::Movie {
                continue;
            }
            let moov = it.read_atom::<MoovAtom>()?;
            let fragmented = moov.is_fragmented();
            let mut tracks = Vec::new();
            for trak in &moov.traks {
                let mdia = &trak.mdia;
                let stbl = &mdia.minf.stbl;
                let mut codec_params = CodecParameters::new();
                stbl.stsd.fill_codec_params(&mut codec_params);
                let handler = match mdia.hdlr.handler_type {
                    HandlerType::Sound => *b"soun",
                    HandlerType::Video => *b"vide",
                    HandlerType::Subtitle => *b"subt",
                    HandlerType::Metadata => *b"meta",
                    HandlerType::Text => *b"text",
                    HandlerType::Other(h) => h,
                };
                let edits = trak.edts.as_ref().and_then(|e| e.elst.as_ref()).map(|e| {
                    e.entries
                        .iter()
                        .map(|v| EditInfo {
                            duration: v.segment_duration,
                            media_time: v.media_time,
                            rate_integer: v.media_rate_int,
                            rate_fraction: v.media_rate_frac,
                        })
                        .collect()
                });
                tracks.push(TrackInfo {
                    id: trak.tkhd.id,
                    handler,
                    codec_params,
                    media_timescale: mdia.mdhd.timescale,
                    media_duration: mdia.mdhd.duration,
                    edits,
                    sample_durations: stbl
                        .stts
                        .entries
                        .iter()
                        .map(|v| (v.sample_count, v.sample_delta))
                        .collect(),
                    sample_count: stbl.stsz.sample_count,
                });
            }
            movies.push(MovieInfo { timescale: moov.mvhd.timescale, fragmented, tracks });
        }
        Ok(Inspection { file_bytes: len, movies })
    }
}
