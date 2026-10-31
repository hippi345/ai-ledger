//! Locating Cursor's data directories on macOS, Windows, and Linux.
//!
//! Cursor is a VS Code fork and keeps its user data under a `Cursor/User`
//! directory in the platform's config location:
//!
//! | OS      | User data dir                                        |
//! |---------|------------------------------------------------------|
//! | macOS   | `~/Library/Application Support/Cursor/User`          |
//! | Windows | `%APPDATA%\Cursor\User`                              |
//! | Linux   | `$XDG_CONFIG_HOME/Cursor/User` (default `~/.config`) |
//!
//! Inside it, `globalStorage/state.vscdb` holds global state and
//! `workspaceStorage/<hash>/state.vscdb` holds per-workspace state, with
//! `workspace.json` naming the opened folder. Logs live in `../logs`.

use std::path::{Path, PathBuf};

/// Target platform for path resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    MacOs,
    Windows,
    Linux,
}

impl Os {
    /// The platform this binary was built for.
    pub fn current() -> Self {
        if cfg!(target_os = "macos") {
            Os::MacOs
        } else if cfg!(windows) {
            Os::Windows
        } else {
            Os::Linux
        }
    }
}

/// Environment inputs needed to resolve paths (injectable for tests).
#[derive(Debug, Clone, Default)]
pub struct Env {
    pub home: Option<PathBuf>,
    pub appdata: Option<PathBuf>,
    pub xdg_config_home: Option<PathBuf>,
}

impl Env {
    /// Reads the real process environment.
    pub fn from_process() -> Self {
        let var = |k: &str| {
            std::env::var_os(k)
                .filter(|v| !v.is_empty())
                .map(PathBuf::from)
        };
        Self {
            home: var("HOME").or_else(|| var("USERPROFILE")),
            appdata: var("APPDATA"),
            xdg_config_home: var("XDG_CONFIG_HOME"),
        }
    }
}

/// Cursor's root data dir (the parent of `User` and `logs`), if resolvable.
pub fn cursor_root(os: Os, env: &Env) -> Option<PathBuf> {
    match os {
        Os::MacOs => env
            .home
            .as_ref()
            .map(|h| h.join("Library").join("Application Support").join("Cursor")),
        Os::Windows => env
            .appdata
            .clone()
            .or_else(|| env.home.as_ref().map(|h| h.join("AppData").join("Roaming")))
            .map(|a| a.join("Cursor")),
        Os::Linux => env
            .xdg_config_home
            .clone()
            .or_else(|| env.home.as_ref().map(|h| h.join(".config")))
            .map(|c| c.join("Cursor")),
    }
}

/// Resolved locations inside a Cursor root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CursorPaths {
    pub root: PathBuf,
    pub global_db: PathBuf,
    pub workspace_storage: PathBuf,
    pub logs: PathBuf,
}

impl CursorPaths {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        let user = root.join("User");
        Self {
            global_db: user.join("globalStorage").join("state.vscdb"),
            workspace_storage: user.join("workspaceStorage"),
            logs: root.join("logs"),
            root,
        }
    }

    /// Resolves paths for `os`/`env` without touching the filesystem.
    pub fn resolve(os: Os, env: &Env) -> Option<Self> {
        cursor_root(os, env).map(Self::new)
    }

    /// Whether Cursor appears to be installed and used here.
    pub fn exists(&self) -> bool {
        self.global_db.is_file() || self.workspace_storage.is_dir()
    }

    /// All per-workspace `state.vscdb` files, sorted for determinism.
    pub fn workspace_dbs(&self) -> Vec<PathBuf> {
        list_workspace_dbs(&self.workspace_storage)
    }
}

fn list_workspace_dbs(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path().join("state.vscdb"))
        .filter(|p| p.is_file())
        .collect();
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env() -> Env {
        Env {
            home: Some("/home/joel".into()),
            appdata: None,
            xdg_config_home: None,
        }
    }

    #[test]
    fn macos_layout() {
        let p = CursorPaths::resolve(Os::MacOs, &env()).unwrap();
        assert_eq!(
            p.global_db,
            Path::new(
                "/home/joel/Library/Application Support/Cursor/User/globalStorage/state.vscdb"
            )
        );
        assert_eq!(
            p.logs,
            Path::new("/home/joel/Library/Application Support/Cursor/logs")
        );
    }

    #[test]
    fn windows_prefers_appdata() {
        let mut e = env();
        assert_eq!(
            cursor_root(Os::Windows, &e).unwrap(),
            Path::new("/home/joel/AppData/Roaming/Cursor")
        );
        e.appdata = Some("/roam".into());
        assert_eq!(
            cursor_root(Os::Windows, &e).unwrap(),
            Path::new("/roam/Cursor")
        );
    }

    #[test]
    fn linux_respects_xdg() {
        let mut e = env();
        assert_eq!(
            cursor_root(Os::Linux, &e).unwrap(),
            Path::new("/home/joel/.config/Cursor")
        );
        e.xdg_config_home = Some("/xdg".into());
        assert_eq!(
            cursor_root(Os::Linux, &e).unwrap(),
            Path::new("/xdg/Cursor")
        );
    }

    #[test]
    fn unresolvable_without_env() {
        assert!(CursorPaths::resolve(Os::Linux, &Env::default()).is_none());
    }

    #[test]
    fn finds_workspace_dbs_on_disk() {
        let tmp = tempfile::tempdir().unwrap();
        let p = CursorPaths::new(tmp.path());
        assert!(!p.exists());
        for ws in ["b2", "a1"] {
            let d = p.workspace_storage.join(ws);
            std::fs::create_dir_all(&d).unwrap();
            std::fs::write(d.join("state.vscdb"), b"").unwrap();
        }
        std::fs::create_dir_all(p.workspace_storage.join("empty")).unwrap();
        assert!(p.exists());
        let dbs = p.workspace_dbs();
        assert_eq!(dbs.len(), 2);
        assert!(dbs[0].ends_with("a1/state.vscdb"));
    }
}
