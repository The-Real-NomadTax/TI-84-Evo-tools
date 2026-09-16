use std::fmt;

const MARK: u8 = 0x01;
const EOL: u8 = 0x0d;
const QCTL: u8 = b'#';
const REPT: u8 = b'~';

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Packet {
    pub sequence: u8,
    pub kind: u8,
    pub data: Vec<u8>,
}

#[derive(Debug)]
pub enum Error {
    InvalidPacket(&'static str),
    Remote(String),
    Incomplete,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPacket(message) => write!(f, "invalid Kermit packet: {message}"),
            Self::Remote(message) => write!(f, "calculator reported: {message}"),
            Self::Incomplete => f.write_str("incomplete Kermit packet"),
        }
    }
}

impl std::error::Error for Error {}

pub fn sender_init(sequence: u8) -> Vec<u8> {
    encode_packet(sequence, b'S', negotiation_parameters())
}

pub fn acknowledgement(sequence: u8, send_init: bool) -> Vec<u8> {
    let parameters = if send_init {
        negotiation_parameters()
    } else {
        &[]
    };
    encode_packet(sequence, b'Y', parameters)
}

fn negotiation_parameters() -> &'static [u8] {
    // MAXL, TIME, NPAD, PADC, EOL, QCTL, QBIN, CHKT, REPT, CAPAS, WINDO,
    // MAXLX1, MAXLX2. These are the exact parameters sent by TI Connect Evo.
    static PARAMETERS: [u8; 13] = *b"~0 @-#Y1~.\"5M";
    &PARAMETERS
}

pub fn encode_packet(sequence: u8, kind: u8, data: &[u8]) -> Vec<u8> {
    if data.len() > 91 {
        return encode_long_packet(sequence, kind, data);
    }
    let mut packet = Vec::with_capacity(data.len() + 6);
    packet.push(MARK);
    packet.push(to_char((data.len() + 3) as u8));
    packet.push(to_char(sequence & 0x3f));
    packet.push(kind);
    packet.extend_from_slice(data);
    let check = checksum1(&packet[1..]);
    packet.push(to_char(check));
    packet.push(EOL);
    packet
}

fn encode_long_packet(sequence: u8, kind: u8, data: &[u8]) -> Vec<u8> {
    let extended_len = data.len() + 1;
    assert!(
        extended_len < 95 * 95,
        "long Kermit packet data is too large"
    );
    let mut packet = Vec::with_capacity(data.len() + 9);
    packet.push(MARK);
    packet.push(to_char(0));
    packet.push(to_char(sequence & 0x3f));
    packet.push(kind);
    packet.push(to_char((extended_len / 95) as u8));
    packet.push(to_char((extended_len % 95) as u8));
    packet.push(to_char(checksum1(&packet[1..])));
    packet.extend_from_slice(data);
    packet.push(to_char(checksum1(&packet[1..])));
    packet.push(EOL);
    packet
}

pub fn file_attributes(contents_len: usize) -> Vec<u8> {
    let length = contents_len.to_string();
    assert!(length.len() <= 94, "file length attribute is too large");
    let mut attributes = b"\"\"B8".to_vec();
    attributes.push(b'1');
    attributes.push(to_char(length.len() as u8));
    attributes.extend_from_slice(length.as_bytes());
    attributes.push(b'@');
    attributes.push(b' ');
    attributes
}

pub fn parse_packet(input: &[u8]) -> Result<(Packet, usize), Error> {
    let start = input
        .iter()
        .position(|byte| *byte == MARK)
        .ok_or(Error::Incomplete)?;
    if input.len() < start + 6 {
        return Err(Error::Incomplete);
    }

    let len = from_char(input[start + 1])? as usize;
    if len != 0 {
        // LEN counts SEQ, TYPE, DATA, and CHECK. MARK, LEN itself, and EOL
        // sit outside that count.
        let end = start + len + 3;
        if input.len() < end {
            return Err(Error::Incomplete);
        }
        if input[end - 1] != EOL {
            return Err(Error::InvalidPacket("missing end-of-line marker"));
        }
        let expected = from_char(input[end - 2])?;
        if checksum1(&input[start + 1..end - 2]) != expected {
            return Err(Error::InvalidPacket("checksum mismatch"));
        }
        let sequence = from_char(input[start + 2])?;
        let packet = Packet {
            sequence,
            kind: input[start + 3],
            data: input[start + 4..end - 2].to_vec(),
        };
        return Ok((packet, end));
    }

    // Long packets use two base-95 length characters and a separate header
    // checksum. XLEN counts data plus the final block check character.
    if input.len() < start + 9 {
        return Err(Error::Incomplete);
    }
    let high = from_char(input[start + 4])? as usize;
    let low = from_char(input[start + 5])? as usize;
    let extended_len = high * 95 + low;
    if extended_len == 0 {
        return Err(Error::InvalidPacket("zero extended length"));
    }
    let end = start + 7 + extended_len + 1;
    if input.len() < end {
        return Err(Error::Incomplete);
    }
    let header_check = from_char(input[start + 6])?;
    if checksum1(&input[start + 1..start + 6]) != header_check {
        return Err(Error::InvalidPacket("long-packet header checksum mismatch"));
    }
    let expected = from_char(input[end - 2])?;
    if checksum1(&input[start + 1..end - 2]) != expected {
        return Err(Error::InvalidPacket("long-packet checksum mismatch"));
    }
    let packet = Packet {
        sequence: from_char(input[start + 2])?,
        kind: input[start + 3],
        data: input[start + 7..end - 2].to_vec(),
    };
    Ok((packet, end))
}

pub fn decode_data(encoded: &[u8]) -> Result<Vec<u8>, Error> {
    let mut decoded = Vec::with_capacity(encoded.len());
    let mut index = 0;
    while index < encoded.len() {
        let mut repetitions = 1usize;
        if encoded[index] == REPT {
            if index + 2 >= encoded.len() {
                return Err(Error::InvalidPacket("truncated repeat quote"));
            }
            repetitions = from_char(encoded[index + 1])? as usize;
            index += 2;
        }

        let mut value = encoded[index];
        if value == QCTL {
            index += 1;
            if index >= encoded.len() {
                return Err(Error::InvalidPacket("truncated control quote"));
            }
            value = encoded[index];
            // Kermit quotes literal prefix characters by placing QCTL before
            // them. Only the printable control-code range is transformed.
            let low = value & 0x7f;
            if low == b'?' || (b'@'..=b'_').contains(&low) {
                value = ctl(value);
            }
        }
        decoded.extend(std::iter::repeat_n(value, repetitions));
        index += 1;
    }
    Ok(decoded)
}

pub fn encode_data_chunks(data: &[u8], maximum: usize) -> Vec<Vec<u8>> {
    let mut chunks = Vec::new();
    let mut chunk = Vec::new();
    for byte in data {
        let mut encoded = Vec::with_capacity(2);
        let value = *byte;
        let low = value & 0x7f;
        if low < 0x20 || low == 0x7f {
            encoded.push(QCTL);
            encoded.push(ctl(value));
        } else if matches!(value, QCTL | REPT) {
            encoded.push(QCTL);
            encoded.push(value);
        } else {
            encoded.push(value);
        }

        if !chunk.is_empty() && chunk.len() + encoded.len() > maximum {
            chunks.push(std::mem::take(&mut chunk));
        }
        chunk.extend(encoded);
    }
    if !chunk.is_empty() {
        chunks.push(chunk);
    }
    chunks
}

fn checksum1(bytes: &[u8]) -> u8 {
    let sum = bytes
        .iter()
        .fold(0u16, |sum, byte| sum.wrapping_add(*byte as u16));
    ((sum + ((sum & 0xc0) >> 6)) & 0x3f) as u8
}

fn to_char(value: u8) -> u8 {
    value + 32
}

fn from_char(value: u8) -> Result<u8, Error> {
    value
        .checked_sub(32)
        .ok_or(Error::InvalidPacket("invalid printable number"))
}

fn ctl(value: u8) -> u8 {
    value ^ 0x40
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packet_round_trip() {
        let bytes = encode_packet(0, b'F', b"hh01/inf/res?name=screencapture");
        let (packet, used) = parse_packet(&bytes).unwrap();
        assert_eq!(used, bytes.len());
        assert_eq!(packet.sequence, 0);
        assert_eq!(packet.kind, b'F');
        assert_eq!(packet.data, b"hh01/inf/res?name=screencapture");
    }

    #[test]
    fn long_packet_round_trip() {
        let data = vec![b'x'; 200];
        let bytes = encode_packet(4, b'F', &data);
        let (packet, used) = parse_packet(&bytes).unwrap();
        assert_eq!(used, bytes.len());
        assert_eq!(packet.sequence, 4);
        assert_eq!(packet.kind, b'F');
        assert_eq!(packet.data, data);
    }

    #[test]
    fn attributes_describe_binary_file_size() {
        assert_eq!(file_attributes(1548), b"\"\"B81$1548@ ");
    }

    #[test]
    fn sender_init_matches_ti_connect_evo() {
        assert_eq!(
            sender_init(0),
            b"\x01\x30\x20\x53\x7e\x30\x20\x40\x2d\x23\x59\x31\x7e\x2e\x22\x35\x4d\x3e\x0d"
        );
    }

    #[test]
    fn one_byte_transfer_matches_ti_connect_evo() {
        assert_eq!(
            encode_packet(2, b'A', &file_attributes(1)),
            b"\x01\x2c\x22\x41\x22\x22\x42\x38\x31\x21\x31\x40\x20\x50\x0d"
        );
        assert_eq!(
            encode_packet(3, b'D', &encode_data_chunks(&[0], 90)[0]),
            b"\x01\x25\x23\x44\x23\x40\x52\x0d"
        );
    }

    #[test]
    fn decodes_control_high_bit_and_repeat_quotes() {
        let encoded = [QCTL, b'A', 0xc1, REPT, to_char(3), b'x'];
        assert_eq!(decode_data(&encoded).unwrap(), [1, 0xc1, b'x', b'x', b'x']);
    }

    #[test]
    fn decodes_evo_high_bit_control_quotes() {
        let encoded = [QCTL, 0xdf, QCTL, 0xbf];
        assert_eq!(decode_data(&encoded).unwrap(), [0x9f, 0xff]);
    }

    #[test]
    fn encodes_binary_data_in_bounded_chunks() {
        let input = [0, b'#', b'&', b'~', 0x80, 0xff, b'A'];
        let chunks = encode_data_chunks(&input, 5);
        assert!(chunks.iter().all(|chunk| chunk.len() <= 5));
        let decoded: Vec<u8> = chunks
            .iter()
            .flat_map(|chunk| decode_data(chunk).unwrap())
            .collect();
        assert_eq!(decoded, input);
    }

    #[test]
    fn round_trips_multi_kilobyte_transfer() {
        let input: Vec<u8> = (0..3975)
            .map(|index| ((index * 37 + 11) & 0xff) as u8)
            .collect();
        let chunks = encode_data_chunks(&input, 90);
        assert!(chunks.len() > 40);
        assert!(chunks.iter().all(|chunk| chunk.len() <= 90));
        let decoded: Vec<u8> = chunks
            .iter()
            .flat_map(|chunk| decode_data(chunk).unwrap())
            .collect();
        assert_eq!(decoded, input);
    }
}
