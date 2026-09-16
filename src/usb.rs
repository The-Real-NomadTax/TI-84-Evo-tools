use crate::kermit::{self, Packet};
use rusb::{Context, Device, DeviceHandle, Direction, TransferType, UsbContext};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

const TI_VENDOR_ID: u16 = 0x0451;
const TI_84_EVO_PRODUCT_ID: u16 = 0xe018;
const IO_TIMEOUT: Duration = Duration::from_secs(2);
const CAPTURE_TIMEOUT: Duration = Duration::from_secs(15);
// TI Connect Evo sends a single zero byte for a variable delete. The Kermit
// negotiation and file attributes must also match TI's transaction so the
// calculator treats this as a named-variable delete rather than a RAM action.
const DELETE_VARIABLE_PAYLOAD: [u8; 1] = [0];
static TRACE_USB: AtomicBool = AtomicBool::new(false);

struct Connection {
    handle: DeviceHandle<Context>,
    interface: u8,
    endpoint_in: u8,
    endpoint_out: u8,
}

impl Drop for Connection {
    fn drop(&mut self) {
        let _ = self.handle.release_interface(self.interface);
    }
}

pub fn capture_screen(include_cursor: bool) -> Result<std::path::PathBuf, String> {
    let payload = request_screen(include_cursor, None)?;
    crate::screen::save(&payload)
}

pub fn probe_screen(
    include_cursor: bool,
    interface: Option<u8>,
    send_break: bool,
    send_attributes: bool,
) -> Result<std::path::PathBuf, String> {
    TRACE_USB.store(true, Ordering::Relaxed);
    let payload =
        request_screen_with_options(include_cursor, interface, send_break, send_attributes)?;
    crate::screen::save(&payload)
}

fn request_screen(include_cursor: bool, interface: Option<u8>) -> Result<Vec<u8>, String> {
    request_screen_with_options(include_cursor, interface, true, true)
}

fn request_screen_with_options(
    include_cursor: bool,
    interface: Option<u8>,
    send_break: bool,
    send_attributes: bool,
) -> Result<Vec<u8>, String> {
    if !include_cursor {
        return match request_uri("hh01/sys/screen", interface, send_break, send_attributes) {
            Ok(payload) => Ok(payload),
            Err(error) if error.contains("timed out waiting for a Kermit packet") => {
                trace(format_args!(
                    "active-screen endpoint did not respond; trying screencapture resource"
                ));
                request_resource(
                    "res?name=screencapture",
                    interface,
                    send_break,
                    send_attributes,
                )
            }
            Err(error) => Err(error),
        };
    }

    let resource = if include_cursor {
        "res?name=screencapture&withcursor=1"
    } else {
        "res?name=screencapture"
    };
    match request_resource(resource, interface, send_break, send_attributes) {
        Ok(payload) => Ok(payload),
        Err(error) if error.contains("timed out waiting for a Kermit packet") => {
            trace(format_args!(
                "screencapture resource did not respond; trying active-screen endpoint"
            ));
            request_uri("hh01/sys/screen", interface, send_break, send_attributes)
        }
        Err(error) => Err(error),
    }
}

pub fn directory() -> Result<Vec<u8>, String> {
    request_resource("res?name=directory", None, true, true)
}

pub fn send_host_file(path: &std::path::Path) -> Result<String, String> {
    send_host_file_on_interface(path, None)
}

pub fn delete_variable(wire_name: &str, kind: u8) -> Result<(), String> {
    if wire_name.is_empty()
        || !wire_name.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'%' | b'-' | b'_' | b'.' | b'~')
        })
    {
        return Err("invalid EVO variable name".into());
    }
    let destination = format!("hh01/del/var?name={wire_name}&type={kind}");
    let connection = connect(None)?;
    send_file(&connection, &destination, &DELETE_VARIABLE_PAYLOAD)
}

pub fn probe_delete_variable(name: &str, kind: u8, interface: Option<u8>) -> Result<(), String> {
    TRACE_USB.store(true, Ordering::Relaxed);
    let wire_name = crate::ti_file::evo_wire_name(name)?;
    let destination = format!("hh01/del/var?name={wire_name}&type={kind}");
    let connection = connect(interface)?;
    send_file(&connection, &destination, &DELETE_VARIABLE_PAYLOAD)
}

pub fn probe_send_host_file(
    path: &std::path::Path,
    interface: Option<u8>,
) -> Result<String, String> {
    TRACE_USB.store(true, Ordering::Relaxed);
    send_host_file_on_interface(path, interface)
}

fn send_host_file_on_interface(
    path: &std::path::Path,
    interface: Option<u8>,
) -> Result<String, String> {
    let variable = crate::ti_file::read(path)?;
    let memory_target = u8::from(variable.archived);
    let destination = format!(
        "hh01/xfr/var?name={}&type={}&memtarget={memory_target}&policy=0",
        variable.wire_name, variable.kind
    );
    trace(format_args!(
        "upload {} as type {}, URI {}, payload {} bytes",
        variable.display_name,
        variable.kind,
        destination,
        variable.contents.len()
    ));
    let connection = connect(interface)?;
    send_file(&connection, &destination, &variable.contents)?;
    Ok(format!(
        "{} ({} bytes)",
        variable.display_name,
        variable.contents.len()
    ))
}

pub fn probe_directory(interface: Option<u8>) -> Result<std::path::PathBuf, String> {
    TRACE_USB.store(true, Ordering::Relaxed);
    let payload = request_resource("res?name=directory", interface, true, true)?;
    let directory = std::env::temp_dir().join("ti-screen-kirigami");
    std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    let path = directory.join("directory.bin");
    std::fs::write(&path, payload).map_err(|error| error.to_string())?;
    Ok(path)
}

pub fn probe_uri(uri: &str, interface: Option<u8>) -> Result<std::path::PathBuf, String> {
    if !uri.starts_with("hh01/")
        || uri.contains("..")
        || !uri.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(byte, b' ' | b'/' | b'?' | b'=' | b'&' | b'-' | b'_' | b'.')
        })
    {
        return Err("probe URI must be a safe hh01 read resource".into());
    }
    if !["hh01/sys/", "hh01/inf/", "hh01/dbg/", "hh01/xfr/"]
        .iter()
        .any(|prefix| uri.starts_with(prefix))
    {
        return Err("probe URI must use a read-only sys, inf, dbg, or xfr namespace".into());
    }

    TRACE_USB.store(true, Ordering::Relaxed);
    let payload = request_uri(uri, interface, true, true)?;
    let directory = std::env::temp_dir().join("ti-screen-kirigami");
    std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    let filename = uri
        .bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_') {
                char::from(byte)
            } else {
                '_'
            }
        })
        .collect::<String>();
    let path = directory.join(format!("probe-{filename}.bin"));
    std::fs::write(&path, payload).map_err(|error| error.to_string())?;
    Ok(path)
}

pub fn probe_variable(
    name: &str,
    kind: u8,
    interface: Option<u8>,
) -> Result<std::path::PathBuf, String> {
    TRACE_USB.store(true, Ordering::Relaxed);
    let wire_name = crate::ti_file::evo_wire_name(name)?;
    let uri = format!("hh01/xfr/var?name={wire_name}&type={kind}");
    let payload = request_uri(&uri, interface, true, true)?;
    let directory = std::env::temp_dir().join("ti-screen-kirigami");
    std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    let path = directory.join(format!("{name}-{kind}.bin"));
    std::fs::write(&path, payload).map_err(|error| error.to_string())?;
    Ok(path)
}

fn request_resource(
    resource: &str,
    interface: Option<u8>,
    send_break: bool,
    send_attributes: bool,
) -> Result<Vec<u8>, String> {
    request_uri(
        &format!("hh01/inf/{resource}"),
        interface,
        send_break,
        send_attributes,
    )
}

fn request_uri(
    uri: &str,
    interface: Option<u8>,
    send_break: bool,
    send_attributes: bool,
) -> Result<Vec<u8>, String> {
    let connection = connect(interface)?;
    // The XFR get layer wraps the already-qualified resource URI in another
    // device-qualified `get` command. This is the actual Kermit filename.
    let command = format!("hh01/get/{uri}");
    let mut stream = PacketStream::default();
    let first_response = send_command(
        &connection,
        &mut stream,
        &command,
        send_break,
        send_attributes,
    )?;
    receive_resource(&connection, &mut stream, first_response)
}

fn connect(requested_interface: Option<u8>) -> Result<Connection, String> {
    let context = Context::new().map_err(|error| error.to_string())?;
    let devices = context.devices().map_err(|error| error.to_string())?;
    let mut permission_error = false;

    for device in devices.iter() {
        let descriptor = device
            .device_descriptor()
            .map_err(|error| error.to_string())?;
        if descriptor.vendor_id() != TI_VENDOR_ID || descriptor.product_id() != TI_84_EVO_PRODUCT_ID
        {
            continue;
        }
        for (interface, endpoint_in, endpoint_out, class) in bulk_interfaces(&device)? {
            if requested_interface.is_some_and(|requested| requested != interface) {
                continue;
            }
            let handle = match device.open() {
                Ok(handle) => handle,
                Err(rusb::Error::Access) => {
                    permission_error = true;
                    continue;
                }
                Err(_) => continue,
            };
            if handle.kernel_driver_active(interface).unwrap_or(false) {
                let _ = handle.detach_kernel_driver(interface);
            }
            if handle.claim_interface(interface).is_err() {
                continue;
            }
            trace(format_args!(
                "opened {:04x}:{:04x}, interface {interface} class {class:#04x}, OUT {endpoint_out:#04x}, IN {endpoint_in:#04x}",
                descriptor.vendor_id(),
                descriptor.product_id()
            ));
            return Ok(Connection {
                handle,
                interface,
                endpoint_in,
                endpoint_out,
            });
        }
    }

    if permission_error {
        Err("a TI device was found, but USB permission was denied (install a udev rule for vendor 0451)".into())
    } else {
        Err("no TI-84 EVO (USB 0451:e018) bulk interface found".into())
    }
}

fn bulk_interfaces(device: &Device<Context>) -> Result<Vec<(u8, u8, u8, u8)>, String> {
    let config = device
        .active_config_descriptor()
        .or_else(|_| device.config_descriptor(0))
        .map_err(|error| error.to_string())?;
    let mut candidates = Vec::new();
    for interface in config.interfaces() {
        for descriptor in interface.descriptors() {
            let mut endpoint_in = None;
            let mut endpoint_out = None;
            for endpoint in descriptor.endpoint_descriptors() {
                if endpoint.transfer_type() != TransferType::Bulk {
                    continue;
                }
                match endpoint.direction() {
                    Direction::In => endpoint_in = Some(endpoint.address()),
                    Direction::Out => endpoint_out = Some(endpoint.address()),
                }
            }
            if let (Some(input), Some(output)) = (endpoint_in, endpoint_out) {
                candidates.push((
                    descriptor.interface_number(),
                    input,
                    output,
                    descriptor.class_code(),
                ));
            }
        }
    }
    Ok(candidates)
}

#[derive(Default)]
struct PacketStream {
    buffered: Vec<u8>,
}

fn send_command(
    connection: &Connection,
    stream: &mut PacketStream,
    command: &str,
    send_break: bool,
    send_attributes: bool,
) -> Result<Option<Packet>, String> {
    // The dump's XFR layer performs K_SEND with the command URI as its file
    // name and `h` as a one-byte request body. At the B packet, the calculator
    // moves directly into a separate send transaction containing the resource.
    send_packet(connection, &kermit::sender_init(0))?;
    expect_ack(connection, stream, 0)?;

    send_packet(
        connection,
        &kermit::encode_packet(1, b'F', command.as_bytes()),
    )?;
    expect_ack(connection, stream, 1)?;

    let mut sequence = 2;
    if send_attributes {
        // TI Connect's GET transaction uploads a one-byte request body before
        // the calculator starts the response transaction.
        let attributes = kermit::file_attributes(1);
        send_packet(
            connection,
            &kermit::encode_packet(sequence, b'A', &attributes),
        )?;
        expect_ack(connection, stream, sequence)?;
        sequence += 1;
    }

    let request_body = kermit::encode_data_chunks(b"h", 90)
        .into_iter()
        .next()
        .expect("one-byte GET body always produces a chunk");
    send_packet(
        connection,
        &kermit::encode_packet(sequence, b'D', &request_body),
    )?;
    expect_ack(connection, stream, sequence)?;
    sequence += 1;

    send_packet(connection, &kermit::encode_packet(sequence, b'Z', &[]))?;
    expect_ack(connection, stream, sequence)?;
    sequence += 1;

    if send_break {
        send_packet(connection, &kermit::encode_packet(sequence, b'B', &[]))?;
        loop {
            let packet = read_packet(connection, stream)?;
            match packet.kind {
                b'Y' if packet.sequence == sequence => break,
                // Some firmware transitions directly into the response
                // transaction instead of ACKing the command transaction's B.
                b'S' => return Ok(Some(packet)),
                b'N' => return Err(format!("calculator rejected Kermit packet {sequence}")),
                b'E' => {
                    let message = kermit::decode_data(&packet.data).unwrap_or(packet.data);
                    return Err(
                        kermit::Error::Remote(String::from_utf8_lossy(&message).into()).to_string(),
                    );
                }
                _ => continue,
            }
        }
    }
    Ok(None)
}

fn expect_ack(
    connection: &Connection,
    stream: &mut PacketStream,
    sequence: u8,
) -> Result<(), String> {
    loop {
        let packet = read_packet(connection, stream)?;
        match packet.kind {
            b'Y' if packet.sequence == sequence => return Ok(()),
            b'N' => return Err(format!("calculator rejected Kermit packet {sequence}")),
            b'E' => {
                let message = kermit::decode_data(&packet.data).unwrap_or(packet.data);
                return Err(
                    kermit::Error::Remote(String::from_utf8_lossy(&message).into()).to_string(),
                );
            }
            _ => continue,
        }
    }
}

fn send_file(connection: &Connection, destination: &str, contents: &[u8]) -> Result<(), String> {
    let mut stream = PacketStream::default();
    send_packet(connection, &kermit::sender_init(0))?;
    expect_ack(connection, &mut stream, 0)
        .map_err(|error| format!("during upload initialization: {error}"))?;

    send_packet(
        connection,
        &kermit::encode_packet(1, b'F', destination.as_bytes()),
    )?;
    expect_ack(connection, &mut stream, 1)
        .map_err(|error| format!("while opening the EVO variable: {error}"))?;

    let attributes = kermit::file_attributes(contents.len());
    send_packet(connection, &kermit::encode_packet(2, b'A', &attributes))?;
    expect_ack(connection, &mut stream, 2)
        .map_err(|error| format!("while sending file attributes: {error}"))?;

    let mut sequence = 3u8;
    for (chunk_index, chunk) in kermit::encode_data_chunks(contents, 90)
        .into_iter()
        .enumerate()
    {
        trace(format_args!(
            "upload data chunk {chunk_index}, sequence {sequence}, encoded {} bytes",
            chunk.len()
        ));
        send_packet(connection, &kermit::encode_packet(sequence, b'D', &chunk))?;
        expect_ack(connection, &mut stream, sequence)
            .map_err(|error| format!("while sending file data chunk {chunk_index}: {error}"))?;
        sequence = (sequence + 1) & 0x3f;
    }

    send_packet(connection, &kermit::encode_packet(sequence, b'Z', &[]))?;
    expect_ack(connection, &mut stream, sequence)
        .map_err(|error| format!("while committing the EVO variable: {error}"))?;
    sequence = (sequence + 1) & 0x3f;
    send_packet(connection, &kermit::encode_packet(sequence, b'B', &[]))?;
    expect_ack(connection, &mut stream, sequence)
        .map_err(|error| format!("while closing the upload session: {error}"))
}

fn receive_resource(
    connection: &Connection,
    stream: &mut PacketStream,
    first_packet: Option<Packet>,
) -> Result<Vec<u8>, String> {
    let deadline = Instant::now() + CAPTURE_TIMEOUT;
    let mut contents = Vec::new();
    let mut saw_file = false;
    let mut previous_packet = None;

    if let Some(packet) = first_packet {
        handle_packet(connection, &packet, &mut contents, &mut saw_file)?;
        previous_packet = Some((packet.kind, packet.sequence));
    }

    while Instant::now() < deadline {
        let packet = match read_packet(connection, stream) {
            Ok(packet) => packet,
            Err(error) if error.contains("timed out") => continue,
            Err(error) => return Err(error),
        };
        if previous_packet == Some((packet.kind, packet.sequence)) {
            // A lost or delayed ACK makes the sender retransmit. ACK it again,
            // but never append the same data packet twice.
            send_ack(connection, packet.sequence, packet.kind == b'S')?;
            continue;
        }
        handle_packet(connection, &packet, &mut contents, &mut saw_file)?;
        if packet.kind == b'B' {
            if contents.is_empty() {
                return Err("calculator completed the transfer without screen data".into());
            }
            return Ok(contents);
        }
        previous_packet = Some((packet.kind, packet.sequence));
    }
    Err("calculator did not complete the capture within 15 seconds".into())
}

fn read_packet(connection: &Connection, stream: &mut PacketStream) -> Result<Packet, String> {
    let deadline = Instant::now() + CAPTURE_TIMEOUT;
    let mut usb_buffer = [0u8; 16_384];
    loop {
        match kermit::parse_packet(&stream.buffered) {
            Ok((packet, used)) => {
                stream.buffered.drain(..used);
                trace(format_args!(
                    "RX type={} seq={} data={} bytes",
                    char::from(packet.kind),
                    packet.sequence,
                    packet.data.len()
                ));
                return Ok(packet);
            }
            Err(kermit::Error::Incomplete) => {}
            Err(error) => return Err(error.to_string()),
        }
        if Instant::now() >= deadline {
            return Err("timed out waiting for a Kermit packet".into());
        }
        match connection
            .handle
            .read_bulk(connection.endpoint_in, &mut usb_buffer, IO_TIMEOUT)
        {
            Ok(length) => stream.buffered.extend_from_slice(&usb_buffer[..length]),
            Err(rusb::Error::Timeout) => continue,
            Err(error) => return Err(format!("USB read failed: {error}")),
        }
    }
}

fn handle_packet(
    connection: &Connection,
    packet: &Packet,
    contents: &mut Vec<u8>,
    saw_file: &mut bool,
) -> Result<(), String> {
    match packet.kind {
        b'S' => send_ack(connection, packet.sequence, true)?,
        b'F' => {
            *saw_file = true;
            send_ack(connection, packet.sequence, false)?;
        }
        b'A' => {
            if TRACE_USB.load(Ordering::Relaxed) {
                eprintln!("[usb-probe] RX attributes {:02x?}", packet.data);
            }
            send_ack(connection, packet.sequence, false)?;
        }
        b'D' => {
            if !*saw_file {
                return Err("calculator sent data before a file header".into());
            }
            if TRACE_USB.load(Ordering::Relaxed) {
                let preview = &packet.data[..packet.data.len().min(48)];
                eprintln!("[usb-probe] RX encoded data prefix {preview:02x?}");
            }
            contents.extend(kermit::decode_data(&packet.data).map_err(|error| error.to_string())?);
            send_ack(connection, packet.sequence, false)?;
        }
        b'Z' | b'B' => send_ack(connection, packet.sequence, false)?,
        b'E' => {
            let message =
                String::from_utf8_lossy(&kermit::decode_data(&packet.data).unwrap_or_default())
                    .into_owned();
            return Err(kermit::Error::Remote(message).to_string());
        }
        _ => send_ack(connection, packet.sequence, false)?,
    }
    Ok(())
}

fn send_ack(connection: &Connection, sequence: u8, init: bool) -> Result<(), String> {
    let ack = kermit::acknowledgement(sequence, init);
    send_packet(connection, &ack)
}

fn send_packet(connection: &Connection, packet: &[u8]) -> Result<(), String> {
    if let Ok((decoded, _)) = kermit::parse_packet(packet) {
        trace(format_args!(
            "TX type={} seq={} data={} bytes",
            char::from(decoded.kind),
            decoded.sequence,
            decoded.data.len()
        ));
    } else {
        trace(format_args!("TX unparsed packet, {} bytes", packet.len()));
    }
    let written = connection
        .handle
        .write_bulk(connection.endpoint_out, packet, IO_TIMEOUT)
        .map_err(|error| format!("could not write Kermit packet: {error}"))?;
    if written != packet.len() {
        return Err(format!(
            "short USB write: wrote {written} of {} bytes",
            packet.len()
        ));
    }
    Ok(())
}

fn trace(message: std::fmt::Arguments<'_>) {
    if TRACE_USB.load(Ordering::Relaxed) {
        eprintln!("[usb-probe] {message}");
    }
}
