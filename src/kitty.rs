use crate::canvas::Canvas;
use crate::render::virtual_terminal::{Content, Op};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use flate2::write::ZlibEncoder;
use flate2::Compression;
use nix::sys::select::{select, FdSet};
use nix::sys::termios::{cfmakeraw, tcgetattr, tcsetattr, SetArg};
use nix::sys::time::{TimeVal, TimeValLike};
use nix::unistd::read;
use std::fmt;
use std::io::{self, Write};
use std::num::NonZeroU32;
use std::os::fd::{BorrowedFd, RawFd};

pub(crate) struct Command(String);

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ImageId(NonZeroU32);

impl ImageId {
    pub(crate) fn new(value: NonZeroU32) -> Self {
        ImageId(value)
    }

    fn value(self) -> u32 {
        self.0.get()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PlacementId(NonZeroU32);

impl PlacementId {
    pub(crate) fn new(value: NonZeroU32) -> Self {
        PlacementId(value)
    }

    pub(crate) fn value(self) -> u32 {
        self.0.get()
    }
}

pub(crate) fn delete(id: ImageId) -> Command {
    Command(format!("\x1b_Ga=d,d=I,i={},q=2;\x1b\\", id.value()))
}

pub(crate) fn transmit(canvas: &Canvas, id: ImageId) -> Command {
    Command(transmit_only(
        &canvas.pixels,
        canvas.width,
        canvas.height,
        id,
    ))
}

pub(crate) fn place_ext(
    id: ImageId,
    placement: PlacementId,
    col: i64,
    row: i64,
    z: i32,
    source: Option<(i64, i64, i64, i64)>,
    cells: Option<(i64, i64)>,
) -> Command {
    let mut keys = format!("a=p,i={},p={},q=2,z={}", id.value(), placement.value(), z);
    if let Some((x, y, w, h)) = source {
        keys.push_str(&format!(",x={x},y={y},w={w},h={h}"));
    }
    if let Some((c, r)) = cells {
        keys.push_str(&format!(",c={c},r={r}"));
    }
    Command(format!(
        "\x1b[{};{}H\x1b_G{};\x1b\\",
        row + 1,
        col + 1,
        keys
    ))
}

pub(crate) fn delete_placement(image: ImageId, placement: PlacementId) -> Command {
    Command(format!(
        "\x1b_Ga=d,d=i,i={},p={},q=2;\x1b\\",
        image.value(),
        placement.value()
    ))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn transmit_animation(
    root: &Canvas,
    frames: &[Canvas],
    id: ImageId,
    gap_ms: u32,
    hold_ms: u32,
    wezterm: bool,
) -> Command {
    let mut output = transmit_only(&root.pixels, root.width, root.height, id);
    for (index, canvas) in frames.iter().enumerate() {
        let gap = if index == frames.len() - 1 {
            hold_ms
        } else {
            gap_ms
        };
        output.push_str(&frame(canvas, id, gap, wezterm));
    }
    output.push_str(&escape(&format!("a=a,i={},s=3,v=2,q=2", id.value()), ""));
    Command(output)
}

pub(crate) fn encode_ops(ops: &[Op], wezterm: bool) -> Vec<u8> {
    let mut result = Vec::new();
    for op in ops {
        let s = match op {
            Op::Upload {
                image,
                content: Content::Still(canvas),
            } => transmit(canvas, *image).to_string(),
            Op::Upload {
                image,
                content: Content::Animation { root, frames },
            } => transmit_animation(
                root,
                frames,
                *image,
                animation_gap_ms(frames.len()),
                ANIMATION_HOLD_MS,
                wezterm,
            )
            .to_string(),
            Op::Place {
                image,
                placement,
                col,
                row,
                z,
                source,
                cells,
            } => {
                let src = source.as_ref().map(|s| (s.x, s.y, s.width, s.height));
                place_ext(*image, *placement, *col, *row, *z, src, *cells).to_string()
            }
            Op::Delete { image, placement } => delete_placement(*image, *placement).to_string(),
            Op::Free { image } => delete(*image).to_string(),
        };
        result.extend_from_slice(s.as_bytes());
    }
    result
}

const ANIMATION_DURATION_MS: u32 = 150;
// WezTerm ignores `a=a` and loops the frames, so the last frame is held for
// about 23 days instead of relying on `v=2` to stop after one play.
const ANIMATION_HOLD_MS: u32 = 2_000_000_000;

fn animation_gap_ms(frame_count: usize) -> u32 {
    let frame_count = frame_count as u32;
    if frame_count <= 1 {
        return ANIMATION_HOLD_MS;
    }
    (ANIMATION_DURATION_MS + (frame_count - 1) / 2) / (frame_count - 1)
}

pub(crate) fn require<W: Write>(stream: &mut W, stdin_fd: RawFd) -> io::Result<()> {
    let borrowed = unsafe { BorrowedFd::borrow_raw(stdin_fd) };
    let saved = tcgetattr(borrowed).map_err(io::Error::from)?;
    stream.write_all(QUERY.as_bytes())?;
    stream.flush()?;
    let mut raw = saved.clone();
    cfmakeraw(&mut raw);
    tcsetattr(borrowed, SetArg::TCSADRAIN, &raw).map_err(io::Error::from)?;

    let result = (|| -> io::Result<bool> {
        let mut fds = FdSet::new();
        fds.insert(borrowed);
        let mut timeout = TimeVal::microseconds(REPLY_TIMEOUT_MICROS);
        let ready = select(None, Some(&mut fds), None, None, Some(&mut timeout))
            .map_err(io::Error::from)?;
        let mut buffer = [0u8; 32];
        let reply: Vec<u8> = if ready > 0 && fds.contains(borrowed) {
            let count = read(borrowed, &mut buffer).map_err(io::Error::from)?;
            buffer[..count].to_vec()
        } else {
            Vec::new()
        };
        Ok(is_supported(&reply))
    })();

    tcsetattr(borrowed, SetArg::TCSADRAIN, &saved).map_err(io::Error::from)?;
    if result? {
        return Ok(());
    }
    stream.write_all(CLEAR_LINE.as_bytes())?;
    stream.flush()?;
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        NOT_SUPPORTED_MESSAGE,
    ))
}

const CHUNK_SIZE: usize = 4096;
const QUERY: &str = "\x1b_Gi=1,a=q;\x1b\\";
const CLEAR_LINE: &str = "\r\x1b[K";
const NOT_SUPPORTED_MESSAGE: &str = "Dre requires a terminal with Kitty graphics protocol support.";
const REPLY_TIMEOUT_MICROS: i64 = 500_000;

fn is_supported(reply: &[u8]) -> bool {
    reply.windows(3).any(|window| window == b"i=1")
}

fn zlib(pixels: &[u8]) -> Vec<u8> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(pixels)
        .and_then(|_| encoder.finish())
        .expect("writes to an in-memory Vec<u8> can't fail")
}

fn base64(bytes: Vec<u8>) -> String {
    STANDARD.encode(bytes)
}

fn encode(pixels: &[u8]) -> String {
    base64(zlib(pixels))
}

fn chunks(payload: &str, chunk_size: usize) -> Vec<String> {
    payload
        .as_bytes()
        .chunks(chunk_size)
        .map(|chunk| {
            std::str::from_utf8(chunk)
                .expect(
                    "base64 payload is single-byte ASCII, so byte chunks are always valid UTF-8",
                )
                .to_owned()
        })
        .collect()
}

fn more(chunks: &[String], index: usize) -> i32 {
    if index == chunks.len() - 1 {
        0
    } else {
        1
    }
}

fn escape(keys: &str, payload: &str) -> String {
    format!("\x1b_G{keys};{payload}\x1b\\")
}

fn transmit_only(pixels: &[u8], width: i64, height: i64, id: ImageId) -> String {
    chunked(
        &format!("a=t,f=32,s={width},v={height},o=z,q=2,i={}", id.value()),
        pixels,
    )
}

fn frame(canvas: &Canvas, id: ImageId, gap_ms: u32, wezterm: bool) -> String {
    // WezTerm reads a frame's gap from `Z` and kitty from `z`; without `Z`, WezTerm
    // uses 40 ms (wezterm-escape-parser/src/apc.rs, KittyImageFrame::from_keys).
    // Kitty rejects the unknown `Z` key and drops the whole command
    // (kitty/parse-graphics-command.h), so `Z` goes to WezTerm only.
    let mut keys = format!(
        "a=f,f=32,s={},v={},o=z,q=2,i={},z={gap_ms}",
        canvas.width,
        canvas.height,
        id.value()
    );
    if wezterm {
        keys.push_str(&format!(",Z={gap_ms}"));
    }
    chunked(&keys, &canvas.pixels)
}

fn chunked(keys: &str, pixels: &[u8]) -> String {
    let payload = encode(pixels);
    let chunk_list = chunks(&payload, CHUNK_SIZE);
    let header = format!("{keys},m={}", more(&chunk_list, 0));
    let mut escapes = vec![escape(&header, &chunk_list[0])];
    for (index, chunk) in chunk_list.iter().enumerate().skip(1) {
        let keys = format!("m={}", more(&chunk_list, index));
        escapes.push(escape(&keys, chunk));
    }
    escapes.join("")
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::read::ZlibDecoder;
    use std::io::Read as _;

    fn image_id(value: u32) -> ImageId {
        ImageId::new(NonZeroU32::new(value).unwrap())
    }

    fn placement_id(value: u32) -> PlacementId {
        PlacementId::new(NonZeroU32::new(value).unwrap())
    }

    struct Solid(crate::canvas::Rgba);
    impl crate::canvas::Shape for Solid {
        fn colour_at(&self, _x: i64, _y: i64) -> Option<crate::canvas::Rgba> {
            Some(self.0)
        }
    }

    fn solid_canvas(colour: u8) -> Canvas {
        Canvas::fill(1, 1, &Solid([colour, colour, colour, 255]))
    }

    use crate::render::virtual_terminal::SourceRect;

    #[test]
    fn encode_upload_still_transmits_without_displaying() {
        let canvas = solid_canvas(128);
        let ops = vec![Op::Upload {
            image: image_id(3),
            content: Content::Still(canvas),
        }];
        let bytes = encode_ops(&ops, false);
        let s = String::from_utf8_lossy(&bytes);
        assert!(s.contains("a=t,"), "expected a=t in: {s}");
        assert!(!s.contains("a=T,"), "unexpected a=T in: {s}");
    }

    #[test]
    fn encode_upload_animation_transmits_root_and_frames() {
        let root = solid_canvas(10);
        let frames = vec![solid_canvas(20), solid_canvas(30)];
        let ops = vec![Op::Upload {
            image: image_id(4),
            content: Content::Animation { root, frames },
        }];
        let bytes = encode_ops(&ops, false);
        let s = String::from_utf8_lossy(&bytes);
        assert!(s.contains("a=t,"), "expected a=t for root in: {s}");
        assert!(s.contains("a=f,"), "expected a=f frame in: {s}");
    }

    #[test]
    fn encode_upload_animation_in_wezterm_sends_capital_z() {
        let root = solid_canvas(10);
        let frames = vec![solid_canvas(20)];
        let ops = vec![Op::Upload {
            image: image_id(4),
            content: Content::Animation { root, frames },
        }];
        let bytes = encode_ops(&ops, true);
        let s = String::from_utf8_lossy(&bytes);
        assert!(s.contains(",Z="), "expected capital Z in: {s}");
    }

    #[test]
    fn encode_place_positions_the_cursor_and_names_the_image() {
        let ops = vec![Op::Place {
            image: image_id(7),
            placement: placement_id(2),
            col: 3,
            row: 5,
            z: -1,
            source: None,
            cells: None,
        }];
        let bytes = encode_ops(&ops, false);
        let s = String::from_utf8_lossy(&bytes);
        assert!(s.contains("a=p,"), "expected a=p in: {s}");
        assert!(s.contains("i=7,"), "expected i=7 in: {s}");
        assert!(s.contains("p=2,"), "expected p=2 in: {s}");
    }

    #[test]
    fn encode_place_with_source_emits_x_y_w_h() {
        let ops = vec![Op::Place {
            image: image_id(1),
            placement: placement_id(1),
            col: 0,
            row: 0,
            z: 0,
            source: Some(SourceRect {
                x: 1,
                y: 2,
                width: 3,
                height: 4,
            }),
            cells: None,
        }];
        let bytes = encode_ops(&ops, false);
        let s = String::from_utf8_lossy(&bytes);
        assert!(s.contains("x=1,"), "expected x=1 in: {s}");
        assert!(s.contains("y=2,"), "expected y=2 in: {s}");
        assert!(s.contains("w=3,"), "expected w=3 in: {s}");
        assert!(s.contains("h=4"), "expected h=4 in: {s}");
    }

    #[test]
    fn encode_place_with_cells_emits_c_r() {
        let ops = vec![Op::Place {
            image: image_id(1),
            placement: placement_id(1),
            col: 0,
            row: 0,
            z: 0,
            source: None,
            cells: Some((5, 3)),
        }];
        let bytes = encode_ops(&ops, false);
        let s = String::from_utf8_lossy(&bytes);
        assert!(s.contains("c=5,"), "expected c=5 in: {s}");
        assert!(s.contains("r=3"), "expected r=3 in: {s}");
    }

    #[test]
    fn encode_delete_emits_d_i_with_placement() {
        let ops = vec![Op::Delete {
            image: image_id(7),
            placement: placement_id(2),
        }];
        let bytes = encode_ops(&ops, false);
        let s = String::from_utf8_lossy(&bytes);
        assert!(s.contains("a=d,"), "expected a=d in: {s}");
        assert!(s.contains("d=i,"), "expected d=i in: {s}");
        assert!(s.contains("i=7,"), "expected i=7 in: {s}");
        assert!(s.contains("p=2,"), "expected p=2 in: {s}");
    }

    #[test]
    fn encode_free_emits_d_capital_i() {
        let ops = vec![Op::Free { image: image_id(5) }];
        let bytes = encode_ops(&ops, false);
        let s = String::from_utf8_lossy(&bytes);
        assert!(s.contains("a=d,"), "expected a=d in: {s}");
        assert!(s.contains("d=I,"), "expected d=I in: {s}");
        assert!(s.contains("i=5,"), "expected i=5 in: {s}");
    }

    #[test]
    fn chunks_splits_at_boundaries() {
        let payload = "a".repeat(10);
        let result = chunks(&payload, 3);
        assert_eq!(result, vec!["aaa", "aaa", "aaa", "a"]);
    }

    #[test]
    fn chunks_exact_multiple_of_chunk_size() {
        let payload = "a".repeat(6);
        let result = chunks(&payload, 3);
        assert_eq!(result, vec!["aaa", "aaa"]);
    }

    #[test]
    fn chunks_single_chunk_when_smaller_than_size() {
        let payload = "abc";
        let result = chunks(payload, 100);
        assert_eq!(result, vec!["abc"]);
    }

    #[test]
    fn more_returns_zero_for_last_index() {
        let list = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        assert_eq!(more(&list, 2), 0);
    }

    #[test]
    fn more_returns_one_for_non_last_index() {
        let list = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        assert_eq!(more(&list, 0), 1);
        assert_eq!(more(&list, 1), 1);
    }

    #[test]
    fn more_single_element_list_is_last() {
        let list = vec!["a".to_string()];
        assert_eq!(more(&list, 0), 0);
    }

    #[test]
    fn escape_wraps_keys_and_payload() {
        let result = escape("a=T,f=32", "PAYLOAD");
        assert_eq!(result, "\x1b_Ga=T,f=32;PAYLOAD\x1b\\");
    }

    #[test]
    fn encode_round_trips_through_zlib_and_base64() {
        let pixels: Vec<u8> = (0..=255).collect();
        let encoded = encode(&pixels);
        let compressed = STANDARD.decode(&encoded).unwrap();
        let mut decoder = ZlibDecoder::new(&compressed[..]);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed).unwrap();
        assert_eq!(decompressed, pixels);
    }

    #[test]
    fn delete_frees_a_named_image() {
        assert_eq!(
            delete(image_id(7)).to_string(),
            "\x1b_Ga=d,d=I,i=7,q=2;\x1b\\"
        );
    }

    #[test]
    fn animation_gap_is_spread_evenly_across_the_frames() {
        assert_eq!(animation_gap_ms(9), 19);
    }

    #[test]
    fn a_single_frame_animation_holds_immediately() {
        assert_eq!(animation_gap_ms(1), ANIMATION_HOLD_MS);
    }

    fn animation_canvas(fill: u8) -> Canvas {
        Canvas {
            pixels: vec![fill; 16],
            width: 2,
            height: 2,
        }
    }

    fn animated(frames: &[Canvas], gap_ms: u32, hold_ms: u32) -> String {
        animated_in(frames, gap_ms, hold_ms, false)
    }

    fn animated_in(frames: &[Canvas], gap_ms: u32, hold_ms: u32, wezterm: bool) -> String {
        transmit_animation(
            &animation_canvas(0),
            frames,
            image_id(7),
            gap_ms,
            hold_ms,
            wezterm,
        )
        .to_string()
    }

    fn frame_key<'a>(frame: &'a str, key: &str) -> Option<&'a str> {
        let (keys, _) = frame.split_once(';').unwrap();
        keys.split(',')
            .find_map(|pair| pair.strip_prefix(key)?.strip_prefix('='))
    }

    fn frame_escapes(output: &str) -> Vec<&str> {
        output
            .split("\x1b_G")
            .filter(|escape| escape.starts_with("a=f,"))
            .collect()
    }

    #[test]
    fn animation_sends_one_frame_per_canvas() {
        let frames = vec![
            animation_canvas(1),
            animation_canvas(2),
            animation_canvas(3),
        ];
        assert_eq!(
            frame_escapes(&animated(&frames, 10, 1000)).len(),
            frames.len()
        );
    }

    #[test]
    fn animation_frames_are_rgba_composited_over_the_root() {
        let output = animated(&[animation_canvas(1), animation_canvas(2)], 10, 1000);
        for frame in frame_escapes(&output) {
            assert!(frame.contains(",f=32,"));
            assert!(frame.contains(",o=z,"));
            assert!(frame.contains(",i=7,"));
        }
    }

    #[test]
    fn animation_writes_the_gap_in_z_and_holds_the_last_frame() {
        let gap_ms = 17;
        let hold_ms = 2_000_000_000;
        let output = animated(
            &[
                animation_canvas(1),
                animation_canvas(2),
                animation_canvas(3),
            ],
            gap_ms,
            hold_ms,
        );
        let frames = frame_escapes(&output);
        let (last, rest) = frames.split_last().unwrap();
        for frame in rest {
            assert_eq!(frame_key(frame, "z"), Some(gap_ms.to_string().as_str()));
        }
        assert_eq!(frame_key(last, "z"), Some(hold_ms.to_string().as_str()));
    }

    #[test]
    fn animation_frames_have_no_capital_z_outside_wezterm() {
        let output = animated(&[animation_canvas(1), animation_canvas(2)], 17, 1000);
        for frame in frame_escapes(&output) {
            assert_eq!(frame_key(frame, "Z"), None);
        }
    }

    #[test]
    fn animation_frames_in_wezterm_have_capital_z_equal_to_z() {
        let output = animated_in(&[animation_canvas(1), animation_canvas(2)], 17, 1000, true);
        let frames = frame_escapes(&output);
        assert!(!frames.is_empty());
        for frame in frames {
            assert!(frame_key(frame, "Z").is_some());
            assert_eq!(frame_key(frame, "Z"), frame_key(frame, "z"));
        }
    }

    #[test]
    fn animation_frames_are_chunked_like_transmission() {
        let mut state: u32 = 0x9E3779B9;
        let pixels: Vec<u8> = (0..40_000u32)
            .map(|_| {
                state = state.wrapping_mul(1664525).wrapping_add(1013904223);
                (state >> 16) as u8
            })
            .collect();
        let frame = Canvas {
            pixels,
            width: 100,
            height: 100,
        };
        let output = animated(&[frame], 10, 1000);
        let first = frame_escapes(&output)[0];
        assert!(first.contains(",m=1;"));
        assert!(output.contains("\x1b_Gm=0;"));
    }

    #[test]
    fn animation_ends_by_playing_the_frames_once() {
        let output = animated(&[animation_canvas(1)], 10, 1000);
        let last = output.rsplit("\x1b_G").next().unwrap();
        assert!(last.starts_with("a=a,i=7,s=3,v=2"));
        assert!(output.ends_with("\x1b\\"));
    }

    #[test]
    fn a_reply_containing_i_1_means_supported() {
        assert!(is_supported(b"\x1b_Gi=1;OK\x1b\\"));
    }

    #[test]
    fn an_empty_reply_means_not_supported() {
        assert!(!is_supported(b""));
    }

    #[test]
    fn an_unrelated_reply_means_not_supported() {
        assert!(!is_supported(b"\x1b[?62;c"));
    }
}
