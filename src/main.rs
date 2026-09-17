use base64::{engine::general_purpose::STANDARD, Engine as _};
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::io::Write;
use std::process::ExitCode;

mod command_mode;
mod dre_format;
mod insert_mode;
mod layout;
mod render;
mod save_prompt_mode;
mod state;
mod writer;

const CHUNK_SIZE: usize = 4096;
const DELETE_ALL: &str = "\x1b_Ga=d,d=A,q=2;\x1b\\";

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
                .expect("base64 payload is single-byte ASCII, so byte chunks are always valid UTF-8")
                .to_owned()
        })
        .collect()
}

fn more(chunks: &[String], index: usize) -> i32 {
    if index == chunks.len() - 1 { 0 } else { 1 }
}

fn escape(keys: &str, payload: &str) -> String {
    format!("\x1b_G{keys};{payload}\x1b\\")
}

fn transmission(pixels: &[u8], width: i64, height: i64) -> String {
    let payload = encode(pixels);
    let chunk_list = chunks(&payload, CHUNK_SIZE);
    let header = format!(
        "a=T,f=32,s={width},v={height},o=z,q=2,z=-1,m={}",
        more(&chunk_list, 0)
    );
    let mut escapes = vec![escape(&header, &chunk_list[0])];
    for (index, chunk) in chunk_list.iter().enumerate().skip(1) {
        let keys = format!("m={}", more(&chunk_list, index));
        escapes.push(escape(&keys, chunk));
    }
    escapes.join("")
}

struct KittyGraphics;

impl KittyGraphics {
    fn new() -> Self {
        KittyGraphics
    }

    fn draw(&self, sprites: Vec<render::Sprite>) -> String {
        let mut out = DELETE_ALL.to_string();
        for sprite in sprites {
            out.push_str(&format!("\x1b[{};{}H", sprite.row + 1, sprite.col + 1));
            out.push_str(&transmission(&sprite.pixels, sprite.width, sprite.height));
        }
        out
    }
}

fn main() -> ExitCode {
    match writer::write() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::read::ZlibDecoder;
    use std::io::Read as _;

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
        let result = transmission(&pixels, 2, 1);

        let expected_payload = encode(&pixels);
        let expected_header = "a=T,f=32,s=2,v=1,o=z,q=2,z=-1,m=0".to_string();
        let expected = escape(&expected_header, &expected_payload);
        assert_eq!(result, expected);
        assert!(result.starts_with("\x1b_Ga=T,f=32,s=2,v=1,o=z,q=2,z=-1,m=0;"));
        assert!(result.ends_with("\x1b\\"));
    }

    #[test]
    fn transmission_multi_chunk_splits_and_sets_more_flag() {
        let mut state: u32 = 0x9E3779B9;
        let pixels: Vec<u8> = (0..200_000u32)
            .map(|_| {
                state = state.wrapping_mul(1664525).wrapping_add(1013904223);
                (state >> 16) as u8
            })
            .collect();
        let result = transmission(&pixels, 100, 100);

        let payload = encode(&pixels);
        let chunk_list = chunks(&payload, CHUNK_SIZE);
        assert!(chunk_list.len() > 1, "expected payload to span multiple chunks");

        let header = format!(
            "a=T,f=32,s=100,v=100,o=z,q=2,z=-1,m={}",
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
}
