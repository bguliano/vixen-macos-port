//! Safe boundary to macOS libcurl. No environment or profile option can disable TLS
//! verification; the optional private CA is compiled only into the test transport.
use std::ffi::{c_char, c_int, c_long, c_void, CStr, CString};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

#[repr(C)]
struct RawResult {
    status: c_int,
    reason: [c_char; 1024],
    error: [c_char; 256],
}

unsafe extern "C" {
    fn vixen_rds_http_init() -> c_int;
    fn vixen_rds_http_get(
        url: *const c_char,
        user: *const c_char,
        password: *const c_char,
        ca_file: *const c_char,
        timeout_ms: c_long,
        connect_ms: c_long,
        cancelled: extern "C" fn(*const c_void) -> c_int,
        context: *const c_void,
        result: *mut RawResult,
    ) -> c_int;
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Error {
    Cancelled,
    Transport(String),
}

struct Cancellation<'a> {
    stop: &'a AtomicBool,
    controller_stop: Option<&'a AtomicBool>,
}

extern "C" fn cancelled(context: *const c_void) -> c_int {
    // SAFETY: get_with_options passes this stack value for one synchronous call;
    // libcurl and all callbacks are removed before the C function returns.
    let context = unsafe { &*context.cast::<Cancellation<'_>>() };
    (context.stop.load(Ordering::Relaxed)
        || context
            .controller_stop
            .is_some_and(|stop| stop.load(Ordering::Relaxed))) as c_int
}

struct Options<'a> {
    timeout_ms: c_long,
    connect_ms: c_long,
    #[cfg(test)]
    ca_file: Option<&'a str>,
    marker: std::marker::PhantomData<&'a ()>,
}

impl Default for Options<'_> {
    fn default() -> Self {
        Self {
            timeout_ms: 100_000, // HttpClient.Timeout default.
            connect_ms: 10_000,
            #[cfg(test)]
            ca_file: None,
            marker: std::marker::PhantomData,
        }
    }
}

pub(super) fn get(
    url: &str,
    credentials: Option<(&str, &str)>,
    stop: &AtomicBool,
    controller_stop: Option<&AtomicBool>,
) -> Result<(u16, String), Error> {
    get_with_options(url, credentials, stop, controller_stop, Options::default())
}

fn get_with_options(
    url: &str,
    credentials: Option<(&str, &str)>,
    stop: &AtomicBool,
    controller_stop: Option<&AtomicBool>,
    options: Options<'_>,
) -> Result<(u16, String), Error> {
    static INIT: OnceLock<c_int> = OnceLock::new();
    // libcurl's process state stays initialized while any driver might use it.
    // Never call global_cleanup while another request or library could be alive.
    if *INIT.get_or_init(|| unsafe { vixen_rds_http_init() }) != 0 {
        return Err(Error::Transport("Cannot initialize HTTP transport".into()));
    }
    let string = |value: &str| {
        CString::new(value).map_err(|_| Error::Transport("HTTP input contains a NUL byte".into()))
    };
    let url = string(url)?;
    let credentials = credentials
        .map(|(user, password)| Ok((string(user)?, string(password)?)))
        .transpose()?;
    #[cfg(test)]
    let ca_file = options.ca_file.map(string).transpose()?;
    #[cfg(test)]
    let ca_pointer = ca_file.as_ref().map_or(std::ptr::null(), |s| s.as_ptr());
    #[cfg(not(test))]
    let ca_pointer = std::ptr::null();
    let context = Cancellation {
        stop,
        controller_stop,
    };
    let mut result = RawResult {
        status: 0,
        reason: [0; 1024],
        error: [0; 256],
    };
    // SAFETY: C owns only its libcurl handles; inputs and callback context remain
    // live through cleanup, output has the identical repr(C) fixed-size layout.
    let outcome = unsafe {
        vixen_rds_http_get(
            url.as_ptr(),
            credentials
                .as_ref()
                .map_or(std::ptr::null(), |(u, _)| u.as_ptr()),
            credentials
                .as_ref()
                .map_or(std::ptr::null(), |(_, p)| p.as_ptr()),
            ca_pointer,
            options.timeout_ms,
            options.connect_ms,
            cancelled,
            (&context as *const Cancellation<'_>).cast(),
            &mut result,
        )
    };
    let text = |buffer: &[c_char]| unsafe {
        CStr::from_ptr(buffer.as_ptr())
            .to_string_lossy()
            .into_owned()
    };
    match outcome {
        0 => Ok((result.status as u16, text(&result.reason))),
        1 => Err(Error::Cancelled),
        _ => Err(Error::Transport(text(&result.error))),
    }
}

#[cfg(test)]
mod tests;
