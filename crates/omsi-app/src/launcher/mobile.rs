//! The launcher on a phone or a tablet: fingers instead of a mouse, a keyboard that comes
//! up on the screen, a narrow rail of icons, pages that scroll as a whole, and a browser of
//! the device's storage where the desktop opens Finder or Explorer (a phone has no file
//! dialog that gives a program a path).
//!
//! `OMSI_MOBILE=1` gives the desktop launcher the same layout (with `OMSI_LAUNCHER_SIZE`
//! the size of a phone), so that it can be looked at without a phone.

use super::{Launcher, Page};
use glam::Vec2;
use std::path::{Path, PathBuf};
use winit::event::{Touch, TouchPhase};

/// Whether the launcher is laid out for fingers.
pub fn mobile() -> bool {
    crate::platform::MOBILE || omsi_cfg::flags::OMSI_MOBILE.is_set()
}

/// The fingers on the launcher (the interface takes the first as its pointer).
#[derive(Default)]
pub struct Fingers {
    /// Where each finger is (interface points), for the pinch over the bus.
    at: Vec<(u64, Vec2)>,
    pinch: Option<f32>,
}

/// What the storage browser chooses.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Purpose {
    /// The OMSI 2 folder (Setup).
    Root,
    /// A mod as a folder.
    ModFolder,
    /// A mod archive (.zip, .7z or .rar).
    ModZip,
}

/// The browser of the device's storage.
pub struct Browser {
    pub purpose: Purpose,
    pub dir: PathBuf,
    /// (name, is a folder, bytes)
    pub(crate) entries: Vec<(String, bool, u64)>,
    /// The folder is a complete OMSI 2 (Root only).
    pub(crate) is_root: bool,
    pub(crate) error: Option<String>,
}

/// The places a phone keeps files: the shared storage and any card or stick.
pub fn storage_roots() -> Vec<(String, PathBuf)> {
    let mut v = Vec::new();
    let shared = PathBuf::from("/storage/emulated/0");
    if shared.is_dir() {
        v.push(("Internal storage".to_string(), shared));
    }
    // the cards: /storage's own folders, where the system lets them be listed, and the
    // volumes mounted there - since Android 11 /storage itself is not readable even with
    // the access to all files, and the SD card did not show up at all (#1306)
    let mut cards: Vec<String> = std::fs::read_dir("/storage").map(|rd| rd.flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect()).unwrap_or_default();
    if let Ok(mounts) = std::fs::read_to_string("/proc/mounts") {
        cards.extend(storage_volumes(&mounts));
    }
    cards.sort();
    cards.dedup();
    for n in cards {
        if n == "emulated" || n == "self" {
            continue;
        }
        let p = PathBuf::from("/storage").join(&n);
        if p.is_dir() {
            v.push((format!("Card {n}"), p));
        }
    }
    if v.is_empty() {
        if let Some(h) = std::env::var_os("HOME") {
            v.push(("Home".to_string(), PathBuf::from(h)));
        }
    }
    v
}

/// The volumes `/proc/mounts` has under /storage (`/storage/1A2B-3C4D`): their names.
fn storage_volumes(mounts: &str) -> Vec<String> {
    mounts
        .lines()
        .filter_map(|l| l.split_whitespace().nth(1))
        .filter_map(|m| m.strip_prefix("/storage/"))
        .filter(|n| !n.is_empty() && !n.contains('/'))
        .map(|n| n.to_string())
        .collect()
}

#[cfg(test)]
mod storage_tests {
    #[test]
    fn an_sd_card_is_found_in_the_mounts() {
        let mounts = "/dev/fuse /storage/emulated fuse rw 0 0\n/dev/fuse /storage/1A2B-3C4D fuse rw 0 0\n/dev/block/vold/public:179,65 /mnt/media_rw/1A2B-3C4D vfat rw 0 0\n";
        assert_eq!(super::storage_volumes(mounts), vec!["emulated".to_string(), "1A2B-3C4D".to_string()]);
    }
}

impl Browser {
    pub fn new(purpose: Purpose, start: &str) -> Browser {
        let start = PathBuf::from(start.trim());
        let dir = if !start.as_os_str().is_empty() && start.is_dir() {
            start
        } else {
            storage_roots().first().map(|r| r.1.clone()).unwrap_or_else(|| PathBuf::from("/"))
        };
        let mut b = Browser { purpose, dir: PathBuf::new(), entries: Vec::new(), is_root: false, error: None };
        b.open(dir);
        b
    }

    pub(crate) fn open(&mut self, dir: PathBuf) {
        self.entries.clear();
        self.error = None;
        match std::fs::read_dir(&dir) {
            Ok(rd) => {
                for e in rd.flatten() {
                    let name = e.file_name().to_string_lossy().to_string();
                    if name.starts_with('.') {
                        continue;
                    }
                    let Ok(m) = e.metadata() else { continue };
                    let archive = [".zip", ".7z", ".rar"].iter().any(|ext| name.to_ascii_lowercase().ends_with(ext));
                    if m.is_dir() || (archive && self.purpose == Purpose::ModZip) {
                        self.entries.push((name, m.is_dir(), m.len()));
                    }
                }
                self.entries.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.to_lowercase().cmp(&b.0.to_lowercase())));
            }
            Err(e) => {
                self.error = Some(if e.kind() == std::io::ErrorKind::PermissionDenied {
                    "This folder cannot be read. Allow openOMSI access to all files (Android settings → Apps → openOMSI → Permissions → Files).".to_string()
                } else {
                    format!("{e}")
                });
            }
        }
        self.is_root = self.purpose == Purpose::Root && omsi_cfg::missing_original_essentials(&dir).is_empty();
        self.dir = dir;
    }

    pub(crate) fn title(&self) -> &'static str {
        match self.purpose {
            Purpose::Root => "Choose the OMSI 2 folder",
            Purpose::ModFolder => "Choose the mod folder",
            Purpose::ModZip => "Choose a mod archive (.zip, .7z, .rar)",
        }
    }
}

impl Launcher {

    /// Two fingers with the new interface (which takes one finger as the pointer): spread
    /// or pinched, they zoom the bus.
    pub(super) fn gui_pinch(&mut self, t: Touch, scale: f32) {
        let p = Vec2::new(t.location.x as f32, t.location.y as f32) / scale;
        match t.phase {
            TouchPhase::Started => {
                self.fingers.at.retain(|(id, _)| *id != t.id);
                self.fingers.at.push((t.id, p));
                if self.fingers.at.len() == 2 {
                    self.fingers.pinch = Some(self.fingers.at[0].1.distance(self.fingers.at[1].1).max(1.0));
                }
            }
            TouchPhase::Moved => {
                if let Some(f) = self.fingers.at.iter_mut().find(|(id, _)| *id == t.id) {
                    f.1 = p;
                }
                if let (Some(d0), 2) = (self.fingers.pinch, self.fingers.at.len()) {
                    let d = self.fingers.at[0].1.distance(self.fingers.at[1].1).max(1.0);
                    self.showroom.zoom_by((d0 / d).clamp(0.8, 1.25));
                    self.fingers.pinch = Some(d);
                }
            }
            TouchPhase::Ended | TouchPhase::Cancelled => {
                self.fingers.at.retain(|(id, _)| *id != t.id);
                if self.fingers.at.len() < 2 {
                    self.fingers.pinch = None;
                }
            }
        }
    }

    /// The narrow rail: an icon for each page.
    /// Open the storage browser (a phone's "Browse").
    pub fn browse(&mut self, purpose: Purpose, start: &str) {
        self.browser = Some(Browser::new(purpose, start));
    }

    pub(super) fn browser_chose(&mut self, purpose: Purpose, p: &Path) {
        let s = p.to_string_lossy().to_string();
        match purpose {
            Purpose::Root => {
                self.pages.setup_root = Some(s);
                self.state.set_status("Folder chosen: press Save.", false);
            }
            Purpose::ModFolder | Purpose::ModZip => {
                self.page = Page::Mods;
                self.state.install(s);
            }
        }
    }
}
