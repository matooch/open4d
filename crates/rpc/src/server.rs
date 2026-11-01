//! The TCP server (docs/specs/RPC.md §1, §3, §5).

use std::io::{BufRead, BufReader, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::{Map, Value, json};

use crate::config::ct_eq;
use crate::host::{Host, dispatch};
use crate::hub::{Hub, Kind, Registration, notification};
use crate::jobs::{self, State, Step, Table};
use crate::protocol::{RpcError, code, failure, success};

/// Longest accepted request line, in bytes.
pub const MAX_MESSAGE_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_REQUESTS_PER_SECOND: u32 = 50;
pub const MAX_CONNECTIONS: usize = 4;

const IDLE_TIMEOUT: Duration = Duration::from_secs(600);
const REPLY_TIMEOUT: Duration = Duration::from_secs(30);
/// How often a connection thread looks for notifications to write.
const TICK: Duration = Duration::from_millis(20);
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
    /// The connection that sent the request (owner of a job it starts).
    conn: u64,
    method: String,
    params: Value,
    reply: Sender<Result<Value, RpcError>>,
}

type Wake = Arc<dyn Fn() + Send + Sync>;

/// The receiving end: the thread that owns the document drains it.
pub struct Inbox {
    rx: Receiver<Job>,
    wake: Arc<Mutex<Option<Wake>>>,
    hub: Arc<Hub>,
    jobs: Mutex<Table>,
    watch: Mutex<Watch>,
}

/// What the notifications compare against (docs/specs/RPC.md §10.1).
#[derive(Default)]
struct Watch {
    seen: Option<Seen>,
}

#[derive(Clone, PartialEq)]
struct Seen {
    revision: u64,
    frame: i64,
    selected: Vec<u64>,
    /// Identity and size of the command registry.
    commands: (usize, usize),
}

impl Seen {
    fn of(host: &dyn Host) -> Seen {
        let app = host.app();
        Seen {
            revision: app.doc.scene().revision(),
            frame: app.timeline.current,
            selected: app.selection.nodes.iter().map(|n| n.0).collect(),
            commands: (Arc::as_ptr(&app.commands) as usize, app.commands.len()),
        }
    }
}

impl Inbox {
    /// Called by the server after it queues a job; the editor passes a repaint request.
    pub fn set_wake(&self, wake: impl Fn() + Send + Sync + 'static) {
        *self.wake.lock().unwrap_or_else(|e| e.into_inner()) = Some(Arc::new(wake));
    }

    /// Runs the queued requests (at most 16) on `host` and sends their replies, runs one
    /// slice of each long job and sends the notifications. Returns how much work was done
    /// (requests plus job slices); the editor repaints while it is not zero. A panicking
    /// method becomes an internal error, not a dead editor.
    pub fn drain(&self, host: &mut dyn Host) -> usize {
        self.watch(host);
        let mut ran = 0;
        while ran < JOBS_PER_DRAIN {
            match self.rx.try_recv() {
                Ok(job) => {
                    self.run_job(host, job);
                    ran += 1;
                }
                Err(_) => break,
            }
        }
        ran += self.step_jobs(host);
        self.watch(host);
        ran
    }

    /// Serves requests on the calling thread until `stop` is set (headless hosts, tests).
    pub fn run_until(&self, host: &mut dyn Host, stop: &AtomicBool) {
        while !stop.load(Ordering::SeqCst) {
            self.watch(host);
            let wait = if self.has_jobs() { 2 } else { 20 };
            match self.rx.recv_timeout(Duration::from_millis(wait)) {
                Ok(job) => self.run_job(host, job),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break,
            }
            self.step_jobs(host);
            self.watch(host);
        }
    }

    fn has_jobs(&self) -> bool {
        !self
            .jobs
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .active
            .is_empty()
    }

    /// Sends a notification for every kind whose value changed since the last look.
    fn watch(&self, host: &dyn Host) {
        let now = Seen::of(host);
        let mut watch = self.watch.lock().unwrap_or_else(|e| e.into_inner());
        let Some(before) = watch.seen.replace(now.clone()) else {
            return;
        };
        if before.revision != now.revision {
            let m = notification("scene.changed", json!({"revision": now.revision}));
            self.hub.publish(Kind::Scene, &m);
        }
        if before.frame != now.frame {
            let m = notification("frame.changed", json!({"frame": now.frame}));
            self.hub.publish(Kind::Frame, &m);
        }
        if before.selected != now.selected {
            let m = notification("selection.changed", json!({"nodes": now.selected}));
            self.hub.publish(Kind::Selection, &m);
        }
        if before.commands != now.commands {
            let m = notification("commands.changed", json!({"count": now.commands.1}));
            self.hub.publish(Kind::Commands, &m);
        }
    }

    fn run_job(&self, host: &mut dyn Host, job: Job) {
        let Job {
            conn,
            method,
            params,
            reply,
        } = job;
        let result = catch_unwind(AssertUnwindSafe(|| match method.as_str() {
            "job.start" | "job.cancel" | "job.list" => self.job_method(host, conn, &method, params),
            _ => dispatch(host, &method, params),
        }))
        .unwrap_or_else(|_| Err(RpcError::new(code::INTERNAL, "internal error")));
        let _ = reply.send(result);
    }

    fn job_method(
        &self,
        host: &dyn Host,
        conn: u64,
        method: &str,
        params: Value,
    ) -> Result<Value, RpcError> {
        let mut table = self.jobs.lock().unwrap_or_else(|e| e.into_inner());
        let field = |name: &str| params.get(name).cloned();
        let only = |allowed: &[&str]| -> Result<(), RpcError> {
            match params
                .as_object()
                .and_then(|o| o.keys().find(|k| !allowed.contains(&k.as_str())))
            {
                Some(k) => Err(RpcError::invalid_params(format!(
                    "{method}: unknown field {k}"
                ))),
                None => Ok(()),
            }
        };
        match method {
            "job.start" => {
                only(&["kind", "args"])?;
                let kind = field("kind")
                    .and_then(|k| k.as_str().map(String::from))
                    .ok_or_else(|| {
                        RpcError::invalid_params("job.start: \"kind\" must be a string")
                    })?;
                let args = match field("args") {
                    None | Some(Value::Null) => json!({}),
                    Some(a @ Value::Object(_)) => a,
                    Some(_) => {
                        return Err(RpcError::invalid_params(
                            "job.start: \"args\" must be an object",
                        ));
                    }
                };
                let job = jobs::make_job(host.app(), &kind, args)?;
                let id = table.start(&kind, conn, job)?;
                Ok(json!({ "job_id": id }))
            }
            "job.cancel" => {
                only(&["id"])?;
                let id = field("id").and_then(|v| v.as_u64()).ok_or_else(|| {
                    RpcError::invalid_params("job.cancel: \"id\" must be an integer")
                })?;
                table.cancel(id)
            }
            _ => {
                only(&[])?;
                Ok(table.list())
            }
        }
    }

    /// One slice of every active job; notifications go to the connection that started it.
    fn step_jobs(&self, host: &dyn Host) -> usize {
        let mut table = self.jobs.lock().unwrap_or_else(|e| e.into_inner());
        let active = std::mem::take(&mut table.active);
        let slices = active.len();
        for mut a in active {
            let finished = |a: &jobs::Active, state: State, extra: Value| {
                let mut p = json!({"id": a.id, "state": state.name()});
                p.as_object_mut()
                    .expect("object")
                    .extend(extra.as_object().cloned().unwrap_or_default());
                self.hub.send_to(a.conn, &notification("job.finished", p));
            };
            if a.cancel {
                finished(&a, State::Cancelled, json!({}));
                table.end(a, State::Cancelled);
                continue;
            }
            let step = catch_unwind(AssertUnwindSafe(|| a.job.step(host.app())))
                .unwrap_or_else(|_| Step::Failed("internal error".to_string()));
            match step {
                Step::Progress { fraction, message } => {
                    a.fraction = fraction.max(a.fraction);
                    a.message = message;
                    let p = json!({"id": a.id, "fraction": a.fraction, "message": a.message});
                    self.hub.send_to(a.conn, &notification("job.progress", p));
                    table.active.push(a);
                }
                Step::Done(result) => {
                    a.fraction = 1.0;
                    a.message = "done".to_string();
                    let p = json!({"id": a.id, "fraction": 1.0, "message": a.message});
                    self.hub.send_to(a.conn, &notification("job.progress", p));
                    finished(&a, State::Done, json!({ "result": result }));
                    table.end(a, State::Done);
                }
                Step::Failed(error) => {
                    a.message = error.clone();
                    finished(&a, State::Failed, json!({ "error": error }));
                    table.end(a, State::Failed);
                }
            }
        }
        slices
    }
}

struct Shared {
    hub: Arc<Hub>,
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
        let hub = Arc::new(Hub::default());
        let shared = Arc::new(Shared {
            hub: hub.clone(),
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
            Inbox {
                rx,
                wake,
                hub,
                jobs: Mutex::new(Table::default()),
                watch: Mutex::new(Watch::default()),
            },
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
    let registration = shared.hub.register();
    let id = registration.id;
    serve_connection(stream, shared, &registration);
    shared.hub.unregister(id);
}

/// Writes the notifications that wait for this connection.
fn flush_outbox(out: &mut TcpStream, registration: &Registration) -> bool {
    while let Ok(message) = registration.outbox.try_recv() {
        if write_line(out, &message).is_err() {
            return false;
        }
    }
    true
}

fn serve_connection(stream: TcpStream, shared: &Shared, registration: &Registration) {
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
        if registration.kill.load(Ordering::SeqCst) || !flush_outbox(&mut out, registration) {
            break;
        }
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
                let (response, after) = handle_line(&line, shared, registration.id, &mut bucket);
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
fn handle_line(line: &[u8], shared: &Shared, conn: u64, bucket: &mut Bucket) -> (Value, After) {
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
    let result = run_request(&request, shared, conn);
    let response = match result {
        Ok(v) => success(&id, v),
        Err(e) => failure(&id, &e),
    };
    (response, After::Continue)
}

fn run_request(
    request: &Map<String, Value>,
    shared: &Shared,
    conn: u64,
) -> Result<Value, RpcError> {
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
    if method == "events.subscribe" || method == "events.unsubscribe" {
        return events_method(shared, conn, method, &params);
    }
    let (reply, wait) = mpsc::channel();
    let job = Job {
        conn,
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

/// `events.subscribe` and `events.unsubscribe` (docs/specs/RPC.md §10.1), answered by the
/// connection thread: the subscription belongs to the connection.
fn events_method(
    shared: &Shared,
    conn: u64,
    method: &str,
    params: &Value,
) -> Result<Value, RpcError> {
    let object = params.as_object().expect("params is an object");
    if let Some(k) = object.keys().find(|k| k.as_str() != "kinds") {
        return Err(RpcError::invalid_params(format!(
            "{method}: unknown field {k}"
        )));
    }
    let kinds: Vec<Kind> = match object.get("kinds") {
        None | Some(Value::Null) => Kind::ALL.to_vec(),
        Some(Value::Array(list)) => list
            .iter()
            .map(|v| {
                v.as_str().and_then(Kind::parse).ok_or_else(|| {
                    RpcError::invalid_params(format!(
                        "{method}: unknown kind {v} (known: scene, frame, selection, commands)"
                    ))
                })
            })
            .collect::<Result<_, _>>()?,
        Some(_) => {
            return Err(RpcError::invalid_params(format!(
                "{method}: \"kinds\" must be an array of strings"
            )));
        }
    };
    let now = if method == "events.subscribe" {
        shared.hub.subscribe(conn, &kinds)
    } else {
        shared.hub.unsubscribe(conn, &kinds)
    };
    let names: Vec<&str> = now.iter().map(|k| k.name()).collect();
    Ok(json!({ "kinds": names }))
}
