//! The TCP server (docs/specs/RPC.md §1, §3, §5).

use std::io::{BufRead, BufReader, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::{Map, Value};

use crate::config::ct_eq;
use crate::host::{Host, dispatch};
use crate::protocol::{RpcError, code, failure, success};

/// Longest accepted request line, in bytes.
pub const MAX_MESSAGE_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_REQUESTS_PER_SECOND: u32 = 50;
pub const MAX_CONNECTIONS: usize = 4;

const IDLE_TIMEOUT: Duration = Duration::from_secs(600);
const REPLY_TIMEOUT: Duration = Duration::from_secs(30);
const TICK: Duration = Duration::from_millis(100);
/// Jobs the UI thread runs per frame.
const JOBS_PER_DRAIN: usize = 16;

#[derive(Debug, Clone, Default)]
pub struct ServerConfig {
    /// 0 asks the OS for a free port.
    pub port: u16,
    /// Required in every request. `None` only works with `insecure_local`.
    pub token: Option<String>,
    /// Allow running without a token (`--rpc-insecure-local`).
    pub insecure_local: bool,
}

#[derive(Debug)]
pub enum StartError {
    /// No token configured and insecure-local was not asked for.
    NoToken,
    Bind(std::io::Error),
}

impl std::fmt::Display for StartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StartError::NoToken => write!(
                f,
                "no token configured (set FORGE_RPC_TOKEN or rpc.token, or allow --rpc-insecure-local)"
            ),
            StartError::Bind(e) => write!(f, "cannot listen on 127.0.0.1: {e}"),
        }
    }
}

impl std::error::Error for StartError {}

struct Job {
    method: String,
    params: Value,
    reply: Sender<Result<Value, RpcError>>,
}

type Wake = Arc<dyn Fn() + Send + Sync>;

/// The receiving end: the thread that owns the document drains it.
pub struct Inbox {
    rx: Receiver<Job>,
    wake: Arc<Mutex<Option<Wake>>>,
}

impl Inbox {
    /// Called by the server after it queues a job; the editor passes a repaint request.
    pub fn set_wake(&self, wake: impl Fn() + Send + Sync + 'static) {
        *self.wake.lock().unwrap_or_else(|e| e.into_inner()) = Some(Arc::new(wake));
    }

    /// Runs the queued jobs (at most 16) on `host` and sends their replies. Returns how
    /// many ran. A panicking method becomes an internal error, not a dead editor.
    pub fn drain(&self, host: &mut dyn Host) -> usize {
        let mut ran = 0;
        while ran < JOBS_PER_DRAIN {
            match self.rx.try_recv() {
                Ok(job) => {
                    run_job(host, job);
                    ran += 1;
                }
                Err(_) => break,
            }
        }
        ran
    }

    /// Serves jobs on the calling thread until `stop` is set (headless hosts, tests).
    pub fn run_until(&self, host: &mut dyn Host, stop: &AtomicBool) {
        while !stop.load(Ordering::SeqCst) {
            match self.rx.recv_timeout(Duration::from_millis(20)) {
                Ok(job) => run_job(host, job),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
    }
}

fn run_job(host: &mut dyn Host, job: Job) {
    let Job {
        method,
        params,
        reply,
    } = job;
    let result = catch_unwind(AssertUnwindSafe(|| dispatch(host, &method, params)))
        .unwrap_or_else(|_| Err(RpcError::new(code::INTERNAL, "internal error")));
    let _ = reply.send(result);
}

struct Shared {
    token: Option<String>,
    jobs: Sender<Job>,
    wake: Arc<Mutex<Option<Wake>>>,
    stop: Arc<AtomicBool>,
}

/// The listening side. Dropping it stops accepting and closes the connections.
pub struct Server {
    addr: SocketAddr,
    stop: Arc<AtomicBool>,
    accept: Option<std::thread::JoinHandle<()>>,
}

impl Server {
    /// Binds `127.0.0.1:port` and starts serving. The returned [`Inbox`] must be drained
    /// by the thread that owns the document, or requests time out.
    pub fn start(config: ServerConfig) -> Result<(Server, Inbox), StartError> {
        let token = config.token.filter(|t| !t.is_empty());
        if token.is_none() && !config.insecure_local {
            return Err(StartError::NoToken);
        }
        let listener = TcpListener::bind(("127.0.0.1", config.port)).map_err(StartError::Bind)?;
        listener.set_nonblocking(true).map_err(StartError::Bind)?;
        let addr = listener.local_addr().map_err(StartError::Bind)?;
        let (tx, rx) = mpsc::channel();
        let wake = Arc::new(Mutex::new(None));
        let stop = Arc::new(AtomicBool::new(false));
        let shared = Arc::new(Shared {
            token,
            jobs: tx,
            wake: wake.clone(),
            stop: stop.clone(),
        });
        let accept = std::thread::Builder::new()
            .name("forge-rpc-accept".into())
            .spawn(move || accept_loop(listener, shared))
            .map_err(StartError::Bind)?;
        Ok((
            Server {
                addr,
                stop,
                accept: Some(accept),
            },
            Inbox { rx, wake },
        ))
    }

    /// The bound address (the real port when 0 was asked for).
    pub fn local_addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn port(&self) -> u16 {
        self.addr.port()
    }

    /// Stops accepting; open connections close within a fraction of a second.
    pub fn shutdown(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(h) = self.accept.take() {
            let _ = h.join();
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn accept_loop(listener: TcpListener, shared: Arc<Shared>) {
    let live = Arc::new(AtomicUsize::new(0));
    while !shared.stop.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((mut stream, _)) => {
                let _ = stream.set_nonblocking(false);
                if live.load(Ordering::SeqCst) >= MAX_CONNECTIONS {
                    let e = RpcError::new(code::TOO_MANY_CONNECTIONS, "too many connections");
                    let _ = write_line(&mut stream, &failure(&Value::Null, &e));
                    let _ = stream.shutdown(Shutdown::Both);
                    continue;
                }
                live.fetch_add(1, Ordering::SeqCst);
                let (shared, live) = (shared.clone(), live.clone());
                let spawned = std::thread::Builder::new()
                    .name("forge-rpc-conn".into())
                    .spawn({
                        let live = live.clone();
                        move || {
                            serve(stream, &shared);
                            live.fetch_sub(1, Ordering::SeqCst);
                        }
                    });
                if spawned.is_err() {
                    live.fetch_sub(1, Ordering::SeqCst);
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(_) => std::thread::sleep(Duration::from_millis(50)),
        }
    }
}

fn write_line(stream: &mut TcpStream, value: &Value) -> std::io::Result<()> {
    let mut text = value.to_string();
    text.push('\n');
    stream.write_all(text.as_bytes())?;
    stream.flush()
}

enum Read {
    Line,
    Eof,
    TooLarge,
    /// The read timed out; partial data stays in the buffer.
    Tick,
}

/// Reads up to the next newline into `buf` without ever holding more than the limit.
fn read_line(r: &mut BufReader<TcpStream>, buf: &mut Vec<u8>) -> std::io::Result<Read> {
    use std::io::ErrorKind::{Interrupted, TimedOut, WouldBlock};
    loop {
        let chunk = match r.fill_buf() {
            Ok(c) => c,
            Err(e) if matches!(e.kind(), WouldBlock | TimedOut) => return Ok(Read::Tick),
            Err(e) if e.kind() == Interrupted => continue,
            Err(e) => return Err(e),
        };
        if chunk.is_empty() {
            return Ok(Read::Eof);
        }
        let newline = chunk.iter().position(|b| *b == b'\n');
        let content = newline.unwrap_or(chunk.len());
        if buf.len() + content > MAX_MESSAGE_BYTES {
            return Ok(Read::TooLarge);
        }
        let take = newline.map_or(chunk.len(), |p| p + 1);
        buf.extend_from_slice(&chunk[..take]);
        r.consume(take);
        if newline.is_some() {
            return Ok(Read::Line);
        }
    }
}

/// 50 requests per second, burst 50.
struct Bucket {
    tokens: f64,
    last: Instant,
}

impl Bucket {
    fn new() -> Bucket {
        Bucket {
            tokens: f64::from(MAX_REQUESTS_PER_SECOND),
            last: Instant::now(),
        }
    }

    fn take(&mut self) -> bool {
        let rate = f64::from(MAX_REQUESTS_PER_SECOND);
        let now = Instant::now();
        self.tokens = (self.tokens + now.duration_since(self.last).as_secs_f64() * rate).min(rate);
        self.last = now;
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

/// What to do after answering one line.
enum After {
    Continue,
    Close,
}

fn serve(stream: TcpStream, shared: &Shared) {
    if stream.set_read_timeout(Some(TICK)).is_err() {
        return;
    }
    let _ = stream.set_nodelay(true);
    let Ok(mut out) = stream.try_clone() else {
        return;
    };
    let mut reader = BufReader::new(stream);
    let mut buf = Vec::new();
    let mut bucket = Bucket::new();
    let mut last_activity = Instant::now();
    while !shared.stop.load(Ordering::SeqCst) {
        match read_line(&mut reader, &mut buf) {
            Ok(Read::Line) => {
                last_activity = Instant::now();
                while buf.last().is_some_and(|b| *b == b'\n' || *b == b'\r') {
                    buf.pop();
                }
                let line = std::mem::take(&mut buf);
                if line.iter().all(u8::is_ascii_whitespace) {
                    continue;
                }
                let (response, after) = handle_line(&line, shared, &mut bucket);
                if write_line(&mut out, &response).is_err() || matches!(after, After::Close) {
                    break;
                }
            }
            Ok(Read::Tick) => {
                if last_activity.elapsed() > IDLE_TIMEOUT {
                    break;
                }
            }
            Ok(Read::TooLarge) => {
                let e = RpcError::new(
                    code::TOO_LARGE,
                    format!("message exceeds {MAX_MESSAGE_BYTES} bytes"),
                );
                let _ = write_line(&mut out, &failure(&Value::Null, &e));
                // Read and discard what the client is still sending, so closing does not
                // reset the connection before it has read the reply.
                let _ = out.shutdown(Shutdown::Write);
                drain_input(&mut reader);
                break;
            }
            Ok(Read::Eof) | Err(_) => break,
        }
    }
    let _ = out.shutdown(Shutdown::Both);
}

/// Discards input until EOF, for at most two seconds.
fn drain_input(reader: &mut BufReader<TcpStream>) {
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        match reader.fill_buf() {
            Ok([]) => return,
            Ok(chunk) => {
                let n = chunk.len();
                reader.consume(n);
            }
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) => {}
            Err(_) => return,
        }
    }
}

/// Validates one request line and runs it. Order: parse, shape and id, rate limit,
/// token, method fields, dispatch.
fn handle_line(line: &[u8], shared: &Shared, bucket: &mut Bucket) -> (Value, After) {
    let null = Value::Null;
    let fail = |id: &Value, code: i64, message: &str| failure(id, &RpcError::new(code, message));
    let value: Value = match serde_json::from_slice(line) {
        Ok(v) => v,
        Err(e) => {
            let m = format!("parse error: {e}");
            return (fail(&null, code::PARSE_ERROR, &m), After::Continue);
        }
    };
    let Value::Object(request) = value else {
        let m = "request must be a JSON object";
        return (fail(&null, code::INVALID_REQUEST, m), After::Continue);
    };
    let id = match request.get("id") {
        Some(id @ (Value::Number(_) | Value::String(_))) => id.clone(),
        _ => {
            let m = "request needs a numeric or string id";
            return (fail(&null, code::INVALID_REQUEST, m), After::Continue);
        }
    };
    if !bucket.take() {
        let m = "rate limit exceeded (50 requests per second)";
        return (fail(&id, code::RATE_LIMITED, m), After::Continue);
    }
    if let Some(expected) = &shared.token {
        let given = request.get("token").and_then(Value::as_str).unwrap_or("");
        if !ct_eq(expected.as_bytes(), given.as_bytes()) {
            return (fail(&id, code::UNAUTHORIZED, "unauthorized"), After::Close);
        }
    }
    let result = run_request(&request, shared);
    let response = match result {
        Ok(v) => success(&id, v),
        Err(e) => failure(&id, &e),
    };
    (response, After::Continue)
}

fn run_request(request: &Map<String, Value>, shared: &Shared) -> Result<Value, RpcError> {
    if request.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        return Err(RpcError::new(
            code::INVALID_REQUEST,
            "\"jsonrpc\" must be \"2.0\"",
        ));
    }
    let method = request
        .get("method")
        .and_then(Value::as_str)
        .ok_or_else(|| RpcError::new(code::INVALID_REQUEST, "\"method\" must be a string"))?;
    let params = match request.get("params") {
        None | Some(Value::Null) => Value::Object(Map::new()),
        Some(p @ Value::Object(_)) => p.clone(),
        Some(_) => return Err(RpcError::invalid_params("\"params\" must be an object")),
    };
    let (reply, wait) = mpsc::channel();
    let job = Job {
        method: method.to_string(),
        params,
        reply,
    };
    shared
        .jobs
        .send(job)
        .map_err(|_| RpcError::new(code::INTERNAL, "the editor is not serving requests"))?;
    let wake = shared
        .wake
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    if let Some(wake) = wake {
        wake();
    }
    match wait.recv_timeout(REPLY_TIMEOUT) {
        Ok(result) => result,
        Err(RecvTimeoutError::Timeout) => Err(RpcError::new(
            code::TIMEOUT,
            "the editor did not answer in time",
        )),
        Err(RecvTimeoutError::Disconnected) => Err(RpcError::new(
            code::INTERNAL,
            "the editor dropped the request",
        )),
    }
}
