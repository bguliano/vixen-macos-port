use crate::{Action, ApiInput};
use std::{
    io::{self, Read, Write},
    net::TcpStream,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

pub(crate) const MAX_BODY: usize = 1024 * 1024;
const MAX_HEADERS: usize = 32 * 1024;

pub(crate) struct Request {
    pub method: String,
    pub target: String,
    pub protocol: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub remaining: Vec<u8>,
}
impl Request {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
    pub fn path(&self) -> &str {
        self.target.split('?').next().unwrap_or("")
    }
    pub fn query(&self) -> Vec<(String, String)> {
        self.target
            .split_once('?')
            .map(|(_, value)| fields(value))
            .unwrap_or_default()
    }
}

/// Each request has a total deadline, not merely a per-read timeout.
pub(crate) fn read(stream: &mut TcpStream, stopping: &AtomicBool) -> Result<Request, u16> {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut bytes = Vec::new();
    let header_end = loop {
        if let Some(index) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
            break index + 4;
        }
        if bytes.len() > MAX_HEADERS {
            return Err(431);
        }
        read_more(stream, &mut bytes, stopping, deadline)?;
    };
    if header_end > MAX_HEADERS {
        return Err(431);
    }
    let header = std::str::from_utf8(&bytes[..header_end]).map_err(|_| 400u16)?;
    let mut lines = header.split("\r\n");
    let mut parts = lines.next().unwrap_or("").split_whitespace();
    let method = parts.next().ok_or(400u16)?.to_owned();
    let target = parts.next().ok_or(400u16)?.to_owned();
    let protocol = parts.next().ok_or(400u16)?.to_owned();
    if parts.next().is_some()
        || !target.starts_with('/')
        || !["HTTP/1.0", "HTTP/1.1"].contains(&protocol.as_str())
        || method.bytes().any(|v| !v.is_ascii_alphabetic())
    {
        return Err(400);
    }
    let mut headers = Vec::new();
    for line in lines.take_while(|line| !line.is_empty()) {
        let (name, value) = line.split_once(':').ok_or(400u16)?;
        if name.is_empty()
            || name
                .bytes()
                .any(|v| !v.is_ascii_alphanumeric() && v != b'-')
            || value.bytes().any(|v| v < 32 && v != b'\t')
        {
            return Err(400);
        }
        headers.push((name.to_owned(), value.trim().to_owned()));
    }
    let sizes: Vec<_> = headers
        .iter()
        .filter(|(name, _)| name.eq_ignore_ascii_case("Content-Length"))
        .collect();
    let encoding: Vec<_> = headers
        .iter()
        .filter(|(name, _)| name.eq_ignore_ascii_case("Transfer-Encoding"))
        .collect();
    if sizes.len() > 1 || encoding.len() > 1 || (!encoding.is_empty() && !sizes.is_empty()) {
        return Err(400);
    }
    if encoding
        .first()
        .is_some_and(|(_, value)| !value.eq_ignore_ascii_case("chunked"))
    {
        return Err(400);
    }
    let size = sizes
        .first()
        .map(|(_, value)| value.parse::<usize>().map_err(|_| 400u16))
        .transpose()?
        .unwrap_or(0);
    if size > MAX_BODY {
        return Err(413);
    }
    if headers.iter().any(|(name, value)| {
        name.eq_ignore_ascii_case("Expect") && value.eq_ignore_ascii_case("100-continue")
    }) {
        stream
            .write_all(b"HTTP/1.1 100 Continue\r\n\r\n")
            .map_err(|_| 400u16)?;
    }
    if !encoding.is_empty() {
        let (body, consumed) = chunked_body(stream, &mut bytes, header_end, stopping, deadline)?;
        return Ok(Request {
            method,
            target,
            protocol,
            headers,
            body,
            remaining: bytes[consumed..].to_vec(),
        });
    }
    while bytes.len() < header_end + size {
        read_more(stream, &mut bytes, stopping, deadline)?;
    }
    Ok(Request {
        method,
        target,
        protocol,
        headers,
        body: bytes[header_end..header_end + size].to_vec(),
        remaining: bytes[header_end + size..].to_vec(),
    })
}

fn chunked_body(
    stream: &mut TcpStream,
    bytes: &mut Vec<u8>,
    mut at: usize,
    stopping: &AtomicBool,
    deadline: Instant,
) -> Result<(Vec<u8>, usize), u16> {
    let mut body = Vec::new();
    loop {
        let end = loop {
            if let Some(index) = bytes[at..].windows(2).position(|w| w == b"\r\n") {
                break at + index;
            }
            if bytes.len() - at > 1024 {
                return Err(400);
            }
            read_more(stream, bytes, stopping, deadline)?;
        };
        if end - at > 1024 {
            return Err(400);
        }
        let line = std::str::from_utf8(&bytes[at..end]).map_err(|_| 400u16)?;
        let digits = line.split(';').next().unwrap_or("");
        if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(400);
        }
        let size = usize::from_str_radix(digits, 16).map_err(|_| 400u16)?;
        at = end + 2;
        if size == 0 {
            let trailer_start = at;
            loop {
                if bytes.len() >= at + 2 && &bytes[at..at + 2] == b"\r\n" {
                    return Ok((body, at + 2));
                }
                if let Some(end) = bytes[at..].windows(4).position(|w| w == b"\r\n\r\n") {
                    if end + 4 > MAX_HEADERS {
                        return Err(431);
                    }
                    return Ok((body, at + end + 4));
                }
                if bytes.len() - trailer_start > MAX_HEADERS {
                    return Err(431);
                }
                read_more(stream, bytes, stopping, deadline)?;
            }
        }
        if size > MAX_BODY - body.len() {
            return Err(413);
        }
        while bytes.len() < at + size + 2 {
            read_more(stream, bytes, stopping, deadline)?;
        }
        if &bytes[at + size..at + size + 2] != b"\r\n" {
            return Err(400);
        }
        body.extend_from_slice(&bytes[at..at + size]);
        // Discard processed wire bytes so tiny chunks cannot accumulate unbounded metadata.
        bytes.drain(..at + size + 2);
        at = 0;
    }
}

fn read_more(
    stream: &mut TcpStream,
    bytes: &mut Vec<u8>,
    stopping: &AtomicBool,
    deadline: Instant,
) -> Result<(), u16> {
    loop {
        if stopping.load(Ordering::Relaxed) || Instant::now() > deadline {
            return Err(408);
        }
        let mut buffer = [0u8; 8192];
        match stream.read(&mut buffer) {
            Ok(0) => return Err(400),
            Ok(count) => {
                bytes.extend_from_slice(&buffer[..count]);
                return Ok(());
            }
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock
                        | io::ErrorKind::TimedOut
                        | io::ErrorKind::Interrupted
                ) =>
            {
                continue
            }
            Err(_) => return Err(400),
        }
    }
}

pub(crate) fn write(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &[u8],
    head: bool,
    extra: &str,
) -> io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nContent-Type: {}\r\nConnection: close\r\n{}\r\n",
        status,
        reason(status),
        body.len(),
        content_type,
        extra
    )?;
    if !head {
        stream.write_all(body)?;
    }
    Ok(())
}
pub(crate) fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        204 => "No Content",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        408 => "Request Timeout",
        413 => "Payload Too Large",
        431 => "Request Header Fields Too Large",
        503 => "Service Unavailable",
        _ => "Internal Server Error",
    }
}

pub(crate) fn decode(text: &str, plus: bool) -> Option<String> {
    let bytes = text.as_bytes();
    let mut result = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        match bytes[at] {
            b'%' => {
                let hi = (*bytes.get(at + 1)? as char).to_digit(16)?;
                let lo = (*bytes.get(at + 2)? as char).to_digit(16)?;
                result.push((hi * 16 + lo) as u8);
                at += 3;
            }
            b'+' if plus => {
                result.push(b' ');
                at += 1;
            }
            value => {
                result.push(value);
                at += 1;
            }
        }
    }
    String::from_utf8(result).ok()
}
fn fields(text: &str) -> Vec<(String, String)> {
    text.split('&')
        .filter(|part| !part.is_empty())
        .filter_map(|part| {
            let (key, value) = part.split_once('=').unwrap_or((part, ""));
            Some((decode(key, true)?, decode(value, true)?))
        })
        .collect()
}

pub(crate) fn route(request: &Request) -> Result<(Action, ApiInput), u16> {
    let path = decode(request.path(), false).ok_or(400u16)?;
    let parts: Vec<_> = path.trim_matches('/').split('/').collect();
    if !(3..=4).contains(&parts.len()) || !parts[0].eq_ignore_ascii_case("api") {
        return Err(404);
    }
    let controller = parts[1].to_ascii_lowercase();
    let action = parts[2].to_ascii_lowercase();
    use Action::*;
    let (action, verb) = match (controller.as_str(), action.as_str()) {
        ("element", "get" | "getelements") => (Elements, None),
        ("element", "searchelements") => (SearchElements, Some("GET")),
        ("element", "getchildelements") => (ChildElements, None),
        ("element", "getparentelements") => (ParentElements, None),
        ("element", "on") => (ElementOn, Some("POST")),
        ("element", "groupon") => (GroupOn, Some("POST")),
        ("element", "groupoff") => (GroupOff, Some("POST")),
        ("element", "off") => (ElementOff, Some("POST")),
        ("element", "clearall") => (ClearElements, Some("POST")),
        ("element", "effect") => (ElementEffect, Some("POST")),
        ("play", "status") => (PlayStatus, Some("GET")),
        ("play", "getsequences") => (Sequences, None),
        ("play", "playsequence") => (PlaySequence, Some("POST")),
        ("play", "pausesequence") => (PauseSequence, Some("POST")),
        ("play", "stopsequence") => (StopSequence, Some("POST")),
        ("system", "getcontrollers") => (Controllers, None),
        ("system", "get" | "getcontroller") => (Controller, None),
        ("system", "setcontrollerstate") => (SetControllerState, Some("POST")),
        ("system", "setallcontrollersstate") => (SetAllControllersState, Some("POST")),
        ("system", "save") => (SaveSystem, Some("POST")),
        _ => return Err(404),
    };
    if verb.is_some_and(|verb| request.method != verb) {
        return Err(405);
    }
    let mime = request
        .header("Content-Type")
        .unwrap_or("")
        .split(';')
        .next()
        .unwrap_or("")
        .trim();
    let mut input = ApiInput::default();
    if mime.eq_ignore_ascii_case("application/x-www-form-urlencoded") {
        input.fields = fields(&String::from_utf8_lossy(&request.body));
    }
    if let Some(id) = parts.get(3) {
        input.fields.push(("id".into(), (*id).into()));
    }
    input.fields.extend(request.query());
    if matches!(action, GroupOn | GroupOff)
        && (mime.eq_ignore_ascii_case("application/json") || mime.ends_with("+json"))
    {
        input.json = serde_json::from_slice(&request.body).ok();
    }
    Ok((action, input))
}
