mod backend;
mod directory;
mod kermit;
mod screen;
mod ti_file;
mod usb;

use cxx_qt::casting::Upcast;
use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QQmlEngine, QUrl};
use std::pin::Pin;

fn main() {
    let arguments: Vec<String> = std::env::args().collect();
    if let Some(uri) = arguments
        .windows(2)
        .find(|pair| pair[0] == "--probe-uri")
        .map(|pair| &pair[1])
    {
        match usb::probe_uri(uri) {
            Ok(path) => {
                println!("Resource response saved to {}", path.display());
                std::process::exit(0);
            }
            Err(error) => {
                eprintln!("Resource probe failed: {error}");
                std::process::exit(1);
            }
        }
    }
    if let Some(name) = arguments
        .windows(2)
        .find(|pair| pair[0] == "--probe-delete")
        .map(|pair| &pair[1])
    {
        let kind = arguments
            .windows(2)
            .find(|pair| pair[0] == "--type")
            .and_then(|pair| pair[1].parse::<u8>().ok())
            .unwrap_or(15);
        match usb::probe_delete_variable(name, kind) {
            Ok(()) => {
                println!("Deleted {name} (type {kind}) successfully");
                std::process::exit(0);
            }
            Err(error) => {
                eprintln!("Delete probe failed: {error}");
                std::process::exit(1);
            }
        }
    }
    if let Some(name) = arguments
        .windows(2)
        .find(|pair| pair[0] == "--probe-var")
        .map(|pair| &pair[1])
    {
        let kind = arguments
            .windows(2)
            .find(|pair| pair[0] == "--type")
            .and_then(|pair| pair[1].parse::<u8>().ok())
            .unwrap_or(15);
        match usb::probe_variable(name, kind) {
            Ok(path) => {
                println!("Variable response saved to {}", path.display());
                std::process::exit(0);
            }
            Err(error) => {
                eprintln!("Variable probe failed: {error}");
                std::process::exit(1);
            }
        }
    }
    if let Some(path) = arguments
        .windows(2)
        .find(|pair| pair[0] == "--probe-send")
        .map(|pair| &pair[1])
    {
        match usb::probe_send_host_file(std::path::Path::new(path)) {
            Ok(description) => {
                println!("File sent successfully: {description}");
                std::process::exit(0);
            }
            Err(error) => {
                eprintln!("File send probe failed: {error}");
                std::process::exit(1);
            }
        }
    }
    if let Some(path) = arguments
        .windows(2)
        .find(|pair| pair[0] == "--decode-files")
        .map(|pair| &pair[1])
    {
        match std::fs::read(path)
            .map_err(|error| error.to_string())
            .and_then(|payload| directory::format_listing(&payload))
        {
            Ok(listing) => {
                println!("{listing}");
                std::process::exit(0);
            }
            Err(error) => {
                eprintln!("Could not decode directory: {error}");
                std::process::exit(1);
            }
        }
    }
    if arguments.iter().any(|argument| argument == "--probe-files") {
        match usb::probe_directory() {
            Ok(path) => {
                println!("Directory response saved to {}", path.display());
                match std::fs::read(&path)
                    .map_err(|error| error.to_string())
                    .and_then(|payload| directory::format_listing(&payload))
                {
                    Ok(listing) => println!("\n{listing}"),
                    Err(error) => eprintln!("Could not decode directory: {error}"),
                }
                std::process::exit(0);
            }
            Err(error) => {
                eprintln!("Directory probe failed: {error}");
                std::process::exit(1);
            }
        }
    }
    if arguments.iter().any(|argument| argument == "--probe") {
        let send_break = !arguments.iter().any(|argument| argument == "--no-break");
        let send_attributes = !arguments
            .iter()
            .any(|argument| argument == "--no-attributes");
        match usb::probe_screen(false, send_break, send_attributes) {
            Ok(path) => {
                println!("Capture saved to {}", path.display());
                std::process::exit(0);
            }
            Err(error) => {
                eprintln!("Probe failed: {error}");
                std::process::exit(1);
            }
        }
    }

    let mut app = QGuiApplication::new();
    let mut engine = QQmlApplicationEngine::new();

    if let Some(engine) = engine.as_mut() {
        engine.load(&QUrl::from("qrc:/qt/qml/org/example/tiscreen/qml/Main.qml"));
    }
    if let Some(engine) = engine.as_mut() {
        let engine: Pin<&mut QQmlEngine> = engine.upcast_pin();
        engine.on_quit(|_| {}).release();
    }
    if let Some(app) = app.as_mut() {
        app.exec();
    }
}
