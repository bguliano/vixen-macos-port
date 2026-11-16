use crate::{http, websocket, ApiError, ApiResponse, Logger, WebApi};
use std::{
    cell::Cell,
    collections::HashMap,
    io,
    net::{SocketAddr, TcpListener, TcpStream},
    path::{Component, Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{SyncSender, TrySendError},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

thread_local! {
    // A callback may release the last AppCore, whose destructor stops this server.
    static ACTIVE_SERVER: Cell<usize> = const { Cell::new(0) };
}
struct WorkerScope(usize);
impl WorkerScope {
    fn enter(shared: &Arc<Shared>) -> Self {
        Self(ACTIVE_SERVER.with(|active| active.replace(Arc::as_ptr(shared) as usize)))
    }
}
impl Drop for WorkerScope {
    fn drop(&mut self) {
        ACTIVE_SERVER.with(|active| active.set(self.0));
    }
}

pub(crate) struct Shared {
    pub stopping: AtomicBool,
    pub api: Arc<dyn WebApi>,
    pub logger: Logger,
    pub clients: Mutex<HashMap<String, SyncSender<String>>>,
    pub tokens: Mutex<HashMap<String, Instant>>,
}

/// Bounded TCP workers and one shared 1 Hz context broadcast clock.
/// Drop and stop close all clients and join workers before returning. When called
/// from this server's own callback, a reaper joins them after the callback exits.
pub struct Server {
    address: SocketAddr,
    shared: Arc<Shared>,
    accept: Option<JoinHandle<()>>,
    broadcast: Option<JoinHandle<()>>,
}
impl Server {
    pub fn start(
        bind: SocketAddr,
        wwwroot: PathBuf,
        api: Arc<dyn WebApi>,
        logger: Logger,
    ) -> io::Result<Self> {
        let root = wwwroot.canonicalize()?;
        if !root.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::NotADirectory,
                "Web content is not a directory",
            ));
        }
        let listener = if bind.is_ipv6() {
            // Explicit dual-stack wildcard listener, matching upstream's http://*.
            // A specific ::1 bind remains IPv6 loopback only.
            let socket = socket2::Socket::new(
                socket2::Domain::IPV6,
                socket2::Type::STREAM,
                Some(socket2::Protocol::TCP),
            )?;
            socket.set_only_v6(false)?;
            socket.bind(&bind.into())?;
            socket.listen(128)?;
            TcpListener::from(socket)
        } else {
            TcpListener::bind(bind)?
        };
        listener.set_nonblocking(true)?;
        let address = listener.local_addr()?;
        let shared = Arc::new(Shared {
            stopping: AtomicBool::new(false),
            api,
            logger,
            clients: Mutex::new(HashMap::new()),
            tokens: Mutex::new(HashMap::new()),
        });
        let accepted = shared.clone();
        let accept = thread::Builder::new()
            .name("vixen-web-http".into())
            .spawn(move || {
                let _scope = WorkerScope::enter(&accepted);
                let mut workers: Vec<JoinHandle<()>> = Vec::new();
                while !accepted.stopping.load(Ordering::Relaxed) {
                    let mut at = 0;
                    while at < workers.len() {
                        if workers[at].is_finished() {
                            let _ = workers.swap_remove(at).join();
                        } else {
                            at += 1;
                        }
                    }
                    match listener.accept() {
                        Ok((mut stream, peer)) => {
                            let _ = stream.set_read_timeout(Some(Duration::from_millis(100)));
                            let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
                            if workers.len() >= 64 {
                                let _ = http::write(&mut stream, 503, "text/plain", &[], false, "");
                                continue;
                            }
                            let state = accepted.clone();
                            let root = root.clone();
                            match thread::Builder::new()
                                .name("vixen-web-client".into())
                                .spawn(move || serve(stream, peer, &root, state))
                            {
                                Ok(worker) => workers.push(worker),
                                Err(error) => {
                                    (accepted.logger)(&format!("Exception in web server: {error}"))
                                }
                            }
                        }
                        Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(20))
                        }
                        Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                        Err(error) => {
                            (accepted.logger)(&format!("Exception in web server: {error}"));
                            break;
                        }
                    }
                }
                for worker in workers {
                    let _ = worker.join();
                }
            })?;
        let broadcast_state = shared.clone();
        let broadcast = match thread::Builder::new()
            .name("vixen-web-contexts".into())
            .spawn(move || broadcast(broadcast_state))
        {
            Ok(handle) => handle,
            Err(error) => {
                shared.stopping.store(true, Ordering::Relaxed);
                let _ = accept.join();
                return Err(error);
            }
        };
        Ok(Self {
            address,
            shared,
            accept: Some(accept),
            broadcast: Some(broadcast),
        })
    }
    pub fn address(&self) -> SocketAddr {
        self.address
    }
    pub fn stop(&mut self) {
        self.shared.stopping.store(true, Ordering::Relaxed);
        self.shared
            .clients
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
        let handles: Vec<_> = [self.broadcast.take(), self.accept.take()]
            .into_iter()
            .flatten()
            .collect();
        if handles.is_empty() {
            return;
        }
        let join = move || {
            for handle in handles {
                let _ = handle.join();
            }
        };
        if ACTIVE_SERVER.with(|active| active.get() == Arc::as_ptr(&self.shared) as usize) {
            // Joining accept here would wait for this worker to return: self-deadlock.
            // If spawning fails, dropping the join closure safely detaches the handles;
            // the stop flag still makes all loops and cooperative API calls terminate.
            let _ = thread::Builder::new()
                .name("vixen-web-shutdown".into())
                .spawn(join);
        } else {
            join();
        }
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.stop();
    }
}

fn broadcast(shared: Arc<Shared>) {
    let _scope = WorkerScope::enter(&shared);
    let mut next = Instant::now() + Duration::from_secs(1);
    let mut last_ping = Instant::now();
    while !shared.stopping.load(Ordering::Relaxed) {
        thread::sleep(Duration::from_millis(25));
        if Instant::now() < next {
            continue;
        }
        next = Instant::now() + Duration::from_secs(1);
        if shared
            .clients
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_empty()
        {
            continue;
        }
        let states =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| shared.api.context_states()));
        let states = match states {
            Ok(states) => states,
            Err(_) => {
                (shared.logger)("Exception in web server: context state callback panicked");
                continue;
            }
        };
        let mut message = serde_json::json!({"type":1,"target":"updatePlayingContextStates","arguments":[states]}).to_string();
        message.push('\u{1e}');
        let ping = last_ping.elapsed() >= Duration::from_secs(15);
        if ping {
            message.push_str("{\"type\":6}\u{1e}");
            last_ping = Instant::now();
        }
        shared
            .clients
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .retain(|_, client| match client.try_send(message.clone()) {
                Ok(()) => true,
                Err(TrySendError::Disconnected(_) | TrySendError::Full(_)) => false,
            });
    }
}

fn serve(mut stream: TcpStream, peer: SocketAddr, root: &Path, shared: Arc<Shared>) {
    let _scope = WorkerScope::enter(&shared);
    let request = match http::read(&mut stream, &shared.stopping) {
        Ok(request) => request,
        Err(status) => {
            let _ = http::write(&mut stream, status, "text/plain", &[], false, "");
            return;
        }
    };
    let path = request.path().trim_end_matches('/').to_owned();
    if path.eq_ignore_ascii_case("/hubs/contextStates") {
        let method = request.method.clone();
        let protocol = request.protocol.clone();
        let status = websocket::serve(stream, request, shared.clone());
        log(&shared, peer, &method, &path, &protocol, status, 0);
        return;
    }
    let (status, mime, body, extra) = if path.eq_ignore_ascii_case("/hubs/contextStates/negotiate")
    {
        if request.method != "POST" {
            (405, "text/plain", Vec::new(), "Allow: POST\r\n")
        } else {
            let connection = uuid::Uuid::new_v4().to_string();
            let token = uuid::Uuid::new_v4().to_string();
            let mut tokens = shared.tokens.lock().unwrap_or_else(|e| e.into_inner());
            tokens.retain(|_, created| created.elapsed() < Duration::from_secs(60));
            if tokens.len() >= 256 {
                (503, "text/plain", Vec::new(), "")
            } else {
                tokens.insert(token.clone(), Instant::now());
                (200, "application/json; charset=utf-8", serde_json::json!({"negotiateVersion":1,"connectionId":connection,"connectionToken":token,"availableTransports":[{"transport":"WebSockets","transferFormats":["Text","Binary"]}]}).to_string().into_bytes(), "")
            }
        }
    } else if path
        .get(..5)
        .is_some_and(|s| s.eq_ignore_ascii_case("/api/"))
        || path.eq_ignore_ascii_case("/api")
    {
        match http::route(&request) {
            Err(status) => (status, "text/plain", Vec::new(), ""),
            Ok((action, input)) => {
                let response = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    shared
                        .api
                        .invoke_cancellable(action, input, &shared.stopping)
                }))
                .unwrap_or_else(|_| Err(ApiError::Internal("API callback panicked".into())));
                match response {
                    Ok(ApiResponse::Json(value)) => (
                        200,
                        "application/json; charset=utf-8",
                        value.to_string().into_bytes(),
                        "",
                    ),
                    Ok(ApiResponse::NoContent) => (204, "text/plain", Vec::new(), ""),
                    Err(ApiError::Internal(error)) => {
                        (shared.logger)(&format!("Exception in web server: {error}"));
                        (500, "text/plain", Vec::new(), "")
                    }
                    Err(ApiError::BadRequest(error)) => {
                        let message = error.replace("\r\n", " ").replace(['\r', '\n'], " ");
                        let accept = request
                            .header("Accept")
                            .unwrap_or("*/*")
                            .to_ascii_lowercase();
                        if accept.contains("application/json")
                            && !accept.contains("*/*")
                            && !accept.contains("text/plain")
                        {
                            (
                                400,
                                "application/json; charset=utf-8",
                                serde_json::to_vec(&message).unwrap_or_default(),
                                "",
                            )
                        } else {
                            (400, "text/plain; charset=utf-8", message.into_bytes(), "")
                        }
                    }
                }
            }
        }
    } else if request.method == "GET" || request.method == "HEAD" {
        match static_file(root, request.path()) {
            Some((mime, bytes)) => (200, mime, bytes, ""),
            None => (404, "text/plain", Vec::new(), ""),
        }
    } else {
        (404, "text/plain", Vec::new(), "")
    };
    let _ = http::write(
        &mut stream,
        status,
        mime,
        &body,
        request.method == "HEAD",
        extra,
    );
    log(
        &shared,
        peer,
        &request.method,
        request.path(),
        &request.protocol,
        status,
        body.len(),
    );
}
fn log(
    shared: &Shared,
    peer: SocketAddr,
    method: &str,
    path: &str,
    protocol: &str,
    status: u16,
    size: usize,
) {
    let ip = if peer.ip() == std::net::IpAddr::V6(std::net::Ipv6Addr::LOCALHOST) {
        "127.0.0.1".to_owned()
    } else {
        peer.ip().to_string()
    };
    (shared.logger)(&format!(
        "{ip} - \"{method} {path} {protocol}\" {status} {size}"
    ));
}
fn static_file(root: &Path, path: &str) -> Option<(&'static str, Vec<u8>)> {
    let decoded = http::decode(path, false)?;
    if decoded.contains(['\\', '\0']) {
        return None;
    }
    let relative = Path::new(decoded.trim_start_matches('/'));
    if relative
        .components()
        .any(|part| !matches!(part, Component::Normal(_) | Component::CurDir))
    {
        return None;
    }
    let mut file = root.join(relative);
    if file.is_dir() {
        file.push("index.htm");
    }
    let file = file.canonicalize().ok()?;
    if !file.starts_with(root) || !file.is_file() {
        return None;
    }
    let mime = match file
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "htm" | "html" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" => "application/javascript; charset=utf-8",
        "json" | "map" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "gif" => "image/gif",
        "jpg" | "jpeg" => "image/jpeg",
        "ico" => "image/x-icon",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "eot" => "application/vnd.ms-fontobject",
        _ => "application/octet-stream",
    };
    std::fs::read(file).ok().map(|bytes| (mime, bytes))
}
