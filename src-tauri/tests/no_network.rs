//! No-network audit (spec §8), backend half: our own crates declare no
//! HTTP/network client dependencies and their sources open no sockets.
//! (Tauri pulls an HTTP client in transitively; nothing here calls it, and
//! the webview half of the audit lives in ui/src/audit.)
//!
//! One scoped exception: `crates/readers/proxy`, the opt-in localhost
//! logging proxy (spec item 21, off by default). It may use sockets and
//! `ureq`, but [`proxy_is_localhost_only`] pins it to 127.0.0.1 and to the
//! two fixed upstreams.

use std::fs;
use std::path::{Path, PathBuf};

const BANNED_DEPS: &[&str] = &[
    "reqwest",
    "ureq",
    "hyper",
    "isahc",
    "curl",
    "attohttpc",
    "surf",
    "tungstenite",
    "tokio-tungstenite",
    "tauri-plugin-http",
    "tauri-plugin-updater",
    "tauri-plugin-shell",
    "tauri-plugin-websocket",
    "tauri-plugin-upload",
];

const BANNED_CODE: &[&str] = &[
    "std::net",
    "TcpStream",
    "TcpListener",
    "UdpSocket",
    "reqwest::",
    "ureq::",
    "hyper::",
];

/// The one crate allowed to use the network (see module docs).
const PROXY_CRATE: &str = "crates/readers/proxy";

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// Our crates except the proxy: the app plus everything under crates/.
fn crate_dirs() -> Vec<PathBuf> {
    let proxy = repo().join(PROXY_CRATE);
    all_crate_dirs()
        .into_iter()
        .filter(|d| *d != proxy)
        .collect()
}

fn all_crate_dirs() -> Vec<PathBuf> {
    let mut out = vec![repo().join("src-tauri"), repo().join("crates/ledger-core")];
    if let Ok(rd) = fs::read_dir(repo().join("crates/readers")) {
        out.extend(
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| p.join("Cargo.toml").exists()),
        );
    }
    out
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in fs::read_dir(dir).into_iter().flatten().flatten() {
        let p = e.path();
        if p.is_dir() {
            if !p.ends_with("target") && !p.ends_with("tests") && !p.ends_with("gen") {
                rust_files(&p, out);
            }
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

#[test]
fn no_network_client_dependencies() {
    for dir in crate_dirs() {
        let toml = fs::read_to_string(dir.join("Cargo.toml")).unwrap_or_default();
        for line in toml.lines().map(str::trim) {
            let name = line.split(['=', ' ', '.']).next().unwrap_or("");
            assert!(
                !BANNED_DEPS.contains(&name),
                "{}: depends on {name}",
                dir.display()
            );
        }
    }
}

#[test]
fn no_socket_code() {
    for dir in crate_dirs() {
        let mut files = vec![];
        rust_files(&dir.join("src"), &mut files);
        assert!(!files.is_empty(), "no sources found in {}", dir.display());
        for f in files {
            let src = fs::read_to_string(&f).unwrap_or_default();
            for pat in BANNED_CODE {
                assert!(!src.contains(pat), "{} uses {pat}", f.display());
            }
        }
    }
}

#[test]
fn proxy_is_localhost_only() {
    let dir = repo().join(PROXY_CRATE);
    assert!(
        all_crate_dirs().contains(&dir),
        "proxy crate moved; update PROXY_CRATE"
    );
    let mut files = vec![];
    rust_files(&dir.join("src"), &mut files);
    let src: String = files
        .iter()
        .map(|f| fs::read_to_string(f).unwrap_or_default())
        .collect();
    assert_eq!(
        src.matches("TcpListener::bind(").count(),
        1,
        "proxy must have exactly one listener"
    );
    assert!(src.contains("TcpListener::bind((Ipv4Addr::LOCALHOST, port))"));
    for bad in ["0.0.0.0", "UNSPECIFIED", "Ipv6Addr", "UdpSocket"] {
        assert!(!src.contains(bad), "proxy uses {bad}");
    }
    // Only the two fixed upstreams appear as URLs.
    let urls: Vec<&str> = src
        .match_indices("https://")
        .map(|(i, _)| {
            let rest = &src[i..];
            &rest[..rest
                .find(|c: char| c == '"' || c.is_whitespace())
                .unwrap_or(rest.len())]
        })
        .collect();
    for u in urls {
        assert!(
            u == "https://api.anthropic.com"
                || u == "https://api.openai.com"
                || u.starts_with("https://api.anthropic.com/")
                || u.starts_with("https://api.openai.com/")
                || u.starts_with("https://evil.example.com"),
            "unexpected upstream {u}"
        );
    }
}
