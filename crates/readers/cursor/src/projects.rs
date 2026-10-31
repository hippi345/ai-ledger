//! Mapping Cursor conversations to projects.
//!
//! Each `workspaceStorage/<hash>/` dir has a `workspace.json` naming the opened
//! `folder` (or multi-root `workspace` file) as a `file://` URI, and a
//! `state.vscdb` whose `ItemTable` key `composer.composerData` lists the
//! composers started in that workspace. Joining the two gives composer → project.

use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;

use crate::paths::CursorPaths;
use crate::snapshot::Snapshot;

#[derive(Deserialize, Default)]
#[serde(default)]
struct WorkspaceJson {
    folder: Option<String>,
    workspace: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct ComposerList {
    all_composers: Vec<ComposerRef>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct ComposerRef {
    composer_id: String,
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Project name from a workspace URI: the last path segment, without a
/// `.code-workspace` suffix. Returns `None` for empty URIs.
pub fn project_name(uri: &str) -> Option<String> {
    let path = uri.split_once("://").map_or(uri, |(_, rest)| rest);
    let decoded = percent_decode(path);
    let last = decoded
        .trim_end_matches(['/', '\\'])
        .rsplit(['/', '\\'])
        .next()?;
    let name = last.strip_suffix(".code-workspace").unwrap_or(last);
    (!name.is_empty()).then(|| name.to_owned())
}

fn workspace_project(dir: &Path) -> Option<String> {
    let raw = std::fs::read(dir.join("workspace.json")).ok()?;
    let ws: WorkspaceJson = serde_json::from_slice(&raw).ok()?;
    project_name(ws.folder.or(ws.workspace)?.as_str())
}

/// composer id → project name, across all workspaces. Unreadable workspaces are skipped.
pub fn composer_projects(paths: &CursorPaths) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for db in paths.workspace_dbs() {
        let Some(dir) = db.parent() else { continue };
        let Some(project) = workspace_project(dir) else {
            continue;
        };
        let Ok(snap) = Snapshot::take(&db) else {
            continue;
        };
        let Ok(Some(raw)) = snap.get("ItemTable", "composer.composerData") else {
            continue;
        };
        let list: ComposerList = serde_json::from_str(&raw).unwrap_or_default();
        for c in list.all_composers {
            if !c.composer_id.is_empty() {
                map.insert(c.composer_id, project.clone());
            }
        }
    }
    map
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Materializes fixtures/cursor/workspaces.json under `root`.
    pub fn write_workspaces(root: &Path) {
        let p = CursorPaths::new(root);
        let f =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/cursor/workspaces.json");
        let doc: serde_json::Value = serde_json::from_slice(&std::fs::read(f).unwrap()).unwrap();
        for (hash, ws) in doc["workspaces"].as_object().unwrap() {
            let dir = p.workspace_storage.join(hash);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("workspace.json"), ws["workspace.json"].to_string()).unwrap();
            let c = rusqlite::Connection::open(dir.join("state.vscdb")).unwrap();
            c.execute_batch(
                "CREATE TABLE ItemTable (key TEXT UNIQUE ON CONFLICT REPLACE, value BLOB);",
            )
            .unwrap();
            c.execute(
                "INSERT INTO ItemTable VALUES ('composer.composerData', ?1)",
                [ws["composer.composerData"].to_string()],
            )
            .unwrap();
        }
    }

    #[test]
    fn names_from_uris() {
        assert_eq!(
            project_name("file:///home/joel/code/ai%20ledger").as_deref(),
            Some("ai ledger")
        );
        assert_eq!(
            project_name("file:///c%3A/Users/joel/proj/randobot/").as_deref(),
            Some("randobot")
        );
        assert_eq!(
            project_name("file:///x/multi.code-workspace").as_deref(),
            Some("multi")
        );
        assert_eq!(
            project_name("vscode-remote://ssh-remote%2Bbox/srv/app").as_deref(),
            Some("app")
        );
        assert_eq!(project_name("file:///"), None);
        assert_eq!(percent_decode("100%"), "100%");
    }

    #[test]
    fn maps_composers_to_projects() {
        let tmp = tempfile::tempdir().unwrap();
        write_workspaces(tmp.path());
        let map = composer_projects(&CursorPaths::new(tmp.path()));
        assert_eq!(map.len(), 2);
        assert_eq!(map["c-1"], "ai ledger");
        assert_eq!(map["c-2"], "randobot");
    }
}
