use crate::directory::{as_bytes, as_integer, decode, map_get};
use std::path::Path;

const EVO_PYTHON_TYPE: u8 = 15;

pub struct VariableFile {
    pub display_name: String,
    pub wire_name: String,
    pub kind: u8,
    pub archived: bool,
    pub contents: Vec<u8>,
}

pub fn read(path: &Path) -> Result<VariableFile, String> {
    let bytes = std::fs::read(path).map_err(|error| format!("could not read file: {error}"))?;
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match extension.as_str() {
        "8xp2" => parse_evo_program(path, &bytes),
        "py" => parse_python(path, &bytes),
        _ => Err("unsupported file: select a TI-84 EVO .8xp2 or Python .py file".into()),
    }
}

fn parse_evo_program(path: &Path, bytes: &[u8]) -> Result<VariableFile, String> {
    let root = decode(bytes).map_err(|error| format!("invalid EVO container: {error}"))?;
    let metadata = map_get(&root, "metaData")
        .ok_or_else(|| "invalid EVO program: missing metadata".to_string())?;
    let container_type = map_get(metadata, "type").and_then(as_integer).unwrap_or(-1);
    if !(0..=i128::from(u8::MAX)).contains(&container_type) {
        return Err(format!("invalid EVO program type {container_type}"));
    }

    let token_name = map_get(metadata, "name")
        .and_then(as_bytes)
        .ok_or_else(|| "invalid EVO program: missing token name".to_string())?;
    let display_name = decode_evo_name(token_name).unwrap_or_else(|| file_stem(path));
    let wire_name = percent_encode_utf8(&token_name_to_string(token_name)?);
    map_get(&root, "data")
        .and_then(as_bytes)
        .ok_or_else(|| "invalid EVO program: missing data".to_string())?;
    let archived = map_get(metadata, "flags")
        .and_then(as_integer)
        .is_some_and(|flags| flags & 1 != 0);

    Ok(VariableFile {
        display_name,
        wire_name,
        // EVO .8xp2 files carry the xfr/var type in their own metadata. It is
        // not the legacy TI-84 Plus CE program type (15); native programs seen
        // on the EVO use type 2, while Python programs can use other types.
        kind: container_type as u8,
        archived,
        // The EVO xfr/var endpoint validates and imports the complete native
        // CBOR container, including its trailing checksum.
        contents: bytes.to_vec(),
    })
}

fn parse_python(path: &Path, bytes: &[u8]) -> Result<VariableFile, String> {
    let display_name = calculator_name(path)?;
    std::str::from_utf8(bytes).map_err(|_| "Python source must be valid UTF-8".to_string())?;
    if bytes.contains(&0) {
        return Err("Python source cannot contain NUL bytes".into());
    }

    let mut program = Vec::new();
    let program_size = bytes.len() + display_name.len() + 18;
    program.extend_from_slice(&0x113u32.to_le_bytes());
    program.extend_from_slice(
        &u32::try_from(program_size)
            .map_err(|_| "Python program is too large".to_string())?
            .to_le_bytes(),
    );
    program.extend_from_slice(&(display_name.len() as u32).to_le_bytes());
    program.extend_from_slice(display_name.as_bytes());
    let source_size =
        u32::try_from(bytes.len()).map_err(|_| "Python program is too large".to_string())?;
    if source_size > 0x00ff_ffff {
        return Err("Python source exceeds the EVO 24-bit size limit".into());
    }
    let source_size = source_size.to_le_bytes();
    program.extend_from_slice(&[0, source_size[0], source_size[1], source_size[2], 2]);
    program.extend_from_slice(bytes);
    program.push(0);

    let contents = python_container(&display_name, &program)?;
    Ok(VariableFile {
        wire_name: percent_encode_utf8(&ascii_name_to_evo_tokens(&display_name)),
        display_name,
        kind: EVO_PYTHON_TYPE,
        archived: false,
        contents,
    })
}

fn python_container(name: &str, program: &[u8]) -> Result<Vec<u8>, String> {
    let mut token_name = Vec::with_capacity(name.len() * 2);
    for byte in name.bytes() {
        token_name.extend_from_slice(&[byte - b'A', 0xe8]);
    }

    let mut output = vec![0xbf];
    cbor_text(&mut output, "metaData")?;
    output.push(0xbf);
    cbor_text(&mut output, "type")?;
    cbor_unsigned(&mut output, u64::from(EVO_PYTHON_TYPE));
    cbor_text(&mut output, "version")?;
    cbor_unsigned(&mut output, 1);
    cbor_text(&mut output, "name")?;
    cbor_bytes(&mut output, &token_name)?;
    output.push(0xff);
    cbor_text(&mut output, "version")?;
    cbor_unsigned(&mut output, 1);
    cbor_text(&mut output, "size")?;
    cbor_unsigned(
        &mut output,
        u64::try_from(program.len()).map_err(|_| "Python program is too large".to_string())?,
    );
    cbor_text(&mut output, "data")?;
    cbor_bytes(&mut output, program)?;
    output.push(0xff);
    Ok(output)
}

fn cbor_text(output: &mut Vec<u8>, value: &str) -> Result<(), String> {
    cbor_len(output, 3, value.len())?;
    output.extend_from_slice(value.as_bytes());
    Ok(())
}

fn cbor_bytes(output: &mut Vec<u8>, value: &[u8]) -> Result<(), String> {
    cbor_len(output, 2, value.len())?;
    output.extend_from_slice(value);
    Ok(())
}

fn cbor_len(output: &mut Vec<u8>, major: u8, len: usize) -> Result<(), String> {
    let len = u64::try_from(len).map_err(|_| "EVO container is too large".to_string())?;
    cbor_major(output, major, len);
    Ok(())
}

fn cbor_unsigned(output: &mut Vec<u8>, value: u64) {
    cbor_major(output, 0, value);
}

fn cbor_major(output: &mut Vec<u8>, major: u8, value: u64) {
    let prefix = major << 5;
    match value {
        0..=23 => output.push(prefix | value as u8),
        24..=0xff => output.extend_from_slice(&[prefix | 24, value as u8]),
        0x100..=0xffff => {
            output.push(prefix | 25);
            output.extend_from_slice(&(value as u16).to_be_bytes());
        }
        0x1_0000..=0xffff_ffff => {
            output.push(prefix | 26);
            output.extend_from_slice(&(value as u32).to_be_bytes());
        }
        _ => {
            output.push(prefix | 27);
            output.extend_from_slice(&value.to_be_bytes());
        }
    }
}

fn calculator_name(path: &Path) -> Result<String, String> {
    let name = file_stem(path).to_ascii_uppercase();
    if name.is_empty() || name.len() > 8 || !name.bytes().all(|byte| byte.is_ascii_uppercase()) {
        return Err("Python filename must contain 1–8 letters for the TI-84 EVO".into());
    }
    Ok(name)
}

fn file_stem(path: &Path) -> String {
    path.file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("PROGRAM")
        .to_ascii_uppercase()
}

fn token_name_to_string(bytes: &[u8]) -> Result<String, String> {
    let (pairs, remainder) = bytes.as_chunks::<2>();
    if !remainder.is_empty() {
        return Err("invalid EVO token name length".into());
    }
    let mut output = String::new();
    for pair in pairs {
        let value = u16::from_le_bytes(*pair);
        if value == 0 {
            break;
        }
        output.push(
            char::from_u32(u32::from(value))
                .ok_or_else(|| "invalid character in EVO token name".to_string())?,
        );
    }
    if output.is_empty() {
        return Err("empty EVO token name".into());
    }
    Ok(output)
}

fn decode_evo_name(bytes: &[u8]) -> Option<String> {
    let tokens = token_name_to_string(bytes).ok()?;
    let mut output = String::new();
    for token in tokens.chars() {
        match token as u32 {
            value @ 0xe800..=0xe819 => output.push(char::from(b'A' + (value - 0xe800) as u8)),
            _ => return None,
        }
    }
    Some(output)
}

fn ascii_name_to_evo_tokens(name: &str) -> String {
    name.bytes()
        .map(|byte| char::from_u32(0xe800 + u32::from(byte - b'A')).unwrap())
        .collect()
}

pub(crate) fn evo_wire_name(name: &str) -> Result<String, String> {
    let name = name.to_ascii_uppercase();
    if name.is_empty() || name.len() > 8 || !name.bytes().all(|byte| byte.is_ascii_uppercase()) {
        return Err("EVO variable name must contain 1–8 letters".into());
    }
    Ok(percent_encode_utf8(&ascii_name_to_evo_tokens(&name)))
}

fn percent_encode_utf8(value: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut output = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            output.push(char::from(byte));
        } else {
            output.push('%');
            output.push(char::from(HEX[usize::from(byte >> 4)]));
            output.push(char::from(HEX[usize::from(byte & 0x0f)]));
        }
    }
    output
}

pub(crate) fn evo_wire_name_from_tokens(bytes: &[u8]) -> Result<String, String> {
    Ok(percent_encode_utf8(&token_name_to_string(bytes)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_native_evo_program() {
        let bytes = b"\xbf\x68metaData\xbf\x64type\x02\x65flags\x00\x64name\x44\x07\xe8\x08\xe8\xff\x64data\x43abc\xff\x00\x00";
        let variable = parse_evo_program(Path::new("HI.8xp2"), bytes).unwrap();
        assert_eq!(variable.display_name, "HI");
        assert_eq!(variable.kind, 2);
        assert_eq!(variable.contents, bytes);
        assert_eq!(variable.wire_name, "%EE%A0%87%EE%A0%88");
    }

    #[test]
    fn converts_python_source_for_evo() {
        let variable = parse_python(Path::new("hello.py"), b"print(42)\r\n").unwrap();
        assert_eq!(variable.display_name, "HELLO");
        assert_eq!(variable.kind, EVO_PYTHON_TYPE);
        let root = decode(&variable.contents).unwrap();
        assert_eq!(
            map_get(map_get(&root, "metaData").unwrap(), "type").and_then(as_integer),
            Some(15)
        );
        let data = map_get(&root, "data").and_then(as_bytes).unwrap();
        let source = b"print(42)\r\n";
        assert!(data.windows(source.len()).any(|window| window == source));
        assert_eq!(&data[..4], &0x113u32.to_le_bytes());
        assert_eq!(
            data.len(),
            u32::from_le_bytes(data[4..8].try_into().unwrap()) as usize
        );
    }

    #[test]
    fn python_conversion_matches_ti_converter() {
        let variable = parse_python(Path::new("hello.py"), b"print(\"hello\")\n").unwrap();
        let expected = b"\xbf\x68metaData\xbf\x64type\x0f\x67version\x01\x64name\x4a\x07\xe8\x04\xe8\x0b\xe8\x0b\xe8\x0e\xe8\xff\x67version\x01\x64size\x18\x26\x64data\x58\x26\x13\x01\x00\x00\x26\x00\x00\x00\x05\x00\x00\x00HELLO\x00\x0f\x00\x00\x02print(\"hello\")\n\x00\xff";
        assert_eq!(variable.contents, expected);
    }

    #[test]
    fn python_source_length_uses_three_bytes() {
        let source = vec![b'x'; 3886];
        let variable = parse_python(Path::new("game.py"), &source).unwrap();
        let root = decode(&variable.contents).unwrap();
        let data = map_get(&root, "data").and_then(as_bytes).unwrap();
        // Fixed fields (12), name (4), and its NUL terminator precede the
        // three-byte little-endian source length and format marker.
        assert_eq!(&data[17..21], &[0x2e, 0x0f, 0x00, 0x02]);
    }

    #[test]
    fn rejects_invalid_python_name() {
        assert!(parse_python(Path::new("hello-world.py"), b"pass\n").is_err());
    }
}
