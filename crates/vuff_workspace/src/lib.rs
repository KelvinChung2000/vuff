//! File discovery and config resolution.
//!
//! Milestone 1 stub: surface only. Milestone 8 wires `ignore`-based path
//! walking and full config resolution.

use std::path::{Path, PathBuf};

/// Walk up from `start` looking for `vuff.toml`. Returns the first hit
/// or `None` if we reach the filesystem root.
#[must_use]
pub fn find_config(start: &Path) -> Option<PathBuf> {
    let mut cur = if start.is_file() {
        start.parent()?.to_path_buf()
    } else {
        start.to_path_buf()
    };
    loop {
        let candidate = cur.join("vuff.toml");
        if candidate.is_file() {
            return Some(candidate);
        }
        if !cur.pop() {
            return None;
        }
    }
}
