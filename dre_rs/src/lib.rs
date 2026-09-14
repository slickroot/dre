use pyo3::prelude::*;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::io::Write;

#[pyfunction]
fn encode(pixels: &[u8]) -> PyResult<String> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(pixels)?;
    let compressed = encoder.finish()?;
    Ok(STANDARD.encode(compressed))
}

#[pyfunction]
fn chunks(payload: &str, chunk_size: usize) -> Vec<String> {
    payload
        .as_bytes()
        .chunks(chunk_size)
        .map(|chunk| std::str::from_utf8(chunk).unwrap().to_owned())
        .collect()
}

#[pyfunction]
fn more(chunks: Vec<String>, index: usize) -> i32 {
    if index == chunks.len() - 1 { 0 } else { 1 }
}

#[pyfunction]
fn escape(keys: &str, payload: &str) -> String {
    format!("\x1b_G{keys};{payload}\x1b\\")
}

#[pymodule]
fn dre_rs(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(encode, m)?)?;
    m.add_function(wrap_pyfunction!(chunks, m)?)?;
    m.add_function(wrap_pyfunction!(more, m)?)?;
    m.add_function(wrap_pyfunction!(escape, m)?)?;
    Ok(())
}
