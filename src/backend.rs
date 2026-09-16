use cxx_qt_lib::QString;
use std::pin::Pin;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, status)]
        #[qproperty(QString, image_url, cxx_name = "imageUrl")]
        #[qproperty(QString, files_text, cxx_name = "filesText")]
        #[qproperty(QString, files_json, cxx_name = "filesJson")]
        #[qproperty(bool, busy)]
        type CaptureBackend = super::CaptureBackendRust;

        #[qinvokable]
        fn capture(self: Pin<&mut Self>, include_cursor: bool);

        #[qinvokable]
        #[cxx_name = "refreshFiles"]
        fn refresh_files(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "uploadFile"]
        fn upload_file(self: Pin<&mut Self>, path: &QString);

        #[qinvokable]
        #[cxx_name = "deleteFile"]
        fn delete_file(
            self: Pin<&mut Self>,
            display_name: &QString,
            wire_name: &QString,
            kind: i32,
        );
    }

    impl cxx_qt::Threading for CaptureBackend {}
}

pub struct CaptureBackendRust {
    status: QString,
    image_url: QString,
    files_text: QString,
    files_json: QString,
    busy: bool,
}

impl Default for CaptureBackendRust {
    fn default() -> Self {
        Self {
            status: QString::from("Connect a TI calculator, then press Capture"),
            image_url: QString::default(),
            files_text: QString::default(),
            files_json: QString::from("[]"),
            busy: false,
        }
    }
}

impl ffi::CaptureBackend {
    pub fn capture(mut self: Pin<&mut Self>, include_cursor: bool) {
        if *self.busy() {
            return;
        }

        self.as_mut().set_busy(true);
        self.as_mut()
            .set_status(QString::from("Contacting calculator…"));

        let qt_thread = cxx_qt::Threading::qt_thread(self.as_ref().get_ref());
        std::thread::spawn(move || {
            let result = crate::usb::capture_screen(include_cursor);
            qt_thread
                .queue(move |mut backend| {
                    match result {
                        Ok(path) => {
                            let url = QUrlPath::from_path(&path);
                            backend.as_mut().set_image_url(QString::from(&url.0));
                            backend
                                .as_mut()
                                .set_status(QString::from("Screen captured"));
                        }
                        Err(error) => {
                            backend
                                .as_mut()
                                .set_status(QString::from(&format!("Capture failed: {error}")));
                        }
                    }
                    backend.as_mut().set_busy(false);
                })
                .ok();
        });
    }

    pub fn refresh_files(mut self: Pin<&mut Self>) {
        if *self.busy() {
            return;
        }

        self.as_mut().set_busy(true);
        self.as_mut()
            .set_status(QString::from("Reading calculator files…"));

        let qt_thread = cxx_qt::Threading::qt_thread(self.as_ref().get_ref());
        std::thread::spawn(move || {
            let result =
                crate::usb::directory().and_then(|payload| crate::directory::listing(&payload));
            qt_thread
                .queue(move |mut backend| {
                    match result {
                        Ok(listing) => {
                            backend
                                .as_mut()
                                .set_files_text(QString::from(&listing.text));
                            backend
                                .as_mut()
                                .set_files_json(QString::from(&listing.files_json));
                            backend
                                .as_mut()
                                .set_status(QString::from("Device files loaded"));
                        }
                        Err(error) => {
                            backend
                                .as_mut()
                                .set_status(QString::from(&format!("File check failed: {error}")));
                        }
                    }
                    backend.as_mut().set_busy(false);
                })
                .ok();
        });
    }

    pub fn upload_file(mut self: Pin<&mut Self>, path: &QString) {
        if *self.busy() {
            return;
        }

        let path = std::path::PathBuf::from(path.to_string());
        self.as_mut().set_busy(true);
        self.as_mut()
            .set_status(QString::from("Sending file to calculator…"));

        let qt_thread = cxx_qt::Threading::qt_thread(self.as_ref().get_ref());
        std::thread::spawn(move || {
            let result = crate::usb::send_host_file(&path);
            qt_thread
                .queue(move |mut backend| {
                    match result {
                        Ok(description) => backend.as_mut().set_status(QString::from(&format!(
                            "File sent successfully: {description}"
                        ))),
                        Err(error) => backend
                            .as_mut()
                            .set_status(QString::from(&format!("File send failed: {error}"))),
                    }
                    backend.as_mut().set_busy(false);
                })
                .ok();
        });
    }

    pub fn delete_file(
        mut self: Pin<&mut Self>,
        display_name: &QString,
        wire_name: &QString,
        kind: i32,
    ) {
        if *self.busy() {
            return;
        }

        let display_name = display_name.to_string();
        let wire_name = wire_name.to_string();
        let kind = match u8::try_from(kind) {
            Ok(kind) => kind,
            Err(_) => {
                self.as_mut()
                    .set_status(QString::from("Delete failed: invalid EVO variable type"));
                return;
            }
        };
        self.as_mut().set_busy(true);
        self.as_mut()
            .set_status(QString::from(&format!("Deleting {display_name}…")));

        let qt_thread = cxx_qt::Threading::qt_thread(self.as_ref().get_ref());
        std::thread::spawn(move || {
            let result = crate::usb::delete_variable(&wire_name, kind).and_then(|()| {
                crate::usb::directory().and_then(|payload| crate::directory::listing(&payload))
            });
            qt_thread
                .queue(move |mut backend| {
                    match result {
                        Ok(listing) => {
                            backend
                                .as_mut()
                                .set_files_text(QString::from(&listing.text));
                            backend
                                .as_mut()
                                .set_files_json(QString::from(&listing.files_json));
                            backend.as_mut().set_status(QString::from(&format!(
                                "Deleted {display_name} from the calculator"
                            )));
                        }
                        Err(error) => backend
                            .as_mut()
                            .set_status(QString::from(&format!("Delete failed: {error}"))),
                    }
                    backend.as_mut().set_busy(false);
                })
                .ok();
        });
    }
}

struct QUrlPath(String);

impl QUrlPath {
    fn from_path(path: &std::path::Path) -> Self {
        let path = path
            .to_string_lossy()
            .replace('%', "%25")
            .replace(' ', "%20");
        Self(format!("file://{path}?v={}", timestamp()))
    }
}

fn timestamp() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis())
}
