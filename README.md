# TI-84 Evo Tools

A native desktop application for working with a TI-84 Evo over USB. It can
capture the calculator screen, inspect and delete device files, and send
TI-BASIC or Python programs. The interface is built with standard Qt Quick Controls;
USB communication, Kermit framing, directory parsing, file conversion, and
screen decoding are implemented in Rust.

This is an experimental, reverse-engineered project. It is not affiliated with
or endorsed by Texas Instruments.

## Current features

- Capture the 320 x 240 calculator screen with or without the cursor.
- Automatically repeat captures at intervals from `0.01` to `300` seconds.
  The controls step by `0.25` seconds and also accept a typed value. Very short
  intervals can make the calculator or USB connection unresponsive, so the UI
  displays a warning below `0.25` seconds and prevents overlapping requests.
- List calculator variables with their display name, token name, size, type,
  version, and RAM/archive location.
- Delete one selected variable after confirmation without clearing all RAM.
- Send native TI-84 Evo `.8xp2` programs while preserving their memory target.
- Convert UTF-8 `.py` source into a native Evo Python program and send it to
  RAM. Python program names must contain one to eight ASCII letters.
- Run lower-level USB and resource probes from the command line.
- Decode a previously saved raw directory response without a calculator.

## Supported hardware and files

The implementation targets the TI-84 Evo's CDC serial port (USB vendor/product
ID `0451:e018`) and exchanges Kermit packets at 115200 baud, 8 data bits, no
parity, and 1 stop bit. It picks the calculator's serial port by USB identity
when available, then confirms it with a Kermit initialization ACK.

| Item | Support |
| --- | --- |
| TI-84 Evo screen capture | Yes |
| TI-84 Evo directory listing and individual deletion | Yes |
| Native Evo `.8xp2` programs | Yes |
| UTF-8 `.py` source | Converted to an Evo Python variable (type 15) |
| Legacy `.8xp` files | No; they are rejected before USB is opened |
| Other TI calculator models | Not tested or supported |
| Raw flash/ROM dumping | No; the exposed Evo protocol has no known dump operation |
| OS installation or restoration | No |

Malformed and unsupported files are rejected before a transfer begins. Python
source must be valid UTF-8, cannot contain a NUL byte, and must fit the native
format's 24-bit source-length field.

## Requirements

- A Rust toolchain with support for Rust 2024 edition
- A C++ compiler, CMake, and `pkg-config`
- Qt 6 development tools and libraries, including Qt Quick, QML, Controls,
  Dialogs, Layouts, and Network
- Access to the calculator's CDC serial device
- `qmake6` (the exact path varies by distribution)

The package names differ by Linux distribution. Make sure the runtime QML
modules are installed as well as the Qt development headers.

## Serial permissions and port selection

On Linux, the logged-in user must have permission to open the calculator's
serial port. Add the user to the system's serial-device group (often `dialout`)
or configure a udev rule for vendor `0451`, product `e018`, then reconnect the
calculator. On macOS the calculator appears as `/dev/cu.usbmodem*`; on Windows
it appears as a COM port.

If automatic selection finds no matching USB identity, set `EVO_USB_SERIAL` to
the calculator's serial-device path, such as `/dev/cu.usbmodem...` or `COM3`.

## Build and run

From the repository root:

```sh
QMAKE=/usr/bin/qmake6 cargo build
QMAKE=/usr/bin/qmake6 cargo run
```

If `qmake6` is already on `PATH`, the `QMAKE` assignment may not be necessary.
Running the binary without arguments starts the graphical application:

```sh
target/debug/ti-screen-kirigami
```

Connect and wake the calculator before starting an operation. Only one USB
operation is run at a time.

## Graphical application

The navigation drawer and bottom bar expose four pages:

### Main Menu

Provides shortcuts to screen capture, device-file management, and file
transfer.

### Screen Capture

`Capture` requests a fresh screen image. Enable `Include cursor` when the
calculator cursor should appear. Normal captures prefer `hh01/sys/screen`,
which remains available while a Python program is running; cursor-inclusive
captures prefer the screencapture information resource. Each mode falls back
to the other endpoint if its preferred endpoint acknowledges the request but
does not return an image.

`Auto capture` repeats the request only while the capture page is visible. The
interval field accepts values from `0.01` through `300.00` seconds. Automatic
captures do not show the full-screen loading spinner, and a new request is
skipped whenever the previous one is still running.

### Device Files

`Refresh` reads the Evo directory without using the optional `gotohome=1`
action, so listing variables does not deliberately change the calculator's
current screen or stop a running program.

Each entry has its own `Delete` action. After confirmation, the application
sends the native delete resource request for that entry's exact token name and
type, then reloads the directory. This is an individual variable deletion; it
does not issue a reset or clear-RAM command.

Deletion is permanent on the calculator. Keep a backup of important programs.

### Send Files

Choose a `.8xp2` or `.py` file and select `Send`. Native `.8xp2` containers are
validated and sent with their embedded variable metadata and archive flag.
Python source is converted to the Evo's native CBOR-based Python container and
sent to RAM as variable type 15.

## Command-line tools

All command-line operations exit when the requested probe finishes. The app
discovers serial ports automatically and verifies candidates with Kermit.

### Capture with packet tracing

```sh
target/debug/ti-screen-kirigami --probe
```

Diagnostic variants can omit the Kermit break packet or file attributes:

```sh
target/debug/ti-screen-kirigami --probe --no-break
target/debug/ti-screen-kirigami --probe --no-attributes
```

Those switches are intended for protocol investigation and may cause the
request to fail.

### List or decode device files

```sh
target/debug/ti-screen-kirigami --probe-files
target/debug/ti-screen-kirigami --decode-files /tmp/ti-screen-kirigami/directory.bin
```

The first command fetches and decodes the directory. The second parses a saved
directory response without accessing USB.

### Receive one variable

```sh
target/debug/ti-screen-kirigami --probe-var GAME --type 15
```

This is a diagnostic receive operation. It stores the calculator's raw
response; it does not currently provide a polished GUI download/export flow.

### Send a program

```sh
target/debug/ti-screen-kirigami --probe-send ./HELLO.py
target/debug/ti-screen-kirigami --probe-send ./GAME.8xp2
```

### Delete one variable

```sh
target/debug/ti-screen-kirigami --probe-delete GAME --type 15
```

This command is destructive. The type defaults to 15 when `--type` is omitted.

### Read a resource

```sh
target/debug/ti-screen-kirigami --probe-uri hh01/sys/attributes
```

For safety, arbitrary probes are limited to valid resource characters and the
known `hh01/sys/`, `hh01/inf/`, `hh01/dbg/`, and `hh01/xfr/` namespaces. These
are still low-level diagnostic requests; use only resources whose behavior is
understood.

## Generated files

Diagnostic output is written under `/tmp/ti-screen-kirigami`:

| Path | Contents |
| --- | --- |
| `/tmp/ti-screen-kirigami/screen.png` | Most recently decoded capture |
| `/tmp/ti-screen-kirigami/last-capture.bin` | Complete raw response from the most recent capture |
| `/tmp/ti-screen-kirigami/directory.bin` | Raw directory response produced by the file probe |

Additional raw probe responses use descriptive names in the same temporary
directory. Files below `/tmp` are not permanent and may be removed at reboot.

## Protocol implementation

The application implements the calculator's resource exchange over Kermit
on its CDC serial port rather than using the browser APIs from TI Connect. It supports short and long
Kermit packets, control/repeat quoting, checksum validation, file attributes,
data chunking, acknowledgements, retransmissions, and the separate response
transaction that begins after the request's final `B` packet.

Resource reads send the same one-byte request body and file attributes observed
in TI Connect. Screen responses are decoded from their metadata envelope and
may contain a PNG, 320 x 240 little-endian RGB565 pixels, or 8-bit grayscale
pixels. Directory metadata is decoded with the project's CBOR parser.

Protocol details were inferred from TI Connect's browser client. A local copy
of that JavaScript is useful for research but is TI proprietary code and is
not part of this repository. The graphical application itself is implemented
by the Rust and QML sources.

## Known limitations

- Hardware support is currently limited to the TI-84 Evo.
- USB and resource behavior was inferred from observed TI Connect traffic and
  can change with a different calculator firmware version.
- The UI can send and delete variables but does not yet export a selected
  variable to a host file.
- The application cannot dump raw flash, back up the complete calculator, or
  install/restore an OS.
- Extremely fast automatic capture intervals can time out, reduce calculator
  responsiveness, or destabilize the USB session.
- There is no packaged release or installer yet; run it from a source build.

## Tests and validation

Run the Rust test suite with:

```sh
QMAKE=/usr/bin/qmake6 cargo test
```

Check the QML when `qmllint` is available:

```sh
qmllint qml/Main.qml
```

Most unit tests do not require a calculator. USB integration and actual
transfers require a connected Evo with suitable permissions.

## Project layout

| Path | Purpose |
| --- | --- |
| `qml/Main.qml` | Qt Quick Controls user interface and auto-capture controls |
| `src/backend.rs` | Asynchronous QML-facing backend |
| `src/usb.rs` | USB discovery and resource operations |
| `src/kermit.rs` | Kermit framing, quoting, transfer, and response handling |
| `src/screen.rs` | Screen payload extraction and PNG conversion |
| `src/directory.rs` | CBOR directory decoder |
| `src/ti_file.rs` | `.8xp2` validation and `.py` conversion |
| `src/main.rs` | Application startup and command-line modes |

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE).
