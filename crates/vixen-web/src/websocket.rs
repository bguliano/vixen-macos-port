use crate::{
    http::{self, Request, MAX_BODY},
    server::Shared,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use std::{
    io::{self, Read, Write},
    net::{Shutdown, TcpStream},
    sync::{atomic::Ordering, mpsc, Arc},
    time::{Duration, Instant},
};

pub(crate) fn serve(mut stream: TcpStream, request: Request, shared: Arc<Shared>) -> u16 {
    let fail = |stream: &mut TcpStream, status| {
        let _ = http::write(stream, status, "text/plain", &[], false, "");
        status
    };
    if request.method != "GET" {
        return fail(&mut stream, 405);
    }
    let key = request.header("Sec-WebSocket-Key").unwrap_or("");
    if !request
        .header("Upgrade")
        .is_some_and(|value| value.eq_ignore_ascii_case("websocket"))
        || !request.header("Connection").is_some_and(|value| {
            value
                .split(',')
                .any(|part| part.trim().eq_ignore_ascii_case("upgrade"))
        })
        || request.header("Sec-WebSocket-Version") != Some("13")
        || STANDARD
            .decode(key)
            .ok()
            .is_none_or(|bytes| bytes.len() != 16)
    {
        return fail(&mut stream, 400);
    }
    let token = request
        .query()
        .into_iter()
        .find(|(key, _)| key.eq_ignore_ascii_case("id"))
        .map(|(_, value)| value)
        .unwrap_or_default();
    if shared
        .tokens
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(&token)
        .is_none_or(|created| created.elapsed() > Duration::from_secs(60))
    {
        return fail(&mut stream, 404);
    }
    if write!(stream, "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {}\r\n\r\n", accept_key(key)).is_err() { return 101; }
    let (sender, receiver) = mpsc::sync_channel(4);
    let mut wire = request.remaining;
    let mut fragments: Option<Vec<u8>> = None;
    let mut records = Vec::new();
    let mut ready = false;
    let mut close_sent = false;
    let connected = Instant::now();
    let mut last_message = Instant::now();
    'connection: while !shared.stopping.load(Ordering::Relaxed) {
        while let Some(frame) = match next_frame(&mut wire) {
            Ok(frame) => frame,
            Err(code) => {
                let _ = close_frame(&mut stream, &mut close_sent, &code.to_be_bytes());
                break 'connection;
            }
        } {
            match frame.opcode {
                8 => {
                    let invalid = if frame.payload.len() == 1 {
                        Some(1002u16)
                    } else if frame.payload.len() >= 2 {
                        let code = u16::from_be_bytes([frame.payload[0], frame.payload[1]]);
                        if !matches!(code, 1000..=1003 | 1007..=1014 | 3000..=4999) {
                            Some(1002)
                        } else if std::str::from_utf8(&frame.payload[2..]).is_err() {
                            Some(1007)
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    let payload = invalid
                        .map(|code| code.to_be_bytes().to_vec())
                        .unwrap_or(frame.payload);
                    let _ = close_frame(&mut stream, &mut close_sent, &payload);
                    close_sent = true;
                    break 'connection;
                }
                9 => {
                    if send_frame(&mut stream, 10, &frame.payload).is_err() {
                        break 'connection;
                    }
                    continue;
                }
                10 => continue,
                2 => {
                    let _ = close_frame(&mut stream, &mut close_sent, &1003u16.to_be_bytes());
                    break 'connection;
                }
                1 if fragments.is_none() => {
                    if frame.fin {
                        // A complete WebSocket text message must itself be UTF-8;
                        // SignalR records may span messages, UTF-8 characters may not.
                        if std::str::from_utf8(&frame.payload).is_err() {
                            let _ =
                                close_frame(&mut stream, &mut close_sent, &1007u16.to_be_bytes());
                            break 'connection;
                        }
                        records.extend_from_slice(&frame.payload);
                    } else {
                        fragments = Some(frame.payload);
                        continue;
                    }
                }
                0 if fragments.is_some() => {
                    let fragment = fragments.as_mut().unwrap();
                    fragment.extend_from_slice(&frame.payload);
                    if fragment.len() > MAX_BODY {
                        let _ = close_frame(&mut stream, &mut close_sent, &1009u16.to_be_bytes());
                        break 'connection;
                    }
                    if frame.fin {
                        if std::str::from_utf8(fragment).is_err() {
                            let _ =
                                close_frame(&mut stream, &mut close_sent, &1007u16.to_be_bytes());
                            break 'connection;
                        }
                        records.extend_from_slice(&fragments.take().unwrap());
                    } else {
                        continue;
                    }
                }
                _ => {
                    let _ = close_frame(&mut stream, &mut close_sent, &1002u16.to_be_bytes());
                    break 'connection;
                }
            }
            if records.len() > MAX_BODY {
                let _ = close_frame(&mut stream, &mut close_sent, &1009u16.to_be_bytes());
                break 'connection;
            }
            while let Some(end) = records.iter().position(|byte| *byte == 0x1e) {
                let record: Vec<_> = records.drain(..=end).collect();
                let value: serde_json::Value = match serde_json::from_slice(&record[..end]) {
                    Ok(value) => value,
                    Err(_) => {
                        let _ = close_frame(&mut stream, &mut close_sent, &1007u16.to_be_bytes());
                        break 'connection;
                    }
                };
                last_message = Instant::now();
                if !ready {
                    if value.get("protocol").and_then(|v| v.as_str()) != Some("json")
                        || value.get("version").and_then(|v| v.as_u64()) != Some(1)
                    {
                        let _ = send_frame(
                            &mut stream,
                            1,
                            b"{\"error\":\"Unsupported protocol or version.\"}\x1e",
                        );
                        break 'connection;
                    }
                    if send_frame(&mut stream, 1, b"{}\x1e").is_err() {
                        break 'connection;
                    }
                    ready = true;
                    shared
                        .clients
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .insert(token.clone(), sender.clone());
                } else if value.get("type").and_then(|v| v.as_u64()) == Some(7) {
                    break 'connection;
                }
            }
        }
        if (!ready && connected.elapsed() > Duration::from_secs(15))
            || last_message.elapsed() > Duration::from_secs(60)
        {
            break;
        }
        while let Ok(message) = receiver.try_recv() {
            if send_frame(&mut stream, 1, message.as_bytes()).is_err() {
                break 'connection;
            }
        }
        if ready
            && !shared
                .clients
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .contains_key(&token)
        {
            break;
        }
        let mut buffer = [0u8; 8192];
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(count) => {
                wire.extend_from_slice(&buffer[..count]);
                if wire.len() > MAX_BODY + 14 {
                    let _ = close_frame(&mut stream, &mut close_sent, &1009u16.to_be_bytes());
                    break;
                }
            }
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock
                        | io::ErrorKind::TimedOut
                        | io::ErrorKind::Interrupted
                ) => {}
            Err(_) => break,
        }
    }
    shared
        .clients
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(&token);
    if !close_sent {
        let _ = close_frame(&mut stream, &mut close_sent, &1000u16.to_be_bytes());
    }
    // A peer may have another frame in flight when we reject a message. Dropping a
    // socket with unread input can send RST and discard the close frame's payload.
    // Half-close and drain briefly so the peer can observe the WebSocket close.
    let _ = stream.shutdown(Shutdown::Write);
    let _ = stream.set_read_timeout(Some(Duration::from_millis(20)));
    let deadline = Instant::now() + Duration::from_millis(200);
    let mut drained = 0;
    let mut buffer = [0u8; 8192];
    while !shared.stopping.load(Ordering::Relaxed)
        && Instant::now() < deadline
        && drained < 64 * 1024
    {
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(count) => drained += count,
            Err(error) if matches!(error.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut | io::ErrorKind::Interrupted) => {},
            Err(_) => break,
        }
    }
    101
}

fn close_frame(stream: &mut TcpStream, sent: &mut bool, payload: &[u8]) -> io::Result<()> {
    *sent = true;
    send_frame(stream, 8, payload)
}

struct Frame {
    fin: bool,
    opcode: u8,
    payload: Vec<u8>,
}
fn next_frame(bytes: &mut Vec<u8>) -> Result<Option<Frame>, u16> {
    if bytes.len() < 2 {
        return Ok(None);
    }
    let fin = bytes[0] & 128 != 0;
    let opcode = bytes[0] & 15;
    if bytes[0] & 0x70 != 0 || bytes[1] & 128 == 0 {
        return Err(1002);
    }
    let mut offset = 2;
    let short = bytes[1] & 127;
    let len = match short {
        126 => {
            if bytes.len() < 4 {
                return Ok(None);
            }
            offset = 4;
            let len = u16::from_be_bytes([bytes[2], bytes[3]]) as u64;
            if len < 126 {
                return Err(1002);
            }
            len
        }
        127 => {
            if bytes.len() < 10 {
                return Ok(None);
            }
            offset = 10;
            let len = u64::from_be_bytes(bytes[2..10].try_into().unwrap());
            if len < 65536 || len >> 63 != 0 {
                return Err(1002);
            }
            len
        }
        value => value as u64,
    };
    if opcode >= 8 && (!fin || len > 125) {
        return Err(1002);
    }
    if len > MAX_BODY as u64 {
        return Err(1009);
    }
    let len = len as usize;
    if bytes.len() < offset + 4 + len {
        return Ok(None);
    }
    let mask: [u8; 4] = bytes[offset..offset + 4].try_into().unwrap();
    offset += 4;
    let payload = bytes[offset..offset + len]
        .iter()
        .enumerate()
        .map(|(i, byte)| byte ^ mask[i % 4])
        .collect();
    bytes.drain(..offset + len);
    Ok(Some(Frame {
        fin,
        opcode,
        payload,
    }))
}
pub(crate) fn send_frame(stream: &mut TcpStream, opcode: u8, payload: &[u8]) -> io::Result<()> {
    let mut header = vec![0x80 | opcode];
    match payload.len() {
        0..=125 => header.push(payload.len() as u8),
        126..=65535 => {
            header.push(126);
            header.extend_from_slice(&(payload.len() as u16).to_be_bytes());
        }
        _ => {
            header.push(127);
            header.extend_from_slice(&(payload.len() as u64).to_be_bytes());
        }
    }
    stream.write_all(&header)?;
    stream.write_all(payload)
}

fn accept_key(key: &str) -> String {
    STANDARD.encode(sha1(
        format!("{key}258EAFA5-E914-47DA-95CA-C5AB0DC85B11").as_bytes(),
    ))
}
/// SHA-1 is required by RFC 6455 solely for the upgrade token, never for security.
fn sha1(input: &[u8]) -> [u8; 20] {
    let mut padded = input.to_vec();
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&((input.len() as u64) * 8).to_be_bytes());
    let mut hash = [
        0x67452301u32,
        0xefcdab89,
        0x98badcfe,
        0x10325476,
        0xc3d2e1f0,
    ];
    for chunk in padded.chunks_exact(64) {
        let mut words = [0u32; 80];
        for (i, part) in chunk.chunks_exact(4).enumerate() {
            words[i] = u32::from_be_bytes(part.try_into().unwrap());
        }
        for i in 16..80 {
            words[i] = (words[i - 3] ^ words[i - 8] ^ words[i - 14] ^ words[i - 16]).rotate_left(1);
        }
        let [mut a, mut b, mut c, mut d, mut e] = hash;
        for (i, word) in words.into_iter().enumerate() {
            let (f, k) = match i {
                0..=19 => ((b & c) | (!b & d), 0x5a827999u32),
                20..=39 => (b ^ c ^ d, 0x6ed9eba1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8f1bbcdc),
                _ => (b ^ c ^ d, 0xca62c1d6),
            };
            let temp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(word);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }
        for (part, value) in hash.iter_mut().zip([a, b, c, d, e]) {
            *part = part.wrapping_add(value);
        }
    }
    let mut result = [0u8; 20];
    for (part, word) in result.chunks_exact_mut(4).zip(hash) {
        part.copy_from_slice(&word.to_be_bytes());
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rfc6455_handshake_and_sha1_vectors() {
        assert_eq!(
            accept_key("dGhlIHNhbXBsZSBub25jZQ=="),
            "s3pPLMBiTxaQ9kYGzzhZRbK+xOo="
        );
        let hex = |bytes: [u8; 20]| bytes.iter().map(|v| format!("{v:02x}")).collect::<String>();
        assert_eq!(hex(sha1(b"")), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
        assert_eq!(
            hex(sha1(b"abc")),
            "a9993e364706816aba3e25717850c26c9cd0d89d"
        );
        assert_eq!(
            hex(sha1(&vec![b'a'; 1_000_000])),
            "34aa973cd4c4daa4f61eeb2bdbad27316534016f"
        );
    }
}
