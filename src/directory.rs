#[derive(Debug)]
pub(crate) enum Value {
    Integer(i128),
    Bytes(Vec<u8>),
    Text(String),
    Array(Vec<Value>),
    Map(Vec<(Value, Value)>),
    Bool(bool),
    Null,
}

pub struct Listing {
    pub text: String,
    pub files_json: String,
}

pub fn format_listing(payload: &[u8]) -> Result<String, String> {
    Ok(listing(payload)?.text)
}

pub fn listing(payload: &[u8]) -> Result<Listing, String> {
    let root = decode(payload)?;
    let entries = map_get(&root, "data")
        .and_then(as_array)
        .ok_or_else(|| "directory response has no data array".to_string())?;

    if entries.is_empty() {
        return Ok(Listing {
            text: "No files found on the calculator.".into(),
            files_json: "[]".into(),
        });
    }

    let mut output = format!("{} items\n\n", entries.len());
    let mut files_json = String::from("[");
    for entry in entries {
        let name = map_get(entry, "dispName")
            .and_then(as_name)
            .unwrap_or_else(|| "(unnamed)".into());
        let size = map_get(entry, "size").and_then(as_integer).unwrap_or(0);
        let kind = map_get(entry, "type").and_then(as_integer).unwrap_or(-1);
        let version = map_get(entry, "version").and_then(as_integer).unwrap_or(0);
        let memory_value = map_get(entry, "mem").and_then(as_bool);
        let memory = memory_value.map_or("unknown", |value| if value { "yes" } else { "no" });
        output.push_str(&format!(
            "{name}\n  {size} bytes  ·  type {kind}  ·  version {version}  ·  memory {memory}\n\n"
        ));

        let wire_name = map_get(entry, "tokName")
            .and_then(as_bytes)
            .and_then(|tokens| crate::ti_file::evo_wire_name_from_tokens(tokens).ok())
            .unwrap_or_default();
        if files_json.len() > 1 {
            files_json.push(',');
        }
        files_json.push_str("{\"name\":\"");
        push_json_string(&mut files_json, &name);
        files_json.push_str("\",\"wireName\":\"");
        push_json_string(&mut files_json, &wire_name);
        files_json.push_str(&format!(
            "\",\"type\":{kind},\"size\":{size},\"version\":{version},\"memory\":{} }}",
            memory_value.unwrap_or(false)
        ));
    }
    files_json.push(']');
    Ok(Listing {
        text: output.trim_end().to_string(),
        files_json,
    })
}

fn push_json_string(output: &mut String, value: &str) {
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character.is_control() => {
                output.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => output.push(character),
        }
    }
}

pub(crate) fn decode(payload: &[u8]) -> Result<Value, String> {
    let mut offset = 0;
    parse(payload, &mut offset)
}

fn parse(input: &[u8], offset: &mut usize) -> Result<Value, String> {
    let initial = take(input, offset)?;
    if initial == 0xff {
        return Err("unexpected CBOR break".into());
    }
    let major = initial >> 5;
    let additional = initial & 0x1f;
    match major {
        0 => Ok(Value::Integer(
            read_argument(input, offset, additional)? as i128
        )),
        1 => Ok(Value::Integer(
            -1 - read_argument(input, offset, additional)? as i128,
        )),
        2 => parse_bytes(input, offset, additional),
        3 => parse_text(input, offset, additional),
        4 => parse_array(input, offset, additional),
        5 => parse_map(input, offset, additional),
        6 => {
            read_argument(input, offset, additional)?;
            parse(input, offset)
        }
        7 => match additional {
            20 => Ok(Value::Bool(false)),
            21 => Ok(Value::Bool(true)),
            22 | 23 => Ok(Value::Null),
            value => Err(format!("unsupported CBOR simple value {value}")),
        },
        _ => Err("unsupported CBOR value".into()),
    }
}

fn parse_bytes(input: &[u8], offset: &mut usize, additional: u8) -> Result<Value, String> {
    if additional == 31 {
        let mut bytes = Vec::new();
        while input.get(*offset) != Some(&0xff) {
            match parse(input, offset)? {
                Value::Bytes(chunk) => bytes.extend(chunk),
                _ => return Err("non-byte chunk in indefinite byte string".into()),
            }
        }
        *offset += 1;
        return Ok(Value::Bytes(bytes));
    }
    let length = read_argument(input, offset, additional)? as usize;
    let end = offset
        .checked_add(length)
        .filter(|end| *end <= input.len())
        .ok_or_else(|| "truncated CBOR byte string".to_string())?;
    let bytes = input[*offset..end].to_vec();
    *offset = end;
    Ok(Value::Bytes(bytes))
}

fn parse_text(input: &[u8], offset: &mut usize, additional: u8) -> Result<Value, String> {
    if additional == 31 {
        let mut text = String::new();
        while input.get(*offset) != Some(&0xff) {
            match parse(input, offset)? {
                Value::Text(chunk) => text.push_str(&chunk),
                _ => return Err("non-text chunk in indefinite text string".into()),
            }
        }
        *offset += 1;
        return Ok(Value::Text(text));
    }
    let length = read_argument(input, offset, additional)? as usize;
    let end = offset
        .checked_add(length)
        .filter(|end| *end <= input.len())
        .ok_or_else(|| "truncated CBOR text".to_string())?;
    let text = String::from_utf8_lossy(&input[*offset..end]).into_owned();
    *offset = end;
    Ok(Value::Text(text))
}

fn parse_array(input: &[u8], offset: &mut usize, additional: u8) -> Result<Value, String> {
    let mut values = Vec::new();
    if additional == 31 {
        while input.get(*offset) != Some(&0xff) {
            values.push(parse(input, offset)?);
        }
        *offset += 1;
    } else {
        for _ in 0..read_argument(input, offset, additional)? {
            values.push(parse(input, offset)?);
        }
    }
    Ok(Value::Array(values))
}

fn parse_map(input: &[u8], offset: &mut usize, additional: u8) -> Result<Value, String> {
    let mut values = Vec::new();
    if additional == 31 {
        while input.get(*offset) != Some(&0xff) {
            values.push((parse(input, offset)?, parse(input, offset)?));
        }
        *offset += 1;
    } else {
        for _ in 0..read_argument(input, offset, additional)? {
            values.push((parse(input, offset)?, parse(input, offset)?));
        }
    }
    Ok(Value::Map(values))
}

fn read_argument(input: &[u8], offset: &mut usize, additional: u8) -> Result<u64, String> {
    match additional {
        value @ 0..=23 => Ok(u64::from(value)),
        24 => Ok(u64::from(take(input, offset)?)),
        25 => Ok(u64::from(u16::from_be_bytes(take_array(input, offset)?))),
        26 => Ok(u64::from(u32::from_be_bytes(take_array(input, offset)?))),
        27 => Ok(u64::from_be_bytes(take_array(input, offset)?)),
        31 => Err("indefinite length is not valid here".into()),
        value => Err(format!("invalid CBOR argument {value}")),
    }
}

fn take(input: &[u8], offset: &mut usize) -> Result<u8, String> {
    let value = *input
        .get(*offset)
        .ok_or_else(|| "truncated CBOR response".to_string())?;
    *offset += 1;
    Ok(value)
}

fn take_array<const N: usize>(input: &[u8], offset: &mut usize) -> Result<[u8; N], String> {
    let end = offset
        .checked_add(N)
        .filter(|end| *end <= input.len())
        .ok_or_else(|| "truncated CBOR number".to_string())?;
    let value = input[*offset..end]
        .try_into()
        .map_err(|_| "invalid CBOR number".to_string())?;
    *offset = end;
    Ok(value)
}

pub(crate) fn map_get<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    let Value::Map(entries) = value else {
        return None;
    };
    entries
        .iter()
        .find_map(|(entry_key, value)| match entry_key {
            Value::Text(entry_key) if entry_key == key => Some(value),
            _ => None,
        })
}

fn as_array(value: &Value) -> Option<&[Value]> {
    match value {
        Value::Array(value) => Some(value),
        _ => None,
    }
}

fn as_name(value: &Value) -> Option<std::borrow::Cow<'_, str>> {
    match value {
        Value::Text(value) => Some(value.into()),
        Value::Bytes(value) => Some(String::from_utf8_lossy(value)),
        _ => None,
    }
}

pub(crate) fn as_integer(value: &Value) -> Option<i128> {
    match value {
        Value::Integer(value) => Some(*value),
        _ => None,
    }
}

pub(crate) fn as_bytes(value: &Value) -> Option<&[u8]> {
    match value {
        Value::Bytes(value) => Some(value),
        _ => None,
    }
}

fn as_bool(value: &Value) -> Option<bool> {
    match value {
        Value::Bool(value) => Some(*value),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_indefinite_directory() {
        let payload = b"\xbf\x64data\x9f\xbf\x68dispName\x65HELLO\x67tokName\x44\x07\xe8\x08\xe8\x64size\x18\x2a\x64type\x05\x67version\x01\x63mem\xf5\xff\xff\xff";
        let listing = listing(payload).unwrap();
        assert!(listing.text.contains("1 items"));
        assert!(listing.text.contains("HELLO"));
        assert!(listing.text.contains("42 bytes"));
        assert!(
            listing
                .files_json
                .contains("\"wireName\":\"%EE%A0%87%EE%A0%88\"")
        );
        assert!(listing.files_json.contains("\"type\":5"));
    }

    #[test]
    fn decodes_indefinite_text() {
        let value = decode(b"\x7f\x62TI\x63EVO\xff").unwrap();
        assert!(matches!(value, Value::Text(text) if text == "TIEVO"));
    }
}
