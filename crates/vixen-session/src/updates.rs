//! Checking GitHub for a newer Vixen, and fetching release notes. Port of `GitHubUpdateService` and `VixenReleaseVersion`.
//!
//! The transport is a trait, so this runs without a network in tests. Nothing here runs on its own: a check happens only when the user
//! opens Check for Updates or Release Notes.

use crate::logging;
use crate::version::VersionInfo;
use parking_lot::Mutex;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, Instant};

/// `GitHubUpdateService`'s logger, for the warnings it writes.
const LOGGER: &str = "VixenApplication.Updates.GitHubUpdateService";
const FALLBACK_RELEASE_PAGE: &str = "https://github.com/VixenLights/Vixen/releases";
const INSTALLER_SUFFIX: &str = "-setup-64bit.exe";
const DEV_PREFIX: &str = "DevBuild-";
const CACHE_LIFETIME: Duration = Duration::from_secs(5 * 60);

/// The GitHub API's base, which every relative address below joins. What the app's transport uses.
pub const API_BASE: &str = "https://api.github.com/repos/VixenLights/Vixen/";
pub const USER_AGENT: &str = "Vixen-UpdateChecker/1.0";
pub const ACCEPT: &str = "application/vnd.github+json";
pub const API_VERSION_HEADER: &str = "X-GitHub-Api-Version";
pub const API_VERSION: &str = "2026-03-10";
pub const TIMEOUT_SECONDS: u64 = 10;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum UpdateChannel {
    Release,
    Development,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateCheckRequest {
    pub channel: UpdateChannel,
    pub installed_version: String,
    pub include_release_notes: bool,
}

impl UpdateCheckRequest {
    /// What this build asks. Development builds follow their own channel, and go by build number.
    pub fn for_version(version: &VersionInfo, include_release_notes: bool) -> UpdateCheckRequest {
        let (channel, installed_version) = if version.is_dev_build() {
            (UpdateChannel::Development, version.build.to_string())
        } else {
            (UpdateChannel::Release, version.release_version())
        };
        UpdateCheckRequest { channel, installed_version, include_release_notes }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateCheckResult {
    pub latest_version: String,
    pub latest_build_number: Option<i32>,
    pub is_update_available: bool,
    pub release_notes: Option<String>,
    pub release_page_uri: String,
    pub published_at: Option<String>,
    pub installer_download_uri: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReleaseNotesResult {
    pub release_tag: String,
    pub release_notes: String,
    pub release_page_uri: String,
    pub published_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpResponse {
    pub status: u16,
    pub body: String,
    pub rate_limit_remaining: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TransportError {
    TimedOut,
    Failed(String),
}

/// A GET, relative to [`API_BASE`]. Blocking: `UpdateService` is called off the main actor.
pub trait HttpTransport: Send + Sync {
    fn get(&self, relative_uri: &str) -> Result<HttpResponse, TransportError>;
}

/// `VixenReleaseVersion`: `major.minor` or `major.minoruN`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ReleaseVersion {
    pub major: i32,
    pub minor: i32,
    pub update: i32,
}

/// `NumberStyles.None`: digits only.
fn parse_digits(text: &str) -> Option<i32> {
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

pub fn parse_release_version(text: &str) -> Option<ReleaseVersion> {
    if text.trim().is_empty() {
        return None;
    }
    let parts: Vec<&str> = text.split('u').collect();
    if parts.len() > 2 {
        return None;
    }
    let release: Vec<&str> = parts[0].split('.').collect();
    if release.len() != 2 {
        return None;
    }
    let major = parse_digits(release[0])?;
    let minor = parse_digits(release[1])?;
    let update = match parts.get(1) {
        Some(part) => parse_digits(part)?,
        None => 0,
    };
    Some(ReleaseVersion { major, minor, update })
}

/// `TryGetDevelopmentBuildNumber`. The prefix is exact, and `int.TryParse` allows spaces around the number and a sign.
pub fn parse_dev_build_number(tag: &str) -> Option<i32> {
    let number = tag.strip_prefix(DEV_PREFIX)?.trim();
    number.parse::<i32>().ok().filter(|n| *n >= 0)
}

/// `GitHubRelease`, as far as anything reads it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Release {
    tag_name: Option<String>,
    is_draft: bool,
    is_prerelease: bool,
    body: Option<String>,
    html_url: Option<String>,
    published_at: Option<String>,
    assets: Vec<(Option<String>, Option<String>)>,
}

impl Release {
    fn from_json(value: &Value) -> Option<Release> {
        let object = value.as_object()?;
        let text = |key: &str| object.get(key).and_then(Value::as_str).map(str::to_string);
        Some(Release {
            tag_name: text("tag_name"),
            is_draft: object.get("draft").and_then(Value::as_bool).unwrap_or(false),
            is_prerelease: object.get("prerelease").and_then(Value::as_bool).unwrap_or(false),
            body: text("body"),
            html_url: text("html_url"),
            published_at: text("published_at"),
            assets: object
                .get("assets")
                .and_then(Value::as_array)
                .map(|list| {
                    list.iter()
                        .map(|a| {
                            let field = |key: &str| a.get(key).and_then(Value::as_str).map(str::to_string);
                            (field("name"), field("browser_download_url"))
                        })
                        .collect()
                })
                .unwrap_or_default(),
        })
    }

    fn page(&self) -> String {
        self.html_url.clone().unwrap_or_else(|| FALLBACK_RELEASE_PAGE.to_string())
    }

    /// The first asset named as a Windows installer. Case doesn't matter.
    fn installer(&self) -> Option<String> {
        self.assets
            .iter()
            .find(|(name, _)| name.as_deref().is_some_and(|n| n.to_lowercase().ends_with(INSTALLER_SUFFIX)))
            .and_then(|(_, url)| url.clone())
    }
}

struct Cached {
    release: Release,
    expires: Instant,
}

/// `IUpdateService`: either real, with caches, or `UnavailableUpdateService`, which has nothing to say.
pub struct UpdateService {
    transport: Option<Box<dyn HttpTransport>>,
    channels: Mutex<HashMap<UpdateChannel, Cached>>,
    notes: Mutex<HashMap<String, Cached>>,
}

impl UpdateService {
    pub fn github(transport: Box<dyn HttpTransport>) -> UpdateService {
        UpdateService { transport: Some(transport), channels: Mutex::default(), notes: Mutex::default() }
    }

    /// Always `None`. What a UI test gets, so it never reaches the network.
    pub fn unavailable() -> UpdateService {
        UpdateService { transport: None, channels: Mutex::default(), notes: Mutex::default() }
    }

    pub fn check(&self, request: &UpdateCheckRequest) -> Option<UpdateCheckResult> {
        let release = self.release_for(request.channel)?;
        create_result(request, &release)
    }

    /// Prereleases are allowed here, since a development build's notes are one.
    pub fn release_notes(&self, tag: &str) -> Option<ReleaseNotesResult> {
        if tag.trim().is_empty() || self.transport.is_none() {
            return None;
        }
        let mut cache = self.notes.lock();
        if let Some(cached) = cache.get(tag) {
            if cached.expires > Instant::now() {
                return Some(notes_result(&cached.release));
            }
        }
        // A failed request is a missing release too, so it gets this warning after its own, as upstream.
        let release = self
            .get_json(&format!("releases/tags/{}", escape_data_string(tag)), Shape::Object)
            .and_then(|value| Release::from_json(&value))
            .filter(|r| !r.is_draft && r.tag_name.as_deref() == Some(tag));
        let Some(release) = release else {
            // NLog puts a string argument of a named hole in quotes.
            logging::warn(LOGGER, &format!("GitHub returned an invalid Vixen release for tag \"{tag}\"."));
            return None;
        };
        let result = notes_result(&release);
        cache.insert(tag.to_string(), Cached { release, expires: Instant::now() + CACHE_LIFETIME });
        Some(result)
    }

    fn release_for(&self, channel: UpdateChannel) -> Option<Release> {
        let mut cache = self.channels.lock();
        if let Some(cached) = cache.get(&channel) {
            if cached.expires > Instant::now() {
                return Some(cached.release.clone());
            }
        }
        let release = match channel {
            UpdateChannel::Release => self.latest_release(),
            UpdateChannel::Development => self.latest_development_release(),
        }?;
        cache.insert(channel, Cached { release: release.clone(), expires: Instant::now() + CACHE_LIFETIME });
        Some(release)
    }

    fn latest_release(&self) -> Option<Release> {
        // `UnavailableUpdateService` answers nothing and logs nothing.
        self.transport.as_ref()?;
        // A failed request is a missing release too, so it gets this warning after its own, as upstream.
        let release = self.get_json("releases/latest", Shape::Object).and_then(|value| Release::from_json(&value)).filter(|r| {
            !r.is_draft && !r.is_prerelease && r.tag_name.as_deref().and_then(parse_release_version).is_some()
        });
        if release.is_none() {
            logging::warn(LOGGER, "GitHub returned an invalid latest Vixen release.");
        }
        release
    }

    /// The largest build number among a page of five, not the newest date.
    fn latest_development_release(&self) -> Option<Release> {
        let value = self.get_json("releases?per_page=5", Shape::Array)?;
        let list = value.as_array()?;
        let mut best: Option<(i32, Release)> = None;
        for release in list.iter().filter_map(Release::from_json) {
            if release.is_draft || !release.is_prerelease {
                continue;
            }
            let Some(number) = release.tag_name.as_deref().and_then(parse_dev_build_number) else {
                continue;
            };
            // `MaxBy` keeps the first of equals.
            if best.as_ref().is_none_or(|(n, _)| number > *n) {
                best = Some((number, release));
            }
        }
        best.map(|(_, release)| release)
    }

    /// `GetAsync<T>`: null for a failure, for an empty body and for JSON `null`; anything that isn't `T`'s shape is malformed.
    fn get_json(&self, relative_uri: &str, shape: Shape) -> Option<Value> {
        let transport = self.transport.as_ref()?;
        let response = match transport.get(relative_uri) {
            Ok(response) => response,
            Err(TransportError::TimedOut) => {
                logging::warn(LOGGER, "GitHub update check timed out.");
                return None;
            }
            Err(TransportError::Failed(reason)) => {
                logging::log(logging::LogLevel::Warn, LOGGER, "GitHub update check failed.", Some(&reason));
                return None;
            }
        };
        if !(200..300).contains(&response.status) {
            if response.status == 403 && response.rate_limit_remaining.as_deref() == Some("0") {
                logging::warn(LOGGER, "GitHub update check was rate limited.");
            } else {
                // NLog writes an enum argument by name.
                logging::warn(LOGGER, &format!("GitHub update check returned HTTP status {}.", http_status_name(response.status)));
            }
            return None;
        }
        // Json.NET gives null for a body with nothing in it, without complaint.
        if response.body.trim().is_empty() {
            return None;
        }
        let malformed = |reason: &str| {
            logging::log(logging::LogLevel::Warn, LOGGER, "GitHub update check returned malformed JSON.", Some(reason));
            None
        };
        match serde_json::from_str::<Value>(&response.body) {
            Ok(Value::Null) => None,
            Ok(value) if shape.fits(&value) => Some(value),
            Ok(_) => malformed(shape.complaint()),
            Err(error) => malformed(&error.to_string()),
        }
    }
}

/// What `GetAsync<T>` deserializes into: a release or a list of them.
#[derive(Copy, Clone)]
enum Shape {
    Object,
    Array,
}

impl Shape {
    fn fits(self, value: &Value) -> bool {
        match self {
            Shape::Object => value.is_object(),
            Shape::Array => value.as_array().is_some_and(|list| list.iter().all(|item| item.is_object() || item.is_null())),
        }
    }

    fn complaint(self) -> &'static str {
        match self {
            Shape::Object => "The JSON is not a release object.",
            Shape::Array => "The JSON is not a list of release objects.",
        }
    }
}

/// `HttpStatusCode`'s name for a status, which is how the original's log line shows it. A status the enum has no name for shows as its
/// number, as .NET prints it.
fn http_status_name(status: u16) -> String {
    let name = match status {
        100 => "Continue",
        101 => "SwitchingProtocols",
        102 => "Processing",
        103 => "EarlyHints",
        // 300 to 303 and 307 each have two names in the enum, and which one .NET prints isn't specified. `HttpClient` follows those
        // before they get here.
        300 => "MultipleChoices",
        301 => "MovedPermanently",
        302 => "Found",
        303 => "SeeOther",
        304 => "NotModified",
        305 => "UseProxy",
        306 => "Unused",
        307 => "TemporaryRedirect",
        308 => "PermanentRedirect",
        400 => "BadRequest",
        401 => "Unauthorized",
        402 => "PaymentRequired",
        403 => "Forbidden",
        404 => "NotFound",
        405 => "MethodNotAllowed",
        406 => "NotAcceptable",
        407 => "ProxyAuthenticationRequired",
        408 => "RequestTimeout",
        409 => "Conflict",
        410 => "Gone",
        411 => "LengthRequired",
        412 => "PreconditionFailed",
        413 => "RequestEntityTooLarge",
        414 => "RequestUriTooLong",
        415 => "UnsupportedMediaType",
        416 => "RequestedRangeNotSatisfiable",
        417 => "ExpectationFailed",
        421 => "MisdirectedRequest",
        422 => "UnprocessableEntity",
        423 => "Locked",
        424 => "FailedDependency",
        426 => "UpgradeRequired",
        428 => "PreconditionRequired",
        429 => "TooManyRequests",
        431 => "RequestHeaderFieldsTooLarge",
        451 => "UnavailableForLegalReasons",
        500 => "InternalServerError",
        501 => "NotImplemented",
        502 => "BadGateway",
        503 => "ServiceUnavailable",
        504 => "GatewayTimeout",
        505 => "HttpVersionNotSupported",
        506 => "VariantAlsoNegotiates",
        507 => "InsufficientStorage",
        508 => "LoopDetected",
        510 => "NotExtended",
        511 => "NetworkAuthenticationRequired",
        _ => return status.to_string(),
    };
    name.to_string()
}

fn notes_result(release: &Release) -> ReleaseNotesResult {
    ReleaseNotesResult {
        release_tag: release.tag_name.clone().unwrap_or_default(),
        release_notes: release.body.clone().unwrap_or_default(),
        release_page_uri: release.page(),
        published_at: release.published_at.clone(),
    }
}

fn create_result(request: &UpdateCheckRequest, release: &Release) -> Option<UpdateCheckResult> {
    let tag = release.tag_name.as_deref()?;
    let notes = request.include_release_notes.then(|| release.body.clone().unwrap_or_default());
    let (latest_version, latest_build_number, is_update_available) = match request.channel {
        UpdateChannel::Development => {
            let latest = parse_dev_build_number(tag)?;
            let installed = request.installed_version.trim().parse::<i32>().ok();
            (latest.to_string(), Some(latest), installed.is_some_and(|n| latest > n))
        }
        UpdateChannel::Release => {
            let installed = parse_release_version(&request.installed_version)?;
            let latest = parse_release_version(tag)?;
            (tag.to_string(), None, latest > installed)
        }
    };
    Some(UpdateCheckResult {
        latest_version,
        latest_build_number,
        is_update_available,
        release_notes: notes,
        release_page_uri: release.page(),
        published_at: release.published_at.clone(),
        installer_download_uri: release.installer(),
    })
}

/// `Uri.EscapeDataString`.
fn escape_data_string(text: &str) -> String {
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

/// The status bar's notice, with its leading space, when a check found something newer. Test builds never check.
pub fn update_notice(version: &VersionInfo, result: &UpdateCheckResult) -> Option<String> {
    if version.is_test_build() || !result.is_update_available {
        return None;
    }
    let text = if version.is_dev_build() {
        format!(" Development build {} available.", result.latest_version)
    } else {
        format!(" Release version {} available.", result.latest_version)
    };
    Some(text)
}

/// The original's startup check. Nothing calls this at launch here, since a check must only run when the user asks for one.
pub fn startup_update_notice(version: &VersionInfo, service: &UpdateService) -> Option<String> {
    if version.is_test_build() {
        return None;
    }
    let notice = service.check(&UpdateCheckRequest::for_version(version, false)).and_then(|r| update_notice(version, &r))?;
    logging::info("VixenApplication.VixenApplication", &notice);
    Some(notice)
}

/// `LoadFallbackReleaseNotesAsync`: `Release Notes.txt` beside the program.
pub fn fallback_release_notes(bundle_dir: &Path) -> Option<String> {
    let bytes = std::fs::read(bundle_dir.join("Release Notes.txt")).ok()?;
    Some(crate::boot::decode_text(&bytes))
}

/// `ReleaseNotes.LoadReleaseNotesAsync`: the release's body from GitHub, or the bundled file if that's blank or out of reach.
pub fn release_notes_text(service: &UpdateService, tag: Option<&str>, bundle_dir: &Path) -> String {
    let Some(tag) = tag.filter(|t| !t.is_empty()) else {
        return "Release notes are unavailable for test builds.".to_string();
    };
    let not_blank = |text: &String| !text.trim().is_empty();
    let notes = service
        .release_notes(tag)
        .map(|result| result.release_notes)
        .filter(not_blank)
        .or_else(|| fallback_release_notes(bundle_dir).filter(not_blank));
    match notes {
        Some(text) => normalize_line_endings(&text),
        None => "Release notes could not be loaded.".to_string(),
    }
}

/// What Check for Updates shows for a result. The heading shows in every outcome, where the original hides it in two of them. Download
/// opens the release page, since the installers upstream publishes are for Windows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateDialogContent {
    pub heading: String,
    pub version_text: String,
    pub show_download: bool,
    pub show_notes: bool,
    pub notes: String,
    pub release_page_uri: Option<String>,
}

pub fn update_dialog_content(version: &VersionInfo, result: Option<&UpdateCheckResult>) -> UpdateDialogContent {
    let Some(result) = result else {
        return UpdateDialogContent {
            heading: "Update check unavailable.".to_string(),
            version_text: "Unable to check for updates. Please try again later.".to_string(),
            show_download: false,
            show_notes: false,
            notes: String::new(),
            release_page_uri: None,
        };
    };
    let kind = version.current_version_type();
    let version_text = format!("{kind} {} is the latest.", result.latest_version);
    if result.is_update_available {
        UpdateDialogContent {
            heading: format!("An updated {} build is available.", kind.to_lowercase()),
            version_text,
            show_download: true,
            show_notes: true,
            notes: normalize_line_endings(result.release_notes.as_deref().unwrap_or("")),
            release_page_uri: Some(result.release_page_uri.clone()),
        }
    } else {
        UpdateDialogContent {
            heading: format!("You have the latest {} build installed.", kind.to_lowercase()),
            version_text,
            show_download: false,
            show_notes: false,
            notes: String::new(),
            release_page_uri: None,
        }
    }
}

/// `NormalizeLineEndings`: the Mac's line ending throughout.
pub fn normalize_line_endings(text: &str) -> String {
    text.replace("\r\n", "\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    const BASE_PATH: &str = "/repos/VixenLights/Vixen/";

    struct Fake {
        response: Result<HttpResponse, TransportError>,
        requests: Arc<Mutex<Vec<String>>>,
    }

    impl HttpTransport for Fake {
        fn get(&self, relative_uri: &str) -> Result<HttpResponse, TransportError> {
            self.requests.lock().push(format!("{BASE_PATH}{relative_uri}"));
            self.response.clone()
        }
    }

    fn service_with(response: Result<HttpResponse, TransportError>) -> (UpdateService, Arc<Mutex<Vec<String>>>) {
        let requests = Arc::new(Mutex::new(Vec::new()));
        (UpdateService::github(Box::new(Fake { response, requests: requests.clone() })), requests)
    }

    fn service(body: &str) -> (UpdateService, Arc<Mutex<Vec<String>>>) {
        service_with(Ok(HttpResponse { status: 200, body: body.to_string(), rate_limit_remaining: None }))
    }

    fn request(channel: UpdateChannel, installed: &str, notes: bool) -> UpdateCheckRequest {
        UpdateCheckRequest { channel, installed_version: installed.to_string(), include_release_notes: notes }
    }

    #[test]
    fn development_selects_the_highest_valid_non_draft_build() {
        let (service, requests) = service(
            r#"[
              { "tag_name": "DevBuild-1500", "prerelease": true, "draft": false, "published_at": "2026-01-03T00:00:00Z", "body": "older build", "html_url": "https://example.test/1500" },
              { "tag_name": "DevBuild-1502", "prerelease": true, "draft": false, "published_at": "2026-01-01T00:00:00Z", "body": "chosen", "html_url": "https://example.test/1502", "assets": [{ "name": "Vixen-DevBuild-0.0.1502-Setup-64bit.exe", "browser_download_url": "https://github.com/VixenLights/Vixen/releases/download/DevBuild-1502/Vixen-DevBuild-0.0.1502-Setup-64bit.exe" }] },
              { "tag_name": "DevBuild-invalid", "prerelease": true, "draft": false },
              { "tag_name": "DevBuild-9999", "prerelease": true, "draft": true },
              { "tag_name": "3.13", "prerelease": false, "draft": false }
            ]"#,
        );
        let result = service.check(&request(UpdateChannel::Development, "1501", true)).unwrap();
        assert_eq!(result.latest_version, "1502");
        assert_eq!(result.latest_build_number, Some(1502));
        assert!(result.is_update_available);
        assert_eq!(result.release_notes.as_deref(), Some("chosen"));
        assert_eq!(
            result.installer_download_uri.as_deref(),
            Some("https://github.com/VixenLights/Vixen/releases/download/DevBuild-1502/Vixen-DevBuild-0.0.1502-Setup-64bit.exe")
        );
        assert_eq!(*requests.lock(), ["/repos/VixenLights/Vixen/releases?per_page=5"]);
    }

    #[test]
    fn release_uses_the_stable_version_and_the_latest_body() {
        let (service, requests) = service(
            r#"{ "tag_name": "3.13u1", "prerelease": false, "draft": false, "body": "latest notes", "html_url": "https://example.test/3.13u1", "assets": [{ "name": "Vixen-3.13.1-Setup-64bit.exe", "browser_download_url": "https://github.com/VixenLights/Vixen/releases/download/3.13u1/Vixen-3.13.1-Setup-64bit.exe" }] }"#,
        );
        let result = service.check(&request(UpdateChannel::Release, "3.13", true)).unwrap();
        assert!(result.is_update_available);
        assert_eq!(result.latest_version, "3.13u1");
        assert_eq!(result.release_notes.as_deref(), Some("latest notes"));
        assert_eq!(
            result.installer_download_uri.as_deref(),
            Some("https://github.com/VixenLights/Vixen/releases/download/3.13u1/Vixen-3.13.1-Setup-64bit.exe")
        );
        assert_eq!(*requests.lock(), ["/repos/VixenLights/Vixen/releases/latest"]);
    }

    #[test]
    fn a_null_body_with_notes_not_asked_for_is_still_available() {
        let (service, _) = service(r#"{ "tag_name": "3.14", "prerelease": false, "draft": false, "body": null, "html_url": "https://example.test/3.14" }"#);
        let result = service.check(&request(UpdateChannel::Release, "3.13u1", false)).unwrap();
        assert!(result.is_update_available);
        assert_eq!(result.release_notes, None);
    }

    #[test]
    fn a_null_body_with_notes_asked_for_is_empty() {
        let (service, _) = service(r#"{ "tag_name": "3.14", "body": null }"#);
        let result = service.check(&request(UpdateChannel::Release, "3.13u1", true)).unwrap();
        assert_eq!(result.release_notes.as_deref(), Some(""));
        assert_eq!(result.release_page_uri, FALLBACK_RELEASE_PAGE);
    }

    #[test]
    fn a_second_check_within_five_minutes_uses_the_cached_body() {
        let (service, requests) = service(r#"{ "tag_name": "3.14", "prerelease": false, "draft": false, "body": "cached notes", "html_url": "https://example.test/3.14" }"#);
        let startup = service.check(&request(UpdateChannel::Release, "3.13", false)).unwrap();
        let dialog = service.check(&request(UpdateChannel::Release, "3.13", true)).unwrap();
        assert_eq!(startup.release_notes, None);
        assert_eq!(dialog.release_notes.as_deref(), Some("cached notes"));
        assert_eq!(requests.lock().len(), 1);
    }

    #[test]
    fn release_notes_use_the_exact_tag_and_keep_the_body() {
        let (service, requests) = service(r#"{ "tag_name": "3.13u1", "prerelease": false, "draft": false, "body": "GitHub notes\r\nfor 3.13u1", "html_url": "https://example.test/3.13u1" }"#);
        let result = service.release_notes("3.13u1").unwrap();
        assert_eq!(result.release_tag, "3.13u1");
        assert_eq!(result.release_notes, "GitHub notes\r\nfor 3.13u1");
        assert_eq!(*requests.lock(), ["/repos/VixenLights/Vixen/releases/tags/3.13u1"]);
        service.release_notes("3.13u1").unwrap();
        assert_eq!(requests.lock().len(), 1, "cached by tag");
    }

    #[test]
    fn a_notes_response_with_another_tag_or_a_draft_is_none() {
        let (service, _) = service(r#"{ "tag_name": "3.14", "body": "x" }"#);
        assert_eq!(service.release_notes("3.13u1"), None);
        let (service, _) = self::service(r#"{ "tag_name": "3.13u1", "draft": true, "body": "x" }"#);
        assert_eq!(service.release_notes("3.13u1"), None);
    }

    #[test]
    fn a_prerelease_is_allowed_for_notes_but_not_as_the_latest_release() {
        let body = r#"{ "tag_name": "3.14", "prerelease": true, "body": "x" }"#;
        let (service, _) = service(body);
        assert!(service.release_notes("3.14").is_some());
        assert_eq!(service.check(&request(UpdateChannel::Release, "3.13", true)), None);
    }

    #[test]
    fn a_latest_release_with_a_bad_tag_is_none() {
        let (service, _) = service(r#"{ "tag_name": "banana" }"#);
        assert_eq!(service.check(&request(UpdateChannel::Release, "3.13", true)), None);
    }

    #[test]
    fn versions_compare_by_major_minor_update() {
        let v = |s| parse_release_version(s).unwrap();
        assert!(v("3.13") < v("3.13u1"));
        assert!(v("3.13u1") < v("3.14"));
        assert!(v("3.14") > v("3.13u9"));
    }

    #[test]
    fn http_failures_and_bad_json_are_none_and_never_up_to_date() {
        for status in [403, 429, 500] {
            let (service, _) = service_with(Ok(HttpResponse { status, body: String::new(), rate_limit_remaining: Some("0".to_string()) }));
            assert_eq!(service.check(&request(UpdateChannel::Release, "3.13", false)), None, "{status}");
        }
        let (service, _) = self::service("not JSON");
        assert_eq!(service.check(&request(UpdateChannel::Release, "3.13", false)), None);
        let (service, _) = service_with(Err(TransportError::TimedOut));
        assert_eq!(service.check(&request(UpdateChannel::Release, "3.13", false)), None);
        let (service, _) = service_with(Err(TransportError::Failed("no route".to_string())));
        assert_eq!(service.check(&request(UpdateChannel::Release, "3.13", false)), None);
    }

    fn warnings() -> Vec<String> {
        let prefix = format!("Warn|{LOGGER}|");
        logging::take_captured().into_iter().filter_map(|line| line.strip_prefix(&prefix).map(str::to_string)).collect()
    }

    #[test]
    fn each_failure_logs_the_originals_warnings() {
        let respond = |status: u16, body: &str, remaining: Option<&str>| {
            service_with(Ok(HttpResponse { status, body: body.to_string(), rate_limit_remaining: remaining.map(str::to_string) })).0
        };
        let latest = "GitHub returned an invalid latest Vixen release.";
        let release = request(UpdateChannel::Release, "3.13", false);
        let development = request(UpdateChannel::Development, "10", false);
        warnings();

        respond(404, "", None).check(&release);
        assert_eq!(warnings(), ["GitHub update check returned HTTP status NotFound.", latest]);
        respond(403, "", Some("0")).check(&release);
        assert_eq!(warnings(), ["GitHub update check was rate limited.", latest]);
        respond(403, "", Some("12")).check(&release);
        assert_eq!(warnings(), ["GitHub update check returned HTTP status Forbidden.", latest]);
        respond(599, "", None).check(&release);
        assert_eq!(warnings(), ["GitHub update check returned HTTP status 599.", latest]);
        service_with(Err(TransportError::TimedOut)).0.check(&release);
        assert_eq!(warnings(), ["GitHub update check timed out.", latest]);
        service_with(Err(TransportError::Failed("no route".to_string()))).0.check(&release);
        assert_eq!(warnings(), ["GitHub update check failed.", latest]);
        respond(200, "not JSON", None).check(&release);
        assert_eq!(warnings(), ["GitHub update check returned malformed JSON.", latest]);

        // A list where a release belongs, and a release where the list belongs, don't deserialize.
        respond(200, "[]", None).check(&release);
        assert_eq!(warnings(), ["GitHub update check returned malformed JSON.", latest]);
        respond(200, r#"{"tag_name":"DevBuild-12","prerelease":true}"#, None).check(&development);
        assert_eq!(warnings(), ["GitHub update check returned malformed JSON."]);
        respond(200, "[5]", None).check(&development);
        assert_eq!(warnings(), ["GitHub update check returned malformed JSON."]);

        // Nothing to read is null, not malformed.
        respond(200, "", None).check(&release);
        assert_eq!(warnings(), [latest]);
        respond(200, "null", None).check(&release);
        assert_eq!(warnings(), [latest]);
        respond(200, "", None).check(&development);
        assert_eq!(warnings(), [] as [&str; 0]);
        respond(500, "", None).check(&development);
        assert_eq!(warnings(), ["GitHub update check returned HTTP status InternalServerError."]);

        respond(200, r#"{"tag_name":"3.13u4","draft":true}"#, None).check(&release);
        assert_eq!(warnings(), [latest]);
    }

    #[test]
    fn a_missing_release_for_notes_names_its_tag_in_quotes() {
        let (service, _) = service(r#"{"tag_name":"3.13u5","body":"x"}"#);
        warnings();
        assert_eq!(service.release_notes("3.13u4"), None);
        assert_eq!(warnings(), ["GitHub returned an invalid Vixen release for tag \"3.13u4\"."]);
        let (service, _) = service_with(Ok(HttpResponse { status: 404, body: String::new(), rate_limit_remaining: None }));
        assert_eq!(service.release_notes("DevBuild-9"), None);
        assert_eq!(
            warnings(),
            ["GitHub update check returned HTTP status NotFound.", "GitHub returned an invalid Vixen release for tag \"DevBuild-9\"."]
        );
    }

    #[test]
    fn the_unavailable_service_logs_nothing() {
        let service = UpdateService::unavailable();
        warnings();
        service.check(&request(UpdateChannel::Release, "3.13", true));
        service.check(&request(UpdateChannel::Development, "10", true));
        service.release_notes("3.13");
        assert_eq!(warnings(), [] as [&str; 0]);
    }

    #[test]
    fn a_failure_is_not_cached() {
        let (service, requests) = service_with(Ok(HttpResponse { status: 500, body: String::new(), rate_limit_remaining: None }));
        service.check(&request(UpdateChannel::Release, "3.13", false));
        service.check(&request(UpdateChannel::Release, "3.13", false));
        assert_eq!(requests.lock().len(), 2);
    }

    #[test]
    fn the_unavailable_service_has_nothing() {
        let service = UpdateService::unavailable();
        assert_eq!(service.check(&request(UpdateChannel::Release, "3.13", true)), None);
        assert_eq!(service.release_notes("3.13"), None);
    }

    #[test]
    fn parse_release_version_rejects_and_accepts_as_the_original() {
        for bad in ["", "3", "3.13.1", "3.13u", "3.13u1u2", "a.b", " 3.13", "+3.13", "-1.2"] {
            assert_eq!(parse_release_version(bad), None, "{bad:?}");
        }
        assert_eq!(parse_release_version("3.13"), Some(ReleaseVersion { major: 3, minor: 13, update: 0 }));
        assert_eq!(parse_release_version("3.13u4"), Some(ReleaseVersion { major: 3, minor: 13, update: 4 }));
    }

    #[test]
    fn parse_dev_build_number_follows_the_rules() {
        assert_eq!(parse_dev_build_number("DevBuild-1502"), Some(1502));
        assert_eq!(parse_dev_build_number("DevBuild- 12 "), Some(12));
        assert_eq!(parse_dev_build_number("DevBuild-"), None);
        assert_eq!(parse_dev_build_number("devbuild-1"), None);
        assert_eq!(parse_dev_build_number("DevBuild--5"), None);
    }

    #[test]
    fn an_installer_asset_matches_ignoring_case() {
        let (service, _) = service(r#"{ "tag_name": "3.14", "assets": [{ "name": "notes.txt", "browser_download_url": "a" }, { "name": "x-setup-64BIT.EXE", "browser_download_url": "b" }] }"#);
        let result = service.check(&request(UpdateChannel::Release, "3.13", false)).unwrap();
        assert_eq!(result.installer_download_uri.as_deref(), Some("b"));
    }

    fn release_version() -> VersionInfo {
        VersionInfo { major: 3, minor: 13, build: 1400, revision: 4 }
    }

    fn dev_version() -> VersionInfo {
        VersionInfo { major: 0, minor: 2, build: 1501, revision: 0 }
    }

    #[test]
    fn the_notice_words_each_channel() {
        let (service, _) = self::service(r#"{ "tag_name": "3.14" }"#);
        assert_eq!(startup_update_notice(&release_version(), &service).as_deref(), Some(" Release version 3.14 available."));
        let (service, _) = self::service(r#"[{ "tag_name": "DevBuild-1502", "prerelease": true }]"#);
        assert_eq!(startup_update_notice(&dev_version(), &service).as_deref(), Some(" Development build 1502 available."));
    }

    #[test]
    fn no_notice_when_current_or_on_failure() {
        let (service, _) = self::service(r#"{ "tag_name": "3.13u4" }"#);
        assert_eq!(startup_update_notice(&release_version(), &service), None);
        let (service, _) = service_with(Err(TransportError::TimedOut));
        assert_eq!(startup_update_notice(&release_version(), &service), None);
    }

    #[test]
    fn a_test_build_makes_no_request() {
        let (service, requests) = self::service(r#"{ "tag_name": "3.14" }"#);
        assert_eq!(startup_update_notice(&VersionInfo { major: 0, minor: 0, build: 0, revision: 0 }, &service), None);
        assert!(requests.lock().is_empty());
    }

    #[test]
    fn what_this_build_asks_for() {
        assert_eq!(UpdateCheckRequest::for_version(&release_version(), true), request(UpdateChannel::Release, "3.13u4", true));
        assert_eq!(UpdateCheckRequest::for_version(&dev_version(), false), request(UpdateChannel::Development, "1501", false));
    }

    fn result(latest: &str, available: bool) -> UpdateCheckResult {
        UpdateCheckResult {
            latest_version: latest.to_string(),
            latest_build_number: None,
            is_update_available: available,
            release_notes: Some("line one\r\nline two".to_string()),
            release_page_uri: "https://example.test/page".to_string(),
            published_at: None,
            installer_download_uri: None,
        }
    }

    #[test]
    fn the_dialog_words_each_outcome() {
        let version = release_version();
        let none = update_dialog_content(&version, None);
        assert_eq!(none.heading, "Update check unavailable.");
        assert_eq!(none.version_text, "Unable to check for updates. Please try again later.");
        assert!(!none.show_download && !none.show_notes);

        let newer = update_dialog_content(&version, Some(&result("3.14", true)));
        assert_eq!(newer.heading, "An updated release build is available.");
        assert_eq!(newer.version_text, "Release 3.14 is the latest.");
        assert!(newer.show_download && newer.show_notes, "no installer is needed: Download opens the release page");
        assert_eq!(newer.notes, "line one\nline two");
        assert_eq!(newer.release_page_uri.as_deref(), Some("https://example.test/page"));

        let current = update_dialog_content(&version, Some(&result("3.13u4", false)));
        assert_eq!(current.heading, "You have the latest release build installed.");
        assert_eq!(current.version_text, "Release 3.13u4 is the latest.");
        assert!(!current.show_download && !current.show_notes);

        let dev = update_dialog_content(&dev_version(), Some(&result("1502", true)));
        assert_eq!(dev.heading, "An updated development build is available.");
        assert_eq!(dev.version_text, "Development 1502 is the latest.");
    }

    fn folder_with_notes(name: &str, text: Option<&[u8]>) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("vixen-updates-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        if let Some(text) = text {
            std::fs::write(dir.join("Release Notes.txt"), text).unwrap();
        }
        dir
    }

    #[test]
    fn release_notes_text_prefers_github_then_the_bundled_file() {
        let dir = folder_with_notes("notes-both", Some(b"bundled\r\nnotes"));
        let (service, _) = self::service(r#"{ "tag_name": "3.13u4", "body": "from\r\ngithub" }"#);
        assert_eq!(release_notes_text(&service, Some("3.13u4"), &dir), "from\ngithub");
        let (service, _) = self::service(r#"{ "tag_name": "3.13u4", "body": "   " }"#);
        assert_eq!(release_notes_text(&service, Some("3.13u4"), &dir), "bundled\nnotes");
        assert_eq!(release_notes_text(&UpdateService::unavailable(), Some("3.13u4"), &dir), "bundled\nnotes");
    }

    #[test]
    fn release_notes_text_has_words_for_a_test_build_and_for_nothing_found() {
        let dir = folder_with_notes("notes-none", None);
        let service = UpdateService::unavailable();
        assert_eq!(release_notes_text(&service, None, &dir), "Release notes are unavailable for test builds.");
        assert_eq!(release_notes_text(&service, Some("3.13u4"), &dir), "Release notes could not be loaded.");
    }

    #[test]
    fn line_endings_normalize_and_tags_escape() {
        assert_eq!(normalize_line_endings("a\r\nb\nc"), "a\nb\nc");
        assert_eq!(escape_data_string("3.13u1"), "3.13u1");
        assert_eq!(escape_data_string("a b/c"), "a%20b%2Fc");
    }
}
