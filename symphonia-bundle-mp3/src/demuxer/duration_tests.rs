use super::*;
use std::io::Cursor;

fn frame(word: u32) -> Vec<u8> {
    let header = header::parse_frame_header(word).unwrap();
    let mut bytes = word.to_be_bytes().to_vec();
    bytes.resize(MPEG_HEADER_LEN + header.frame_size, 0);
    bytes
}
fn stream(first: u32, later: u32, count: usize) -> Vec<u8> {
    (0..count).flat_map(|i| frame(if i < 17 { first } else { later })).collect()
}
fn reader(bytes: Vec<u8>, gapless: bool) -> MpaReader {
    MpaReader::try_new(
        MediaSourceStream::new(Box::new(Cursor::new(bytes)), Default::default()),
        &FormatOptions { enable_gapless: gapless, ..Default::default() },
    )
    .unwrap()
}
fn packets(reader: &mut MpaReader) -> Vec<Packet> {
    let mut packets = vec![];
    while let Ok(packet) = reader.next_packet() {
        packets.push(packet);
    }
    packets
}
fn total(packets: &[Packet]) -> u64 {
    packets.iter().map(|p| p.dur).sum()
}
fn tag(kind: &[u8; 4], count: Option<u32>, lame: bool) -> Vec<u8> {
    let mut tag = frame(0xfffb9000);
    tag[36..40].copy_from_slice(kind);
    if kind == b"VBRI" {
        tag[40..42].copy_from_slice(&1u16.to_be_bytes());
        tag[50..54].copy_from_slice(&count.unwrap().to_be_bytes());
    } else {
        tag[40..44].copy_from_slice(&u32::from(count.is_some()).to_be_bytes());
        let at = if let Some(n) = count {
            tag[44..48].copy_from_slice(&n.to_be_bytes());
            48
        } else {
            44
        };
        if lame {
            tag[at..at + 9].copy_from_slice(b"Lavc-test");
            let trim = ((576u32 << 12) | 800).to_be_bytes();
            tag[at + 21..at + 24].copy_from_slice(&trim[1..]);
        }
    }
    tag.extend(stream(0xfffb9000, 0xfffb9000, 20));
    tag
}
#[test]
fn estimated_underflow_never_trims_complete_mpeg1_2_or_25_packets() {
    for (high, low) in
        [(0xfffb9000, 0xfffb5000), (0xfff38000, 0xfff34000), (0xffe38000, 0xffe34000)]
    {
        let duration = header::parse_frame_header(high).unwrap().duration();
        let mut r = reader(stream(high, low, 40), true);
        assert!(r.tracks()[0].codec_params.n_frames.unwrap() < 40 * duration);
        let p = packets(&mut r);
        assert_eq!(p.len(), 40);
        assert_eq!(total(&p), 40 * duration);
        assert!(p.iter().all(|p| p.trim_end == 0 && p.trim_start == 0));
    }
}
#[test]
fn exact_and_overestimated_untagged_counts_keep_all_packets() {
    for (a, b) in [(0xfffb9000, 0xfffb9000), (0xfffb5000, 0xfffb9000)] {
        let mut r = reader(stream(a, b, 40), true);
        assert!(r.tracks()[0].codec_params.n_frames.unwrap() >= 40 * 1152);
        let p = packets(&mut r);
        assert_eq!(p.len(), 40);
        assert_eq!(total(&p), 40 * 1152);
    }
}
#[test]
fn declared_xing_info_and_vbri_limits_still_trim() {
    for kind in [b"Xing", b"Info", b"VBRI"] {
        let bytes = tag(kind, Some(10), false);
        let mut r = reader(bytes.clone(), true);
        assert_eq!(r.tracks()[0].codec_params.n_frames, Some(10 * 1152));
        let p = packets(&mut r);
        assert_eq!(p.len(), 20);
        assert_eq!(total(&p), 10 * 1152);
        assert!(p[10..].iter().all(|p| p.trim_end == 1152));
        assert_eq!(total(&packets(&mut reader(bytes, false))), 20 * 1152);
    }
}
#[test]
fn declared_encoder_delay_and_padding_are_preserved() {
    let bytes = tag(b"Xing", Some(20), true);
    let mut r = reader(bytes.clone(), true);
    assert_eq!(r.tracks()[0].codec_params.delay, Some(1105));
    assert_eq!(r.tracks()[0].codec_params.padding, Some(271));
    let p = packets(&mut r);
    assert_eq!(p[0].trim_start, 1105);
    assert_eq!(p.last().unwrap().trim_end, 271);
    assert_eq!(total(&p), 20 * 1152 - 1105 - 271);
    assert_eq!(total(&packets(&mut reader(bytes, false))), 20 * 1152);
}
#[test]
fn missing_count_retains_delay_without_inventing_tail_boundary() {
    let mut r = reader(tag(b"Xing", None, true), true);
    assert!(r.tracks()[0].codec_params.n_frames.is_none());
    let p = packets(&mut r);
    assert_eq!(total(&p), 20 * 1152 - 1105);
    assert!(p.iter().all(|p| p.trim_end == 0));
}
#[test]
fn short_or_truncated_streams_do_not_invent_packets() {
    let mut short = reader(stream(0xfffb9000, 0xfffb9000, 10), true);
    assert!(short.tracks()[0].codec_params.n_frames.is_none());
    assert_eq!(total(&packets(&mut short)), 10 * 1152);
    let mut bytes = stream(0xfffb9000, 0xfffb5000, 40);
    bytes.truncate(bytes.len() - 50);
    let mut r = reader(bytes, true);
    let p = packets(&mut r);
    assert_eq!(p.len(), 39);
    assert_eq!(total(&p), 39 * 1152);
}
