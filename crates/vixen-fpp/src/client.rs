//! `FppClient`: the requests of F76.

use crate::http::{self, Body, Cancel, Response, Target};
use crate::models::{self, Playlist, SystemInfo};
use crate::FppError;
use serde_json::Value;
use std::fs::File;
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Options {
    pub timeout: Duration,
    pub upload_timeout: Duration,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            timeout: Duration::from_secs(30),
            upload_timeout: Duration::from_secs(600),
        }
    }
}

/// What an upload the device accepted has to say for itself: text for the log, never a failure.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct UploadOutcome {
    pub warning: Option<String>,
}

pub struct FppClient {
    target: Target,
    /// Whatever path the base address carries, without a trailing slash.
    root: String,
    options: Options,
}

impl FppClient {
    /// `base_url` is an HTTP(S) host URL, with or without a base path or trailing slash.
    pub fn new(base_url: &str, options: Options) -> Result<FppClient, FppError> {
        let bad = || FppError::Argument(format!("'{base_url}' isn't an HTTP(S) address."));
        let rest = base_url.trim();
        let (scheme, rest) = rest.split_once("://").ok_or_else(bad)?;
        let https = scheme.eq_ignore_ascii_case("https");
        if !https && !scheme.eq_ignore_ascii_case("http") {
            return Err(bad());
        }
        let (authority, path) = rest.split_once('/').map_or((rest, ""), |(authority, path)| (authority, path));
        let (host, port) = if let Some(inner) = authority.strip_prefix('[') {
            let (host, after) = inner.split_once(']').ok_or_else(bad)?;
            if host.parse::<std::net::Ipv6Addr>().is_err() || (!after.is_empty() && !after.starts_with(':')) {
                return Err(bad());
            }
            (host, after.strip_prefix(':'))
        } else {
            authority.split_once(':').map_or((authority, None), |(host, port)| (host, Some(port)))
        };
        let port = match port {
            None => {
                if https {
                    443
                } else {
                    80
                }
            }
            Some(text) => text.parse::<u16>().map_err(|_| bad())?,
        };
        if host.is_empty() || port == 0 || rest.chars().any(char::is_control) || authority.contains(['@', '?', '#', ' ', '\\']) || path.contains(['?', '#']) {
            return Err(bad());
        }
        let path = path.trim_end_matches('/');
        let root = if path.is_empty() { String::new() } else { format!("/{path}") };
        Ok(FppClient {
            target: Target {
                cookies: http::CookieJar::new()?,
                loopback_only: std::env::var_os("VIXEN_UI_TEST").is_some(),
                https,
                host: host.to_string(),
                port,
            },
            root,
            options,
        })
    }

    /// The wizard's `http://{host}/`, for a host that passed [`crate::is_host_address_valid`]. A host that carries a port, which the
    /// wizard's own box refuses, is taken as written.
    pub fn for_host(host: &str) -> Result<FppClient, FppError> {
        let host = host.trim();
        if std::env::var_os("VIXEN_UI_TEST").is_some() {
            let port = std::env::var("VIXEN_TEST_FPP_PORT").ok();
            return FppClient::new(&crate::host::ui_test_address(host, port.as_deref())?, Options::default());
        }
        let address = if host.parse::<std::net::Ipv6Addr>().is_ok() {
            format!("http://[{host}]/")
        } else {
            format!("http://{host}/")
        };
        FppClient::new(&address, Options::default())
    }

    fn api(&self, relative: &str) -> String {
        format!("{}/api/{relative}", self.root)
    }

    fn get(&self, relative: &str, cancel: &Cancel) -> Result<Value, FppError> {
        let response = http::send(&self.target, "GET", &self.api(relative), Body::None, self.options.timeout, cancel)?;
        if !response.is_success() {
            return Err(status(format!("Request to '{relative}' failed with HTTP {}.", response.status), &response));
        }
        serde_json::from_slice(&response.body).map_err(|e| FppError::Response(format!("The answer to '{relative}' isn't valid JSON: {e}")))
    }

    fn read<T>(&self, relative: &str, cancel: &Cancel, parse: impl FnOnce(&Value) -> Result<T, String>) -> Result<T, FppError> {
        let value = self.get(relative, cancel)?;
        parse(&value).map_err(|why| FppError::Response(format!("The answer to '{relative}' isn't what was expected: {why}")))
    }

    /// `None` when the device answers with a JSON null.
    pub fn system_info(&self, cancel: &Cancel) -> Result<Option<SystemInfo>, FppError> {
        self.read("system/info", cancel, models::system_info)
    }

    pub fn sequences(&self, cancel: &Cancel) -> Result<Vec<String>, FppError> {
        self.read("sequence", cancel, |value| models::names(value, "The list of sequences"))
    }

    pub fn playlists(&self, cancel: &Cancel) -> Result<Vec<String>, FppError> {
        self.read("playlists", cancel, |value| models::names(value, "The list of playlists"))
    }

    pub fn playlist(&self, name: &str, cancel: &Cancel) -> Result<Playlist, FppError> {
        self.read(&format!("playlist/{}", escape_data_string(name)), cancel, models::playlist)
    }

    pub fn playlist_sequences(&self, name: &str, cancel: &Cancel) -> Result<Vec<String>, FppError> {
        Ok(self.playlist(name, cancel)?.sequences())
    }

    fn post_file(&self, path: &str, source: &Path, cancel: &Cancel) -> Result<Response, FppError> {
        let mut file = File::open(source).map_err(|e| FppError::Io(format!("Could not open '{}': {e}", source.display())))?;
        let length = file.metadata().map_err(|e| FppError::Io(e.to_string()))?.len();
        http::send(&self.target, "POST", path, Body::File(&mut file, length), self.options.upload_timeout, cancel)
    }

    /// `POST file/{dir}/{name}` with the file at `source` as the body.
    pub fn upload_file(&self, directory: &str, filename: &str, source: &Path, cancel: &Cancel) -> Result<UploadOutcome, FppError> {
        not_blank(directory, "dirName")?;
        not_blank(filename, "filename")?;
        let response = self.post_file(
            &self.api(&format!("file/{}/{}", escape_data_string(directory), escape_data_string(filename))),
            source,
            cancel,
        )?;
        if !response.is_success() {
            return Err(status(format!("Upload of '{filename}' to '{directory}' failed with HTTP {}.", response.status), &response));
        }
        // The device has the file by now, so an answer that reads oddly is only worth a line in the log.
        let warning = match serde_json::from_slice::<Value>(&response.body) {
            Ok(Value::Object(object)) => match models::field(&object, "status") {
                None => None,
                Some(Value::String(text)) if text == "OK" || text.is_empty() => None,
                Some(Value::String(text)) => Some(format!("Upload of {filename} returned non-OK status: {text}")),
                Some(Value::Null) => Some(format!("Upload of {filename} returned non-OK status: ")),
                Some(_) => Some(format!("Could not parse upload response body for {filename}; treating as success")),
            },
            Ok(Value::Null) => None,
            _ => Some(format!("Could not parse upload response body for {filename}; treating as success")),
        };
        Ok(UploadOutcome { warning })
    }

    pub fn upload_sequence(&self, filename: &str, source: &Path, cancel: &Cancel) -> Result<UploadOutcome, FppError> {
        self.upload_file("sequences", filename, source, cancel)
    }

    pub fn upload_music(&self, filename: &str, source: &Path, cancel: &Cancel) -> Result<UploadOutcome, FppError> {
        self.upload_file("music", filename, source, cancel)
    }

    pub fn upload_video(&self, filename: &str, source: &Path, cancel: &Cancel) -> Result<UploadOutcome, FppError> {
        self.upload_file("videos", filename, source, cancel)
    }

    pub fn upload_esp_sequence(&self, filename: &str, source: &Path, cancel: &Cancel) -> Result<(), FppError> {
        self.upload_esp(filename, source, "sequence", cancel)
    }

    pub fn upload_esp_archive(&self, filename: &str, source: &Path, cancel: &Cancel) -> Result<(), FppError> {
        self.upload_esp(filename, source, "archive", cancel)
    }

    /// An ESPixelStick takes files at its own root, not under `api/`, and answers with what it stored.
    fn upload_esp(&self, filename: &str, source: &Path, kind: &str, cancel: &Cancel) -> Result<(), FppError> {
        not_blank(filename, "filename")?;
        if filename.contains(['/', '\\']) {
            return Err(FppError::Argument("The filename must not contain path separators.".into()));
        }
        let response = self.post_file(&format!("/fpp?path=uploadFile&filename={}", escape_data_string(filename)), source, cancel)?;
        if !response.is_success() {
            return Err(status(
                format!("ESPixelStick {kind} upload of '{filename}' failed with HTTP {}.", response.status),
                &response,
            ));
        }
        match serde_json::from_slice::<Value>(&response.body) {
            Ok(Value::Object(object)) if object.get("Name").and_then(Value::as_str) == Some(filename) => Ok(()),
            Ok(_) => Err(FppError::Response(format!("ESPixelStick {kind} upload of '{filename}' returned unexpected file metadata."))),
            Err(_) => Err(FppError::Response(format!("ESPixelStick {kind} upload of '{filename}' returned invalid JSON."))),
        }
    }

    pub fn reboot_esp(&self, cancel: &Cancel) -> Result<(), FppError> {
        let response = http::send(&self.target, "POST", "/X6", Body::Empty, self.options.timeout, cancel)?;
        if !response.is_success() {
            return Err(status(format!("ESPixelStick reboot request failed with HTTP {}.", response.status), &response));
        }
        match serde_json::from_slice::<Value>(&response.body) {
            Ok(Value::Object(object)) if object.get("status").and_then(Value::as_str) == Some("Rebooting") => Ok(()),
            Ok(_) => Err(FppError::Response("ESPixelStick reboot request returned an unexpected acknowledgement.".into())),
            Err(_) => Err(FppError::Response("ESPixelStick reboot request returned invalid JSON.".into())),
        }
    }

    /// A file that isn't there to rename (404) is not a failure.
    pub fn rename_file(&self, directory: &str, source: &str, destination: &str, cancel: &Cancel) -> Result<(), FppError> {
        not_blank(directory, "dirName")?;
        not_blank(source, "source")?;
        not_blank(destination, "dest")?;
        let path = self.api(&format!(
            "file/{}/rename/{}/{}",
            escape_data_string(directory),
            escape_data_string(source),
            escape_data_string(destination)
        ));
        let response = http::send(&self.target, "POST", &path, Body::Empty, self.options.timeout, cancel)?;
        if response.status == 404 || response.is_success() {
            return Ok(());
        }
        Err(status(format!("Rename of '{source}' in '{directory}' failed with HTTP {}.", response.status), &response))
    }

    pub fn restart_fppd(&self, quick: bool, cancel: &Cancel) -> Result<(), FppError> {
        let relative = if quick { "system/fppd/restart?quick=1" } else { "system/fppd/restart" };
        let response = http::send(&self.target, "GET", &self.api(relative), Body::None, self.options.timeout, cancel)?;
        if response.is_success() {
            return Ok(());
        }
        Err(status(format!("FPPD restart failed with HTTP {}.", response.status), &response))
    }
}

fn status(message: String, response: &Response) -> FppError {
    FppError::Status { message, status: response.status }
}

fn not_blank(value: &str, name: &str) -> Result<(), FppError> {
    if value.trim().is_empty() {
        return Err(FppError::Argument(format!(
            "The value cannot be an empty string or composed entirely of whitespace. (Parameter '{name}')"
        )));
    }
    Ok(())
}

/// .NET's `Uri.EscapeDataString`: everything but letters, digits and `-._~`, as UTF-8.
pub fn escape_data_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}
