//! No-network audit (spec §8), backend half: our own crates declare no
//! HTTP/network client dependencies and their sources open no sockets.
//! (Tauri pulls an HTTP client in transitively; nothing here calls it, and
//! the webview half of the audit lives in ui/src/audit.)

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

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// Our crates: the app plus everything under crates/.
fn crate_dirs() -> Vec<PathBuf> {
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
