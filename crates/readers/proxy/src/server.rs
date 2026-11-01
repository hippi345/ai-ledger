//! The opt-in localhost logging proxy.
//!
//! Security model, which is the reason this crate is exempt from the
//! no-network audit:
//! - It listens on `127.0.0.1` only. The bind address is not configurable.
//! - It forwards only to two fixed upstreams, chosen by path prefix:
//!   `/anthropic/...` goes to `https://api.anthropic.com/...` and
//!   `/openai/...` goes to `https://api.openai.com/...`.
//! - It rejects any request whose `Host` isn't localhost (this blocks DNS
//!   rebinding) and any request carrying an `Origin` header, so web pages
//!   can't use it.
//! - It never logs or stores headers. API keys and auth headers are passed
//!   through to the upstream and dropped. Only usage metadata is recorded
//!   (see [`crate::usage`]).
//!
//! Starting it is the caller's decision; the app keeps it off by default.

use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use ledger_core::ingest::NewEvent;

use crate::usage::{event_from_response, Vendor};

const MAX_HEADER_BYTES: usize = 64 * 1024;
const MAX_BODY_BYTES: usize = 32 * 1024 * 1024;
/// Response bytes kept for usage parsing; the rest is streamed only.
const MAX_CAPTURE_BYTES: usize = 8 * 1024 * 1024;

/// Headers not forwarded in either direction.
const HOP: &[&str] = &[
    "host",
    "connection",
    "keep-alive",
    "proxy-connection",
    "proxy-authorization",
    "te",
    "trailer",
    "transfer-encoding",
    "upgrade",
    "content-length",
    "accept-encoding",
    "content-encoding",
];

/// Base URL of each fixed upstream.
pub fn upstream_base(v: Vendor) -> &'static str {
    match v {
        Vendor::Anthropic => "https://api.anthropic.com",
        Vendor::OpenAi => "https://api.openai.com",
    }
}

/// A response from the upstream, with its body streamed.
pub struct UpResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Box<dyn Read + Send>,
}

/// Sends one request upstream. Swapped out in tests.
pub trait Upstream: Send + Sync {
    fn send(
        &self,
        vendor: Vendor,
        method: &str,
        path_and_query: &str,
        headers: &[(String, String)],
        body: &[u8],
    ) -> io::Result<UpResponse>;
}

/// The real upstream: HTTPS via ureq + rustls to the fixed hosts.
pub struct HttpsUpstream {
    agent: ureq::Agent,
}

impl Default for HttpsUpstream {
    fn default() -> Self {
        Self {
            agent: ureq::AgentBuilder::new()
                .timeout_connect(Duration::from_secs(15))
                .redirects(0)
                .build(),
        }
    }
}

impl Upstream for HttpsUpstream {
    fn send(
        &self,
        vendor: Vendor,
        method: &str,
        path_and_query: &str,
        headers: &[(String, String)],
        body: &[u8],
    ) -> io::Result<UpResponse> {
        let url = format!("{}{}", upstream_base(vendor), path_and_query);
        let mut req = self.agent.request(method, &url);
        for (k, v) in headers {
            req = req.set(k, v);
        }
        // Ask for an uncompressed body so usage can be read from it.
        req = req.set("Accept-Encoding", "identity");
        let resp = match req.send_bytes(body) {
            Ok(r) | Err(ureq::Error::Status(_, r)) => r,
            // Only the error kind is kept; it never includes headers.
            Err(e) => return Err(io::Error::other(e.kind().to_string())),
        };
        let headers = resp
            .headers_names()
            .into_iter()
            .filter_map(|n| resp.header(&n).map(|v| (n.clone(), v.to_owned())))
            .collect();
        Ok(UpResponse {
            status: resp.status(),
            headers,
            body: Box::new(resp.into_reader()),
        })
    }
}

/// Receives each recorded usage event.
pub type Sink = Arc<dyn Fn(NewEvent) + Send + Sync>;
/// Supplies the event timestamp (UTC ISO-8601).
pub type Clock = Arc<dyn Fn() -> String + Send + Sync>;

/// A running proxy. Dropping it stops the listener.
pub struct Handle {
    addr: SocketAddr,
    stop: Arc<AtomicBool>,
}

impl Handle {
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn stop(&self) {
        if !self.stop.swap(true, Ordering::SeqCst) {
            // Wake the accept loop.
            let _ = TcpStream::connect(self.addr);
        }
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Starts the proxy on `127.0.0.1:port` (port 0 picks a free port).
pub fn start(
    port: u16,
    upstream: Arc<dyn Upstream>,
    sink: Sink,
    clock: Clock,
) -> io::Result<Handle> {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, port))?;
    let addr = listener.local_addr()?;
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    thread::Builder::new()
        .name("ledger-proxy".into())
        .spawn(move || {
            for conn in listener.incoming() {
                if flag.load(Ordering::SeqCst) {
                    break;
                }
                let Ok(conn) = conn else { continue };
                let (up, sink, clock) = (upstream.clone(), sink.clone(), clock.clone());
                let _ = thread::Builder::new()
                    .name("ledger-proxy-conn".into())
                    .spawn(move || {
                        let _ = handle(conn, up.as_ref(), &sink, &clock);
                    });
            }
        })?;
    Ok(Handle { addr, stop })
}

struct Request {
    method: String,
    target: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

fn header<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
}

type Reject = (u16, &'static str);

fn read_request(r: &mut impl BufRead) -> Result<Request, Reject> {
    let mut head = Vec::new();
    loop {
        let mut line = Vec::new();
        let n = r
            .by_ref()
            .take((MAX_HEADER_BYTES + 1) as u64)
            .read_until(b'\n', &mut line)
            .map_err(|_| (400, "bad request"))?;
        if n == 0 {
            return Err((400, "bad request"));
        }
        head.extend_from_slice(&line);
        if head.len() > MAX_HEADER_BYTES {
            return Err((431, "headers too large"));
        }
        if line == b"\r\n" || line == b"\n" {
            break;
        }
    }
    let text = std::str::from_utf8(&head).map_err(|_| (400, "bad request"))?;
    let mut lines = text.lines();
    let mut start = lines.next().unwrap_or("").split_whitespace();
    let (Some(method), Some(target)) = (start.next(), start.next()) else {
        return Err((400, "bad request"));
    };
    let headers: Vec<(String, String)> = lines
        .filter_map(|l| l.split_once(':'))
        .map(|(k, v)| (k.trim().to_owned(), v.trim().to_owned()))
        .collect();
    if header(&headers, "transfer-encoding").is_some() {
        return Err((411, "chunked request bodies are not supported"));
    }
    let len: usize = match header(&headers, "content-length") {
        None => 0,
        Some(v) => v.parse().map_err(|_| (400, "bad content-length"))?,
    };
    if len > MAX_BODY_BYTES {
        return Err((413, "body too large"));
    }
    let mut body = vec![0; len];
    r.read_exact(&mut body).map_err(|_| (400, "short body"))?;
    Ok(Request {
        method: method.to_owned(),
        target: target.to_owned(),
        headers,
        body,
    })
}

/// True when `host` names this machine's loopback interface.
fn is_local_host(host: &str) -> bool {
    let name = match host.rsplit_once(':') {
        Some((n, port)) if port.chars().all(|c| c.is_ascii_digit()) => n,
        _ => host,
    };
    matches!(
        name.to_ascii_lowercase().as_str(),
        "127.0.0.1" | "localhost"
    )
}

/// Checks Host and Origin, then picks the upstream and path.
fn route(req: &Request) -> Result<(Vendor, String), Reject> {
    if !header(&req.headers, "host").is_some_and(is_local_host) {
        return Err((403, "host must be localhost"));
    }
    if header(&req.headers, "origin").is_some() {
        return Err((403, "browser requests are not allowed"));
    }
    let (vendor, rest) = if let Some(r) = req.target.strip_prefix("/anthropic/") {
        (Vendor::Anthropic, r)
    } else if let Some(r) = req.target.strip_prefix("/openai/") {
        (Vendor::OpenAi, r)
    } else {
        return Err((404, "use /anthropic/... or /openai/..."));
    };
    Ok((vendor, format!("/{rest}")))
}

fn write_error(w: &mut impl Write, (status, msg): Reject) -> io::Result<()> {
    write!(
        w,
        "HTTP/1.1 {status} Error\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{msg}",
        msg.len()
    )
}

fn handle(conn: TcpStream, up: &dyn Upstream, sink: &Sink, clock: &Clock) -> io::Result<()> {
    conn.set_read_timeout(Some(Duration::from_secs(60)))?;
    let mut out = conn.try_clone()?;
    let mut rd = BufReader::new(conn);
    let req = match read_request(&mut rd) {
        Ok(r) => r,
        Err(e) => return write_error(&mut out, e),
    };
    let (vendor, path) = match route(&req) {
        Ok(v) => v,
        Err(e) => return write_error(&mut out, e),
    };
    let fwd: Vec<(String, String)> = req
        .headers
        .into_iter()
        .filter(|(k, _)| !HOP.contains(&k.to_ascii_lowercase().as_str()))
        .collect();
    let ts = clock();
    let Ok(mut resp) = up.send(vendor, &req.method, &path, &fwd, &req.body) else {
        return write_error(&mut out, (502, "upstream unreachable"));
    };
    write!(out, "HTTP/1.1 {} Proxied\r\n", resp.status)?;
    for (k, v) in &resp.headers {
        if !HOP.contains(&k.to_ascii_lowercase().as_str()) {
            write!(out, "{k}: {v}\r\n")?;
        }
    }
    out.write_all(b"Transfer-Encoding: chunked\r\nConnection: close\r\n\r\n")?;
    let mut captured = Vec::new();
    let mut buf = [0u8; 16 * 1024];
    loop {
        let n = resp.body.read(&mut buf)?;
        if n == 0 {
            break;
        }
        write!(out, "{n:x}\r\n")?;
        out.write_all(&buf[..n])?;
        out.write_all(b"\r\n")?;
        out.flush()?;
        if captured.len() + n <= MAX_CAPTURE_BYTES {
            captured.extend_from_slice(&buf[..n]);
        }
    }
    if (200..300).contains(&resp.status) {
        if let Some(ev) = event_from_response(vendor, &captured, &ts) {
            sink(ev);
        }
    }
    out.write_all(b"0\r\n\r\n")?;
    out.flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    const KEY: &str = "sk-ant-TESTKEY-must-not-be-stored";

    type Seen = (Vendor, String, String, Vec<(String, String)>, Vec<u8>);

    #[derive(Default)]
    struct Fake {
        seen: Mutex<Vec<Seen>>,
        status: u16,
        body: &'static str,
    }

    impl Upstream for Fake {
        fn send(
            &self,
            vendor: Vendor,
            method: &str,
            path: &str,
            headers: &[(String, String)],
            body: &[u8],
        ) -> io::Result<UpResponse> {
            self.seen.lock().unwrap().push((
                vendor,
                method.into(),
                path.into(),
                headers.to_vec(),
                body.to_vec(),
            ));
            Ok(UpResponse {
                status: self.status,
                headers: vec![
                    ("content-type".into(), "application/json".into()),
                    ("content-length".into(), "999".into()),
                ],
                body: Box::new(io::Cursor::new(self.body.as_bytes())),
            })
        }
    }

    const MSG: &str = r#"{"id":"msg_1","model":"claude-sonnet-4-5","content":[{"type":"text","text":"hello"}],"usage":{"input_tokens":3,"output_tokens":4}}"#;

    fn run(fake: Arc<Fake>, raw: &str) -> (String, Vec<NewEvent>) {
        let events = Arc::new(Mutex::new(Vec::new()));
        let ev2 = events.clone();
        let h = start(
            0,
            fake,
            Arc::new(move |e| ev2.lock().unwrap().push(e)),
            Arc::new(|| "2026-11-01T12:00:00Z".into()),
        )
        .unwrap();
        let mut c = TcpStream::connect(h.addr()).unwrap();
        c.write_all(
            raw.replace("{port}", &h.addr().port().to_string())
                .as_bytes(),
        )
        .unwrap();
        let mut resp = String::new();
        c.read_to_string(&mut resp).unwrap();
        let evs = events.lock().unwrap().clone();
        (resp, evs)
    }

    fn post(host: &str, extra: &str) -> String {
        format!(
            "POST /anthropic/v1/messages?beta=1 HTTP/1.1\r\nHost: {host}\r\nx-api-key: {KEY}\r\nAuthorization: Bearer {KEY}\r\nAccept-Encoding: gzip\r\n{extra}Content-Length: 2\r\n\r\n{{}}"
        )
    }

    fn ok_fake() -> Arc<Fake> {
        Arc::new(Fake {
            status: 200,
            body: MSG,
            ..Fake::default()
        })
    }

    #[test]
    fn binds_loopback_only() {
        let h = start(0, ok_fake(), Arc::new(|_| {}), Arc::new(String::new)).unwrap();
        assert_eq!(h.addr().ip(), Ipv4Addr::LOCALHOST);
    }

    #[test]
    fn forwards_streams_back_and_records_usage() {
        let fake = ok_fake();
        let (resp, evs) = run(fake.clone(), &post("127.0.0.1:{port}", ""));
        assert!(resp.starts_with("HTTP/1.1 200"), "{resp}");
        assert!(resp.contains("hello"));
        assert!(!resp.to_ascii_lowercase().contains("content-length: 999"));
        let seen = fake.seen.lock().unwrap();
        let (v, m, p, hs, b) = &seen[0];
        assert_eq!(
            (*v, m.as_str(), p.as_str()),
            (Vendor::Anthropic, "POST", "/v1/messages?beta=1")
        );
        assert_eq!(b, b"{}");
        // Auth goes upstream unchanged; hop-by-hop and encoding headers don't.
        assert!(header(hs, "x-api-key") == Some(KEY));
        assert!(header(hs, "host").is_none() && header(hs, "accept-encoding").is_none());
        assert_eq!(evs.len(), 1);
        assert_eq!((evs[0].input_tokens, evs[0].output_tokens), (3, 4));
    }

    #[test]
    fn api_keys_and_auth_headers_are_never_stored_or_logged() {
        let (_, evs) = run(ok_fake(), &post("localhost:{port}", ""));
        assert_eq!(evs.len(), 1);
        let dump = format!("{evs:?}");
        assert!(!dump.contains("TESTKEY"));
        assert!(!dump.to_ascii_lowercase().contains("bearer"));
        // The proxy sources never print or log. The patterns are built at
        // runtime so this test doesn't match itself.
        let pats = ["println", "eprintln", "dbg", "tracing", "log"].map(|p| {
            format!(
                "{p}{}",
                if p == "tracing" || p == "log" {
                    "::"
                } else {
                    "!"
                }
            )
        });
        for src in [
            include_str!("server.rs"),
            include_str!("usage.rs"),
            include_str!("lib.rs"),
        ] {
            for pat in &pats {
                assert!(!src.contains(pat.as_str()), "{pat} used in proxy sources");
            }
        }
    }

    #[test]
    fn rejects_foreign_host_origin_and_unknown_routes() {
        for (raw, code) in [
            (post("evil.example.com", ""), "403"),
            (post("127.0.0.1.evil.com:{port}", ""), "403"),
            (
                post("localhost:{port}", "Origin: https://evil.example.com\r\n"),
                "403",
            ),
            (
                "POST /v1/messages HTTP/1.1\r\nHost: localhost\r\n\r\n".to_owned(),
                "404",
            ),
            ("POST /anthropic/x HTTP/1.1\r\n\r\n".to_owned(), "403"),
            (
                "POST /openai/x HTTP/1.1\r\nHost: localhost\r\nTransfer-Encoding: chunked\r\n\r\n"
                    .to_owned(),
                "411",
            ),
        ] {
            let fake = ok_fake();
            let (resp, evs) = run(fake.clone(), &raw);
            assert!(
                resp.starts_with(&format!("HTTP/1.1 {code}")),
                "{raw} -> {resp}"
            );
            assert!(fake.seen.lock().unwrap().is_empty(), "forwarded {raw}");
            assert!(evs.is_empty());
        }
    }

    #[test]
    fn upstream_errors_pass_through_without_events() {
        let fake = Arc::new(Fake {
            status: 401,
            body: r#"{"error":"bad key","usage":{"input_tokens":1}}"#,
            ..Fake::default()
        });
        let (resp, evs) = run(fake, &post("localhost", ""));
        assert!(resp.starts_with("HTTP/1.1 401"));
        assert!(evs.is_empty());
    }

    #[test]
    fn fixed_upstreams_and_host_check() {
        assert_eq!(
            upstream_base(Vendor::Anthropic),
            "https://api.anthropic.com"
        );
        assert_eq!(upstream_base(Vendor::OpenAi), "https://api.openai.com");
        assert!(is_local_host("localhost:8080") && is_local_host("127.0.0.1"));
        assert!(!is_local_host("localhost.evil.com") && !is_local_host(&["0"; 4].join(".")));
    }
}
