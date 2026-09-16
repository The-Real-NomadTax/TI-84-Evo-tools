use image::{ImageBuffer, Rgb};
use std::path::{Path, PathBuf};

const WIDTH: u32 = 320;
const HEIGHT: u32 = 240;

pub fn save(payload: &[u8]) -> Result<PathBuf, String> {
    let cache_dir = std::env::temp_dir().join("ti-screen-kirigami");
    std::fs::create_dir_all(&cache_dir).map_err(|error| error.to_string())?;
    let raw_path = cache_dir.join("last-capture.bin");
    std::fs::write(&raw_path, payload).map_err(|error| error.to_string())?;

    let data = extract_data_field(payload).unwrap_or(payload);
    let output = cache_dir.join("screen.png");
    if data.starts_with(b"\x89PNG\r\n\x1a\n") {
        std::fs::write(&output, data).map_err(|error| error.to_string())?;
        return Ok(output);
    }

    let image = match data.len() {
        153_600 => rgb565(data, false),
        76_800 => grayscale(data),
        length => {
            return Err(format!(
                "unsupported screen payload ({length} bytes); raw response saved to {}",
                raw_path.display()
            ));
        }
    };
    image.save(&output).map_err(|error| error.to_string())?;
    Ok(output)
}

fn rgb565(data: &[u8], big_endian: bool) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
    ImageBuffer::from_fn(WIDTH, HEIGHT, |x, y| {
        let offset = ((y * WIDTH + x) * 2) as usize;
        let value = if big_endian {
            u16::from_be_bytes([data[offset], data[offset + 1]])
        } else {
            u16::from_le_bytes([data[offset], data[offset + 1]])
        };
        Rgb([
            (((value >> 11) & 0x1f) * 255 / 31) as u8,
            (((value >> 5) & 0x3f) * 255 / 63) as u8,
            ((value & 0x1f) * 255 / 31) as u8,
        ])
    })
}

fn grayscale(data: &[u8]) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
    ImageBuffer::from_fn(WIDTH, HEIGHT, |x, y| {
        let value = data[(y * WIDTH + x) as usize];
        Rgb([value, value, value])
    })
}

// The TI protocol wraps resources in a CBOR-like metadata map. This deliberately
// small reader only locates a definite-length text key named "data" followed by
// a definite-length byte string. Raw framebuffer responses still work directly.
fn extract_data_field(payload: &[u8]) -> Option<&[u8]> {
    for index in 0..payload.len().saturating_sub(5) {
        if payload.get(index) == Some(&0x64) && payload.get(index + 1..index + 5) == Some(b"data") {
            let value = index + 5;
            let (length, header) = cbor_byte_string_len(payload.get(value..)?)?;
            return payload.get(value + header..value + header + length);
        }
    }
    None
}

fn cbor_byte_string_len(input: &[u8]) -> Option<(usize, usize)> {
    let first = *input.first()?;
    if first >> 5 != 2 {
        return None;
    }
    match first & 0x1f {
        length @ 0..=23 => Some((length as usize, 1)),
        24 => Some((*input.get(1)? as usize, 2)),
        25 => Some((
            u16::from_be_bytes([*input.get(1)?, *input.get(2)?]) as usize,
            3,
        )),
        26 => Some((
            u32::from_be_bytes([
                *input.get(1)?,
                *input.get(2)?,
                *input.get(3)?,
                *input.get(4)?,
            ]) as usize,
            5,
        )),
        _ => None,
    }
}

#[allow(dead_code)]
fn _is_file(path: &Path) -> bool {
    path.is_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_cbor_data() {
        let payload = [0xa1, 0x64, b'd', b'a', b't', b'a', 0x43, 1, 2, 3];
        assert_eq!(extract_data_field(&payload), Some(&[1, 2, 3][..]));
    }
}
