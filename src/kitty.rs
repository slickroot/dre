use crate::canvas::Canvas;
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

    fn value(self) -> u32 {
        self.0.get()
    }
}

pub(crate) fn soft_clear() -> Command {
    Command("\x1b_Ga=d,d=a,q=2;\x1b\\".to_string())
}

pub(crate) fn delete(id: ImageId) -> Command {
    Command(format!("\x1b_Ga=d,d=I,i={},q=2;\x1b\\", id.value()))
}

pub(crate) fn show(canvas: &Canvas, id: ImageId, col: i64, row: i64, z: i32) -> Command {
    Command(format!(
        "\x1b[{};{}H{}",
        row + 1,
        col + 1,
        transmission(&canvas.pixels, canvas.width, canvas.height, id, z)
    ))
}

#[allow(clippy::too_many_arguments)]
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn grow(
    root: &Canvas,
    frames: &[Canvas],
    id: ImageId,
    col: i64,
    row: i64,
    z: i32,
    gap_ms: u32,
    hold_ms: u32,
) -> Command {
    let mut output = show(root, id, col, row, z).0;
    for (index, canvas) in frames.iter().enumerate() {
        // WezTerm ignores `a=a` and loops the frames, so callers hold the last one long.
        let gap = if index == frames.len() - 1 {
            hold_ms
        } else {
            gap_ms
        };
        output.push_str(&frame(canvas, id, gap));
    }
    output.push_str(&escape(&format!("a=a,i={},s=3,v=2,q=2", id.value()), ""));
    Command(output)
}

pub(crate) fn place(id: ImageId, placement: PlacementId, col: i64, row: i64, z: i32) -> Command {
    Command(format!(
        "\x1b[{};{}H\x1b_Ga=p,i={},p={},q=2,z={};\x1b\\",
        row + 1,
        col + 1,
        id.value(),
        placement.value(),
        z
    ))
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

fn transmission(pixels: &[u8], width: i64, height: i64, id: ImageId, z: i32) -> String {
    chunked(
        &format!(
            "a=T,f=32,s={width},v={height},o=z,q=2,i={},z={z}",
            id.value()
        ),
        pixels,
    )
}

#[cfg_attr(not(test), allow(dead_code))]
fn frame(canvas: &Canvas, id: ImageId, gap_ms: u32) -> String {
    // WezTerm reads a frame's gap from `Z` and kitty from `z`; without `Z`, WezTerm
    // uses 40 ms (wezterm-escape-parser/src/apc.rs, KittyImageFrame::from_keys).
    chunked(
        &format!(
            "a=f,f=32,s={},v={},o=z,q=2,i={},z={gap_ms},Z={gap_ms}",
            canvas.width,
            canvas.height,
            id.value()
        ),
        &canvas.pixels,
    )
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
    fn transmission_single_chunk_has_expected_header_and_m0() {
        let pixels = vec![1u8, 2, 3, 4];
        let result = transmission(&pixels, 2, 1, image_id(1), -1);

        let expected_payload = encode(&pixels);
        let expected_header = "a=T,f=32,s=2,v=1,o=z,q=2,i=1,z=-1,m=0".to_string();
        let expected = escape(&expected_header, &expected_payload);
        assert_eq!(result, expected);
        assert!(result.starts_with("\x1b_Ga=T,f=32,s=2,v=1,o=z,q=2,i=1,z=-1,m=0;"));
        assert!(result.ends_with("\x1b\\"));
    }

    #[test]
    fn transmission_multi_chunk_splits_and_sets_more_flag() {
        let mut state: u32 = 0x9E3779B9;
        let pixels: Vec<u8> = (0..8_000u32)
            .map(|_| {
                state = state.wrapping_mul(1664525).wrapping_add(1013904223);
                (state >> 16) as u8
            })
            .collect();
        let result = transmission(&pixels, 100, 100, image_id(1), -1);

        let payload = encode(&pixels);
        let chunk_list = chunks(&payload, CHUNK_SIZE);
        assert!(
            chunk_list.len() > 1,
            "expected payload to span multiple chunks"
        );

        let header = format!(
            "a=T,f=32,s=100,v=100,o=z,q=2,i=1,z=-1,m={}",
            more(&chunk_list, 0)
        );
        let mut expected = escape(&header, &chunk_list[0]);
        for (index, chunk) in chunk_list.iter().enumerate().skip(1) {
            let keys = format!("m={}", more(&chunk_list, index));
            expected.push_str(&escape(&keys, chunk));
        }

        assert_eq!(result, expected);
        assert!(result.contains(",m=1;"));
        assert!(result.ends_with("\x1b\\"));
        assert!(result.matches("\x1b_G").count() > 1);
    }

    #[test]
    fn soft_clear_removes_placements_but_keeps_image_data() {
        assert_eq!(soft_clear().to_string(), "\x1b_Ga=d,d=a,q=2;\x1b\\");
    }

    #[test]
    fn delete_frees_a_named_image() {
        assert_eq!(
            delete(image_id(7)).to_string(),
            "\x1b_Ga=d,d=I,i=7,q=2;\x1b\\"
        );
    }

    #[test]
    fn show_moves_the_cursor_to_the_sprite_cell_then_transmits_its_pixels() {
        let canvas = Canvas {
            pixels: vec![1, 2, 3, 4, 5, 6, 7, 8],
            width: 2,
            height: 1,
        };
        assert_eq!(
            show(&canvas, image_id(1), 3, 5, -1).to_string(),
            format!(
                "\x1b[6;4H{}",
                transmission(&canvas.pixels, canvas.width, canvas.height, image_id(1), -1)
            )
        );
    }

    #[test]
    fn place_positions_the_cursor_and_names_the_image_without_payload() {
        assert_eq!(
            place(image_id(7), placement_id(2), 3, 5, -1).to_string(),
            "\x1b[6;4H\x1b_Ga=p,i=7,p=2,q=2,z=-1;\x1b\\"
        );
    }

    #[test]
    fn place_names_the_requested_placement() {
        assert!(place(image_id(7), placement_id(9), 0, 0, 0)
            .to_string()
            .contains(",p=9,"));
    }

    fn grow_canvas(fill: u8) -> Canvas {
        Canvas {
            pixels: vec![fill; 16],
            width: 2,
            height: 2,
        }
    }

    fn grown(frames: &[Canvas], gap_ms: u32, hold_ms: u32) -> String {
        grow(
            &grow_canvas(0),
            frames,
            image_id(7),
            3,
            5,
            -1,
            gap_ms,
            hold_ms,
        )
        .to_string()
    }

    fn frame_escapes(output: &str) -> Vec<&str> {
        output
            .split("\x1b_G")
            .filter(|escape| escape.starts_with("a=f,"))
            .collect()
    }

    #[test]
    fn grow_moves_the_cursor_then_transmits_the_root() {
        let root = grow_canvas(0);
        let output = grown(&[grow_canvas(1)], 10, 1000);
        assert!(output.starts_with(&format!(
            "\x1b[6;4H{}",
            transmission(&root.pixels, root.width, root.height, image_id(7), -1)
        )));
        assert_eq!(output.matches("a=T,").count(), 1);
    }

    #[test]
    fn grow_sends_one_frame_per_canvas() {
        let frames = vec![grow_canvas(1), grow_canvas(2), grow_canvas(3)];
        assert_eq!(frame_escapes(&grown(&frames, 10, 1000)).len(), frames.len());
    }

    #[test]
    fn grow_frames_are_rgba_composited_over_the_root() {
        let output = grown(&[grow_canvas(1), grow_canvas(2)], 10, 1000);
        for frame in frame_escapes(&output) {
            assert!(frame.contains(",f=32,"));
            assert!(frame.contains(",o=z,"));
            assert!(frame.contains(",i=7,"));
        }
    }

    #[test]
    fn grow_writes_z_and_capital_z_with_the_same_gap_and_holds_the_last_frame() {
        let gap_ms = 17;
        let hold_ms = 2_000_000_000;
        let output = grown(
            &[grow_canvas(1), grow_canvas(2), grow_canvas(3)],
            gap_ms,
            hold_ms,
        );
        let frames = frame_escapes(&output);
        let (last, rest) = frames.split_last().unwrap();
        for frame in rest {
            assert!(frame.contains(&format!(",z={gap_ms},Z={gap_ms},")));
        }
        assert!(last.contains(&format!(",z={hold_ms},Z={hold_ms},")));
    }

    #[test]
    fn grow_frames_are_chunked_like_transmission() {
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
        let output = grown(&[frame], 10, 1000);
        let first = frame_escapes(&output)[0];
        assert!(first.contains(",m=1;"));
        assert!(output.contains("\x1b_Gm=0;"));
    }

    #[test]
    fn grow_ends_by_playing_the_frames_once() {
        let output = grown(&[grow_canvas(1)], 10, 1000);
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
