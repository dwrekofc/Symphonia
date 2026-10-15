// Symphonia
// Copyright (c) 2026 dwrekofc.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. See https://mozilla.org/MPL/2.0/.
use super::*;

fn tag(version: u8, flags: u8, body: &[u8]) -> Vec<u8> {
    assert!(body.len() < 128);
    let mut bytes = vec![b'I', b'D', b'3', version, 0, flags, 0, 0, 0, body.len() as u8];
    bytes.extend_from_slice(body);
    bytes.extend_from_slice(b"NEXT");
    bytes
}
#[test]
fn empty_declared_body_is_a_noop_and_leaves_following_bytes_exact() {
    // Compatibility tolerance: the ID3 standard requires a frame, but empty
    // headers occur in files accepted by newer Symphonia and FFmpeg readers.
    for version in [2, 3, 4] {
        for flags in [0, 0x80] {
            let bytes = tag(version, flags, &[]);
            let mut reader = BufReader::new(&bytes);
            let mut metadata = MetadataBuilder::new();
            read_id3v2(&mut reader, &mut metadata).unwrap();
            assert!(metadata.metadata().tags().is_empty());
            assert_eq!(reader.pos(), 10);
            assert_eq!(reader.read_be_u32().unwrap(), u32::from_be_bytes(*b"NEXT"));
        }
    }
}
#[test]
fn malformed_nonempty_or_extended_headers_keep_their_errors_and_bounds() {
    for (version, flags, body) in [
        (3, 0, &b"TIT"[..]),
        (3, 0, &b"TIT2\0\0\0\x14\0\0"[..]),
        (3, 0x40, &b""[..]),
        (4, 0x40, &b""[..]),
        (3, 0x40, &b"\0\0\0\x06\0\0\0\0\0\0"[..]),
        (2, 0x40, &b""[..]),
    ] {
        let bytes = tag(version, flags, body);
        let mut reader = BufReader::new(&bytes);
        assert!(read_id3v2(&mut reader, &mut MetadataBuilder::new()).is_err());
        let end = 10 + body.len() as u64;
        assert!(reader.pos() <= end);
        reader.ignore_bytes(end - reader.pos()).unwrap();
        assert_eq!(reader.read_be_u32().unwrap(), u32::from_be_bytes(*b"NEXT"));
    }
    let bytes = b"ID3\x03\0\0\0";
    assert!(read_id3v2(&mut BufReader::new(bytes), &mut MetadataBuilder::new()).is_err());
}
#[test]
fn normal_title_frame_keeps_its_value_and_stream_boundary() {
    let bytes = tag(3, 0, b"TIT2\0\0\0\x03\0\0\0Hi");
    let mut reader = BufReader::new(&bytes);
    let mut metadata = MetadataBuilder::new();
    read_id3v2(&mut reader, &mut metadata).unwrap();
    let revision = metadata.metadata();
    assert_eq!(revision.tags().len(), 1);
    assert_eq!(revision.tags()[0].value.to_string(), "Hi");
    assert_eq!(reader.read_be_u32().unwrap(), u32::from_be_bytes(*b"NEXT"));
}
