//! The shared capture folder.
//!
//! The folder (`~/Screenshots/MyApp` by default) is the only source of truth.
//! There is no shared index file: history is built by scanning the folder for
//! images, and each image may have a sidecar `<stem>.json` with metadata.
//! Every writer (this app now, the MCP server later) only ever creates its own
//! files, so two processes can never overwrite each other's entries.
//! See docs/STORAGE.md for the full contract.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

pub const IMAGE_EXTENSIONS: [&str; 3] = ["png", "jpg", "jpeg"];

/// What a sidecar `<stem>.json` holds. Every field is optional so a sidecar
/// written by an older or newer writer still reads.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CaptureMeta {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<u32>,
    /// Milliseconds since the Unix epoch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<u64>,
    /// Who took it: "app" or "mcp".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// How it was taken: "region", "screen" or "window".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
}

/// One entry in the history, as sent to the frontend.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Capture {
    /// The image's file name. Commands take this, never a full path.
    pub id: String,
    pub path: String,
    pub created_at: u64,
    pub source: String,
    pub kind: String,
    pub size_bytes: u64,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

pub fn default_dir() -> PathBuf {
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    home.join("Screenshots").join("MyApp")
}

fn is_image(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| IMAGE_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

pub fn sidecar_path(image: &Path) -> PathBuf {
    image.with_extension("json")
}

/// Width and height from a PNG's IHDR chunk, without decoding the image.
pub fn png_dimensions(path: &Path) -> Option<(u32, u32)> {
    use std::io::Read;
    let mut header = [0u8; 24];
    fs::File::open(path).ok()?.read_exact(&mut header).ok()?;
    if &header[0..8] != b"\x89PNG\r\n\x1a\n" || &header[12..16] != b"IHDR" {
        return None;
    }
    let w = u32::from_be_bytes(header[16..20].try_into().ok()?);
    let h = u32::from_be_bytes(header[20..24].try_into().ok()?);
    Some((w, h))
}

/// All captures in `dir`, newest first. A missing folder is an empty history.
pub fn list(dir: &Path) -> io::Result<Vec<Capture>> {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(vec![]),
        Err(e) => return Err(e),
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        // Hidden files are writes in progress (see `temp_path_for`).
        if name.starts_with('.') || !is_image(&path) {
            continue;
        }
        let Ok(md) = entry.metadata() else { continue };
        if !md.is_file() {
            continue;
        }
        let meta: CaptureMeta = fs::read(sidecar_path(&path))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        let fs_time = md
            .created()
            .or_else(|_| md.modified())
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let dims = png_dimensions(&path);
        out.push(Capture {
            id: name,
            path: path.to_string_lossy().into_owned(),
            created_at: meta.created_at.unwrap_or(fs_time),
            source: meta.source.unwrap_or_else(|| "unknown".into()),
            kind: meta.kind.unwrap_or_else(|| "unknown".into()),
            size_bytes: md.len(),
            width: dims.map(|d| d.0),
            height: dims.map(|d| d.1),
        });
    }
    out.sort_by(|a, b| b.created_at.cmp(&a.created_at).then_with(|| b.id.cmp(&a.id)));
    Ok(out)
}

/// Turns an id from the frontend into a path inside `dir`, refusing anything
/// that could point outside it.
pub fn resolve(dir: &Path, id: &str) -> Result<PathBuf, String> {
    let bad = id.is_empty()
        || id.starts_with('.')
        || id.contains('/')
        || id.contains('\\')
        || id.contains('\0');
    let path = dir.join(id);
    if bad || !is_image(&path) {
        return Err(format!("not a capture id: {id:?}"));
    }
    if !path.is_file() {
        return Err(format!("capture not found: {id}"));
    }
    Ok(path)
}

/// A fresh, unused file name for a new capture, like `2026-11-24_223105_123.png`.
pub fn new_file_name(dir: &Path, now: chrono::DateTime<chrono::Local>) -> String {
    let stem = now.format("%Y-%m-%d_%H%M%S_%3f").to_string();
    let mut name = format!("{stem}.png");
    let mut n = 2;
    while dir.join(&name).exists() {
        name = format!("{stem}-{n}.png");
        n += 1;
    }
    name
}

/// Where to write an image before it is complete. Hidden, so `list` skips it.
pub fn temp_path_for(dir: &Path, file_name: &str) -> PathBuf {
    dir.join(format!(".partial-{file_name}"))
}

/// Writes the sidecar atomically (temp file, then rename).
pub fn write_meta(image: &Path, meta: &CaptureMeta) -> io::Result<()> {
    let target = sidecar_path(image);
    let name = target.file_name().unwrap_or_default().to_string_lossy();
    let tmp = target.with_file_name(format!(".partial-{name}"));
    fs::write(&tmp, serde_json::to_vec_pretty(meta)?)?;
    fs::rename(&tmp, &target)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(w: u32, h: u32) -> Vec<u8> {
        let mut v = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR".to_vec();
        v.extend_from_slice(&w.to_be_bytes());
        v.extend_from_slice(&h.to_be_bytes());
        v.extend_from_slice(&[8, 6, 0, 0, 0]);
        v
    }

    #[test]
    fn missing_folder_is_empty_history() {
        let dir = tempfile::tempdir().unwrap();
        assert!(list(&dir.path().join("nope")).unwrap().is_empty());
    }

    #[test]
    fn lists_images_newest_first_with_sidecar_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        fs::write(d.join("a.png"), png(100, 50)).unwrap();
        write_meta(&d.join("a.png"), &CaptureMeta {
            version: Some(1), created_at: Some(1_000), source: Some("app".into()), kind: Some("region".into()),
        }).unwrap();
        fs::write(d.join("b.png"), png(10, 20)).unwrap();
        write_meta(&d.join("b.png"), &CaptureMeta { created_at: Some(2_000), source: Some("mcp".into()), ..Default::default() }).unwrap();
        // Skipped: in-progress writes, non-images, orphan sidecars.
        fs::write(d.join(".partial-c.png"), png(1, 1)).unwrap();
        fs::write(d.join("notes.txt"), "x").unwrap();
        fs::write(d.join("orphan.json"), "{}").unwrap();

        let got = list(d).unwrap();
        let ids: Vec<_> = got.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, ["b.png", "a.png"]);
        assert_eq!(got[1].source, "app");
        assert_eq!(got[1].kind, "region");
        assert_eq!((got[1].width, got[1].height), (Some(100), Some(50)));
        assert_eq!(got[0].source, "mcp");
        assert_eq!(got[0].kind, "unknown");
    }

    #[test]
    fn image_without_sidecar_or_with_bad_sidecar_still_lists() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("x.PNG"), png(3, 4)).unwrap();
        fs::write(dir.path().join("x.json"), "not json").unwrap();
        let got = list(dir.path()).unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].source, "unknown");
        assert!(got[0].created_at > 0);
    }

    #[test]
    fn resolve_rejects_paths_outside_the_folder() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("ok.png"), png(1, 1)).unwrap();
        assert!(resolve(dir.path(), "ok.png").is_ok());
        for bad in ["", "../ok.png", "/etc/passwd", "a/b.png", ".partial-ok.png", "ok.json", "missing.png"] {
            assert!(resolve(dir.path(), bad).is_err(), "{bad} should be rejected");
        }
    }

    #[test]
    fn new_file_names_do_not_collide() {
        let dir = tempfile::tempdir().unwrap();
        let now = chrono::Local::now();
        let first = new_file_name(dir.path(), now);
        fs::write(dir.path().join(&first), b"").unwrap();
        let second = new_file_name(dir.path(), now);
        assert_ne!(first, second);
        assert!(second.ends_with("-2.png"));
    }

    #[test]
    fn png_dimensions_rejects_non_png() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("f.png");
        fs::write(&p, b"GIF89a................").unwrap();
        assert_eq!(png_dimensions(&p), None);
    }
}
