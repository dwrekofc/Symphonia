use super::*;
use std::io::Cursor;

fn frame(word: u32) -> Vec<u8> {
    let header = header::parse_frame_header(word).expect("valid duration test fixture");
    let mut bytes = word.to_be_bytes().to_vec();
    bytes.resize(MPEG_HEADER_LEN + header.frame_size, 0);
    bytes
}
fn stream(first: u32, later: u32, count: usize) -> Vec<u8> {
    (0..count).flat_map(|i| frame(if i < 17 { first } else { later })).collect()
}
fn reader(bytes: Vec<u8>) -> MpaReader<'static> {
    MpaReader::try_new(
        MediaSourceStream::new(Box::new(Cursor::new(bytes)), Default::default()),
        FormatOptions::default(),
    )
    .expect("valid duration test fixture")
}
fn packets(reader: &mut MpaReader<'_>) -> Vec<Packet> {
    let mut packets = vec![];
    while let Ok(Some(packet)) = reader.next_packet() {
        packets.push(packet);
    }
    packets
}
fn total(packets: &[Packet]) -> u64 {
    use symphonia_core::codecs::audio::AudioDecoder;
    let first = &packets[0].data;
    let header =
        header::parse_frame_header(u32::from_be_bytes(first[..4].try_into().expect("header word")))
            .expect("frame header");
    let mut params = AudioCodecParameters::new();
    params
        .for_codec(header.codec())
        .with_sample_rate(header.sample_rate)
        .with_channels(header.channel_mode.channels());
    let mut decoder =
        crate::MpaDecoder::try_new(&params, &Default::default()).expect("MPEG decoder");
    packets
        .iter()
        .map(|packet| decoder.decode(packet).expect("valid zeroed MPEG frame").frames() as u64)
        .sum()
}
fn tag(kind: &[u8; 4], count: Option<u32>, lame: bool) -> Vec<u8> {
    let mut tag = frame(0xfffb9000);
    tag[36..40].copy_from_slice(kind);
    if kind == b"VBRI" {
        tag[40..42].copy_from_slice(&1u16.to_be_bytes());
        tag[50..54].copy_from_slice(&count.expect("valid duration test fixture").to_be_bytes());
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
        let duration =
            header::parse_frame_header(high).expect("valid duration test fixture").duration().get();
        let mut r = reader(stream(high, low, 40));
        assert!(r.tracks()[0].num_frames.expect("valid duration test fixture") < 40 * duration);
        let p = packets(&mut r);
        assert_eq!(p.len(), 40);
        assert_eq!(total(&p), 40 * duration);
        assert!(p.iter().all(|p| p.trim_end.get() == 0 && p.trim_start.get() == 0));
    }
}
#[test]
fn exact_and_overestimated_untagged_counts_keep_all_packets() {
    for (a, b) in [(0xfffb9000, 0xfffb9000), (0xfffb5000, 0xfffb9000)] {
        let mut r = reader(stream(a, b, 40));
        assert!(r.tracks()[0].num_frames.expect("valid duration test fixture") >= 40 * 1152);
        let p = packets(&mut r);
        assert_eq!(p.len(), 40);
        assert_eq!(total(&p), 40 * 1152);
    }
}
#[test]
fn declared_xing_info_and_vbri_limits_still_trim() {
    for kind in [b"Xing", b"Info", b"VBRI"] {
        let bytes = tag(kind, Some(10), false);
        let mut r = reader(bytes.clone());
        assert_eq!(r.tracks()[0].num_frames, Some(10 * 1152));
        let p = packets(&mut r);
        assert_eq!(p.len(), 20);
        assert_eq!(total(&p), 10 * 1152);
        assert!(p[10..].iter().all(|p| p.trim_end.get() >= 1152));
    }
}
#[test]
fn declared_encoder_delay_and_padding_are_preserved() {
    let bytes = tag(b"Xing", Some(20), true);
    let mut r = reader(bytes.clone());
    assert_eq!(r.tracks()[0].delay, Some(1105));
    assert_eq!(r.tracks()[0].padding, Some(271));
    let p = packets(&mut r);
    assert_eq!(p[0].trim_start.get(), 1105);
    assert_eq!(p.last().expect("valid duration test fixture").trim_end.get(), 271);
    assert_eq!(total(&p), 20 * 1152 - 1105 - 271);
}
#[test]
fn missing_count_retains_delay_without_inventing_tail_boundary() {
    let mut r = reader(tag(b"Xing", None, true));
    assert!(r.tracks()[0].num_frames.is_none());
    let p = packets(&mut r);
    assert_eq!(total(&p), 20 * 1152 - 1105);
    assert!(p.iter().all(|p| p.trim_end.get() == 0));
}
#[test]
fn short_or_truncated_streams_do_not_invent_packets() {
    let mut short = reader(stream(0xfffb9000, 0xfffb9000, 10));
    assert!(short.tracks()[0].num_frames.is_none());
    assert_eq!(total(&packets(&mut short)), 10 * 1152);
    let mut bytes = stream(0xfffb9000, 0xfffb5000, 40);
    bytes.truncate(bytes.len() - 50);
    let mut r = reader(bytes);
    let p = packets(&mut r);
    assert_eq!(p.len(), 39);
    assert_eq!(total(&p), 39 * 1152);
}
