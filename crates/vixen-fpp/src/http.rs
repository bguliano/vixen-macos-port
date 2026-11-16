//! Synchronous HTTP(S), streamed uploads and explicit HttpClient-compatible redirects.

use crate::FppError;
use std::ffi::{c_char, c_int, c_long, c_void, CStr, CString};
use std::fs::File;
use std::os::fd::AsRawFd;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

#[derive(Clone, Default, Debug)]
pub struct Cancel(Arc<AtomicBool>);

impl Cancel {
    pub fn new() -> Cancel {
        Cancel::default()
    }

    /// A cancel that follows a flag the caller already has, such as an export's.
    pub fn from_flag(flag: Arc<AtomicBool>) -> Cancel {
        Cancel(flag)
    }

    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

pub(crate) struct Target {
    pub cookies: CookieJar,
    pub loopback_only: bool,
    pub https: bool,
    pub host: String,
    pub port: u16,
}

pub(crate) struct Response {
    pub status: u16,
    pub body: Vec<u8>,
}

impl Response {
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }
}

pub(crate) enum Body<'a> {
    None,
    Empty,
    File(&'a mut File, u64),
}

#[repr(C)]
struct RawResponse {
    status: c_int,
    error: [c_char; 256],
    body: *mut u8,
    length: usize,
}

unsafe extern "C" {
    fn vixen_fpp_http_init() -> c_int;
    fn vixen_fpp_http_free(body: *mut u8);
    fn vixen_fpp_cookies_new() -> *mut c_void;
    fn vixen_fpp_cookies_free(jar: *mut c_void);
    fn vixen_fpp_http_send(
        url: *const c_char,
        post: c_int,
        fd: c_int,
        length: i64,
        ca_file: *const c_char,
        timeout_ms: c_long,
        cookies: *mut c_void,
        loopback_only: c_int,
        cancelled: extern "C" fn(*const c_void) -> c_int,
        context: *const c_void,
        result: *mut RawResponse,
    ) -> c_int;
}

#[derive(Clone)]
pub(crate) struct CookieJar(Arc<CookieJarInner>);
struct CookieJarInner {
    native: std::ptr::NonNull<c_void>,
}
// SAFETY: native CURLSH cookie access has mutex lock/unlock callbacks. Each transfer
// owns a distinct easy handle and borrows this Arc, so cleanup cannot race a call.
unsafe impl Send for CookieJarInner {}
unsafe impl Sync for CookieJarInner {}
impl CookieJar {
    pub(crate) fn new() -> Result<Self, FppError> {
        initialize()?;
        let jar = std::ptr::NonNull::new(unsafe { vixen_fpp_cookies_new() }).ok_or_else(|| FppError::Io("Cannot allocate HTTP cookie jar.".into()))?;
        Ok(Self(Arc::new(CookieJarInner { native: jar })))
    }
}
impl Drop for CookieJarInner {
    fn drop(&mut self) {
        unsafe {
            vixen_fpp_cookies_free(self.native.as_ptr());
        }
    }
}
fn initialize() -> Result<(), FppError> {
    static INIT: OnceLock<c_int> = OnceLock::new();
    // No global cleanup: other concurrent requests and the RDS transport use libcurl too.
    let init = *INIT.get_or_init(|| unsafe { vixen_fpp_http_init() });
    if init != 0 {
        return Err(FppError::Io(format!("Could not initialize HTTP transport ({init}).")));
    }
    Ok(())
}

extern "C" fn cancelled(context: *const c_void) -> c_int {
    // SAFETY: send_with_ca lends its Cancel for the entire synchronous native call.
    unsafe { (&*context.cast::<Cancel>()).is_cancelled() as c_int }
}

pub(crate) fn send(target: &Target, method: &str, path: &str, body: Body<'_>, timeout: Duration, cancel: &Cancel) -> Result<Response, FppError> {
    send_with_ca(target, method, path, body, timeout, cancel, None)
}

fn send_with_ca(target: &Target, method: &str, path: &str, body: Body<'_>, timeout: Duration, cancel: &Cancel, ca: Option<&CStr>) -> Result<Response, FppError> {
    if cancel.is_cancelled() {
        return Err(FppError::Cancelled);
    }
    if timeout.is_zero() {
        return Err(FppError::TimedOut);
    }
    initialize()?;
    let host = if target.host.contains(':') { format!("[{}]", target.host) } else { target.host.clone() };
    let scheme = if target.https { "https" } else { "http" };
    let url = CString::new(format!("{scheme}://{host}:{}{path}", target.port)).map_err(|_| FppError::Argument("The device URL contains a null character.".into()))?;
    let (fd, length) = match body {
        Body::File(file, length) => (file.as_raw_fd(), i64::try_from(length).map_err(|_| FppError::Io("The upload file is too large.".into()))?),
        Body::None | Body::Empty => (-1, 0),
    };
    let mut raw = RawResponse {
        status: 0,
        error: [0; 256],
        body: std::ptr::null_mut(),
        length: 0,
    };
    // Ceiling to milliseconds prevents a positive sub-millisecond deadline becoming curl's
    // zero (infinite) timeout. Cap well below i64::MAX so native monotonic addition is safe.
    let millis = timeout.as_nanos().div_ceil(1_000_000).min((c_long::MAX / 2) as u128) as c_long;
    // SAFETY: strings, fd, Cancel and result remain live for this synchronous call; the
    // bridge owns response bytes until the matching free below, and bounds them to16MiB.
    let code = unsafe {
        vixen_fpp_http_send(
            url.as_ptr(),
            (method == "POST") as c_int,
            fd,
            length,
            ca.map_or(std::ptr::null(), CStr::as_ptr),
            millis,
            target.cookies.0.native.as_ptr(),
            target.loopback_only as c_int,
            cancelled,
            (cancel as *const Cancel).cast(),
            &mut raw,
        )
    };
    let result = match code {
        0 => {
            let body = if raw.length == 0 {
                Vec::new()
            } else {
                unsafe { std::slice::from_raw_parts(raw.body, raw.length) }.to_vec()
            };
            Ok(Response { status: raw.status as u16, body })
        }
        1 => Err(FppError::Cancelled),
        2 => Err(FppError::TimedOut),
        3 => Err(FppError::Argument("External devices are disabled in UI tests.".into())),
        _ => Err(FppError::Io(unsafe { CStr::from_ptr(raw.error.as_ptr()) }.to_string_lossy().into_owned())),
    };
    unsafe {
        vixen_fpp_http_free(raw.body);
    }
    result
}

#[cfg(test)]
mod tests;
