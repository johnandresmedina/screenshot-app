//! Capturing with macOS's built-in `screencapture` tool.

use crate::store::{self, CaptureMeta};
use std::fs;
use std::path::Path;
use std::process::Command;

/// Runs `screencapture -i`, which shows the native drag-to-select overlay
/// (press Space in it to pick a window instead). Returns the new file name,
/// or `None` if the user pressed Esc.
pub fn region(dir: &Path) -> Result<Option<String>, String> {
    fs::create_dir_all(dir).map_err(|e| format!("could not create {}: {e}", dir.display()))?;
    let name = store::new_file_name(dir, chrono::Local::now());
    // Write to a hidden temp name first so a scan never sees a half-written file.
    let tmp = store::temp_path_for(dir, &name);

    let status = Command::new("/usr/sbin/screencapture")
        .arg("-i")
        .arg(&tmp)
        .status()
        .map_err(|e| format!("could not run screencapture: {e}"))?;

    if !tmp.exists() {
        // Esc in the overlay exits without writing a file.
        return if status.success() { Ok(None) } else { Err(format!("screencapture failed ({status})")) };
    }

    let final_path = dir.join(&name);
    fs::rename(&tmp, &final_path).map_err(|e| format!("could not save capture: {e}"))?;
    let meta = CaptureMeta {
        version: Some(1),
        created_at: Some(chrono::Utc::now().timestamp_millis() as u64),
        source: Some("app".into()),
        kind: Some("region".into()),
    };
    // The image is already saved; a missing sidecar only loses metadata.
    if let Err(e) = store::write_meta(&final_path, &meta) {
        eprintln!("could not write metadata for {name}: {e}");
    }
    Ok(Some(name))
}

/// Puts the image on the clipboard as PNG data, so it pastes as an image.
/// The path goes in as an argument, never spliced into the script.
pub fn copy_to_clipboard(path: &Path) -> Result<(), String> {
    let out = Command::new("/usr/bin/osascript")
        .args([
            "-e", "on run argv",
            "-e", "set the clipboard to (read (POSIX file (item 1 of argv)) as «class PNGf»)",
            "-e", "end run",
        ])
        .arg(path)
        .output()
        .map_err(|e| format!("could not run osascript: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(format!("copy failed: {}", String::from_utf8_lossy(&out.stderr).trim()))
    }
}

/// `open` (default app, usually Preview) or `open -R` (reveal in Finder).
pub fn open(path: &Path, reveal: bool) -> Result<(), String> {
    let mut cmd = Command::new("/usr/bin/open");
    if reveal {
        cmd.arg("-R");
    }
    let status = cmd.arg(path).status().map_err(|e| format!("could not run open: {e}"))?;
    if status.success() { Ok(()) } else { Err(format!("open failed ({status})")) }
}

/// Moves the image and its sidecar to the Trash (recoverable, unlike rm).
pub fn trash(image: &Path) -> Result<(), String> {
    let mut paths = vec![image.to_path_buf()];
    let sidecar = store::sidecar_path(image);
    if sidecar.exists() {
        paths.push(sidecar);
    }
    #[allow(unused_mut)]
    let mut ctx = trash::TrashContext::default();
    #[cfg(target_os = "macos")]
    {
        use trash::macos::{DeleteMethod, TrashContextExtMacos};
        // The Finder method needs an Automation permission prompt; this one doesn't.
        ctx.set_delete_method(DeleteMethod::NsFileManager);
    }
    ctx.delete_all(&paths).map_err(|e| format!("could not move to Trash: {e}"))
}
