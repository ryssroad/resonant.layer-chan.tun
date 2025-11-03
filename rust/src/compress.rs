use std::io::Cursor;
use zstd::stream::{decode_all, encode_all};

/// Compress data using zstd
pub fn zstd_compress(input: &[u8], level: i32) -> anyhow::Result<Vec<u8>> {
    Ok(encode_all(Cursor::new(input), level)?)
}

/// Decompress data using zstd
pub fn zstd_decompress(input: &[u8]) -> anyhow::Result<Vec<u8>> {
    Ok(decode_all(Cursor::new(input))?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compress_decompress() {
        let data = b"Hello, Resonant Protocol!";
        let compressed = zstd_compress(data, 3).unwrap();
        let decompressed = zstd_decompress(&compressed).unwrap();
        assert_eq!(data, decompressed.as_slice());
    }
}
