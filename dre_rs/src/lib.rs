use pyo3::prelude::*;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::io::Write;

const CHUNK_SIZE: usize = 4096;
const DELETE_ALL: &str = "\x1b_Ga=d,d=A,q=2;\x1b\\";

fn encode(pixels: &[u8]) -> std::io::Result<String> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(pixels)?;
    let compressed = encoder.finish()?;
    Ok(STANDARD.encode(compressed))
}

fn chunks(payload: &str, chunk_size: usize) -> Vec<String> {
    payload
        .as_bytes()
        .chunks(chunk_size)
        .map(|chunk| std::str::from_utf8(chunk).unwrap().to_owned())
        .collect()
}

fn more(chunks: &[String], index: usize) -> i32 {
    if index == chunks.len() - 1 { 0 } else { 1 }
}

fn escape(keys: &str, payload: &str) -> String {
    format!("\x1b_G{keys};{payload}\x1b\\")
}

fn transmission(pixels: &[u8], width: i64, height: i64) -> PyResult<String> {
    let payload = encode(pixels)?;
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
    Ok(escapes.join(""))
}

#[pyclass]
struct KittyGraphics;

#[pymethods]
impl KittyGraphics {
    #[new]
    fn new() -> Self {
        KittyGraphics
    }

    fn draw(&self, sprites: Vec<Bound<'_, PyAny>>) -> PyResult<String> {
        let mut out = DELETE_ALL.to_string();
        for sprite in sprites {
            let row: i64 = sprite.getattr("row")?.extract()?;
            let col: i64 = sprite.getattr("col")?.extract()?;
            let width: i64 = sprite.getattr("width")?.extract()?;
            let height: i64 = sprite.getattr("height")?.extract()?;
            let pixels: Vec<u8> = sprite.getattr("pixels")?.extract()?;
            out.push_str(&format!("\x1b[{};{}H", row + 1, col + 1));
            out.push_str(&transmission(&pixels, width, height)?);
        }
        Ok(out)
    }
}

#[pymodule]
fn dre_rs(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<KittyGraphics>()?;
    m.add("CHUNK_SIZE", CHUNK_SIZE)?;
    m.add("DELETE_ALL", DELETE_ALL)?;
    Ok(())
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
        let encoded = encode(&pixels).unwrap();
        let compressed = STANDARD.decode(&encoded).unwrap();
        let mut decoder = ZlibDecoder::new(&compressed[..]);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed).unwrap();
        assert_eq!(decompressed, pixels);
    }

    #[test]
    fn transmission_single_chunk_has_expected_header_and_m0() {
        let pixels = vec![1u8, 2, 3, 4];
        let result = transmission(&pixels, 2, 1).unwrap();

        let expected_payload = encode(&pixels).unwrap();
        let expected_header = "a=T,f=32,s=2,v=1,o=z,q=2,z=-1,m=0".to_string();
        let expected = escape(&expected_header, &expected_payload);
        assert_eq!(result, expected);
        assert!(result.starts_with("\x1b_Ga=T,f=32,s=2,v=1,o=z,q=2,z=-1,m=0;"));
        assert!(result.ends_with("\x1b\\"));
    }

    #[test]
    fn transmission_multi_chunk_splits_and_sets_more_flag() {
        // Build pixel data large enough that the base64-encoded, zlib-compressed
        // payload spans more than one CHUNK_SIZE chunk.
        let mut state: u32 = 0x9E3779B9;
        let pixels: Vec<u8> = (0..200_000u32)
            .map(|_| {
                state = state.wrapping_mul(1664525).wrapping_add(1013904223);
                (state >> 16) as u8
            })
            .collect();
        let result = transmission(&pixels, 100, 100).unwrap();

        let payload = encode(&pixels).unwrap();
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
        // first chunk should be marked "more data coming" (m=1)
        assert!(result.contains(",m=1;"));
        // and the transmission should end with the final escape terminator
        assert!(result.ends_with("\x1b\\"));
        // there should be more than one escape sequence emitted
        assert!(result.matches("\x1b_G").count() > 1);
    }
}
