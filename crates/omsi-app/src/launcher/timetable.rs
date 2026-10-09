//! The Timetable page: a map's lines, their tours and the trips each tour runs - the part of
//! OMSI 2's timetable editor a driver uses to change when buses go. A line is saved as its
//! `.ttl`; a map of the original installation gets its `TTData` copied into the content
//! folder first (the game reads that copy before the original's, which stays untouched).
//! Trips (`.ttp`) and their tracks (`.ttr`) are the map maker's and are only chosen here.

use super::Launcher;
use omsi_launcher_lib as core;
use omsi_timetable::{Line, TimetableData, Tour};
use std::path::{Path, PathBuf};

#[derive(Default)]
pub struct TimetableView {
    pub map: usize,
    /// The map the data was read for (its `global.cfg`).
    pub(crate) loaded: Option<String>,
    pub(crate) data: Option<TimetableData>,
    pub(crate) map_dir: PathBuf,
    pub(crate) line: usize,
    pub(crate) tour: usize,
    /// The departures being typed, one per trip of the shown tour.
    pub(crate) times: Vec<String>,
    pub(crate) times_for: Option<(usize, usize)>,
    /// Minutes a copied tour runs after the one it copies (and the repeat's interval).
    pub(crate) offset: String,
    /// The repeat's last departure ("h:mm").
    pub(crate) until: String,
    /// The name typed for a new line.
    pub(crate) new_line: String,
    /// The lines changed and not saved yet (by name): kept while the player moves between
    /// lines - the page used to drop a line's changes when another was clicked, and a day's
    /// timetable took a save after every line.
    pub(crate) dirty: std::collections::BTreeSet<String>,
    /// The reset button was pressed once: the next press puts the map's own timetable back.
    pub(crate) reset_armed: bool,
}

/// Marks a `TTData` folder the launcher copied into the content folder (see `save_target`):
/// the reset deletes such a copy, and only such a one.
const COPY_MARK: &str = ".openomsi-ttdata-copy";

/// "h:mm" (or "h:mm:ss") from minutes after midnight.
pub fn fmt_time(min: f32) -> String {
    let s = (min as f64 * 60.0).round() as i64;
    if s % 60 == 0 {
        format!("{}:{:02}", s / 3600, s / 60 % 60)
    } else {
        format!("{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
    }
}

/// Minutes after midnight from "h:mm", "h:mm:ss" or "h.mm" (after midnight as 24:10 and on).
pub fn parse_time(t: &str) -> Option<f32> {
    let parts: Vec<&str> = t.trim().split([':', '.']).collect();
    if parts.len() < 2 || parts.len() > 3 {
        return None;
    }
    let h: u32 = parts[0].trim().parse().ok()?;
    let m: u32 = parts[1].trim().parse().ok()?;
    let s: u32 = parts.get(2).map(|x| x.trim().parse().ok()).unwrap_or(Some(0))?;
    (h < 48 && m < 60 && s < 60).then(|| h as f32 * 60.0 + m as f32 + s as f32 / 60.0)
}

/// Where a map's line is written: in place when the map lies unpacked in the content folder
/// (its file backed up as `<file>.orig` the first time, for the reset), else in the content
/// folder's copy of its `TTData` (made whole first - the game reads the one folder, not a mix
/// of both). A map inside a mod archive (`.zip`) counts as not in the content folder: its
/// file was "saved in place" into the archive's path, which is no folder, and never saved.
fn save_target(line: &Line, map_folder: &str, original_ttdata: &Path) -> Result<PathBuf, String> {
    let content = core::content_dir().ok_or("no content folder")?;
    let file = line.path.file_name().map(|f| f.to_owned()).unwrap_or_else(|| format!("{}.ttl", line.name).into());
    if line.path.starts_with(&content) && omsi_cfg::vfs::archive_of(&line.path).is_none() {
        let orig = PathBuf::from(format!("{}.orig", line.path.display()));
        let copied = line.path.parent().is_some_and(|d| d.join(COPY_MARK).is_file());
        if !copied && line.path.is_file() && !orig.exists() {
            std::fs::copy(&line.path, &orig).map_err(|e| e.to_string())?;
        }
        return Ok(line.path.clone());
    }
    let dir = content.join("maps").join(map_folder).join("TTData");
    if !dir.is_dir() {
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        for (name, is_dir) in omsi_cfg::vfs::list_dir(original_ttdata).ok_or_else(|| format!("{} cannot be read", original_ttdata.display()))? {
            if !is_dir {
                let bytes = omsi_cfg::vfs::read(&original_ttdata.join(&name)).map_err(|e| e.to_string())?;
                std::fs::write(dir.join(&name), bytes).map_err(|e| e.to_string())?;
            }
        }
        std::fs::write(dir.join(COPY_MARK), b"TTData copied by the openOMSI launcher's timetable editor; its reset deletes this folder\n").map_err(|e| e.to_string())?;
    }
    Ok(dir.join(file))
}

/// The map's own timetable back: the launcher's copy of its `TTData` deleted, or (a map
/// unpacked in the content folder) every line saved over put back from its `.orig`.
/// Returns what was done.
pub(crate) fn reset_timetable(map_dir: &Path, map_folder: &str) -> Result<String, String> {
    let content = core::content_dir().ok_or("no content folder")?;
    let copy = content.join("maps").join(map_folder).join("TTData");
    if copy.join(COPY_MARK).is_file() {
        std::fs::remove_dir_all(&copy).map_err(|e| e.to_string())?;
        return Ok(format!("The timetable of {map_folder} is the map's own again (the edited copy was removed)"));
    }
    let own = map_dir.join("TTData");
    let mut restored = 0;
    if own.is_dir() {
        for e in std::fs::read_dir(&own).map_err(|e| e.to_string())?.flatten() {
            let p = e.path();
            if let Some(orig) = p.to_str().and_then(|s| s.strip_suffix(".orig")) {
                std::fs::rename(&p, orig).map_err(|e| e.to_string())?;
                restored += 1;
            }
        }
    }
    if restored > 0 {
        Ok(format!("{restored} line(s) of {map_folder} put back as they were"))
    } else {
        Err(format!("The timetable of {map_folder} has not been changed here"))
    }
}

/// Save every changed line of the map (see `save_target`); how many were saved, and the
/// first error.
pub(crate) fn save_all(tv: &mut TimetableView) -> (usize, Option<String>) {
    let Some(data) = tv.data.as_mut() else { return (0, None) };
    let folder = tv.map_dir.file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default();
    let original = omsi_cfg::resolve_path(&tv.map_dir, "TTData");
    let mut saved = 0;
    let mut err = None;
    for line in data.lines.iter_mut().filter(|l| tv.dirty.contains(&l.name)) {
        for t in &mut line.tours {
            t.trips.sort_by(|a, b| a.departure.total_cmp(&b.departure));
        }
        match save_target(line, &folder, &original).and_then(|p| line.save(&p).map(|_| p).map_err(|e| e.to_string())) {
            Ok(p) => {
                line.path = p;
                saved += 1;
            }
            Err(e) => {
                err.get_or_insert(format!("line {}: {e}", line.name));
            }
        }
    }
    if err.is_none() {
        tv.dirty.clear();
    }
    if saved > 0 {
        omsi_cfg::content_changed();
    }
    (saved, err)
}

/// Copies of `base` every `every` minutes after it, as long as the copy's first departure is
/// not after `until` (minutes of the day); numbered on from the highest. Returns how many.
pub(crate) fn repeat_tour(tours: &mut Vec<Tour>, base: &Tour, every: f32, until: f32) -> usize {
    let first = base.trips.first().map(|t| t.departure).unwrap_or(0.0);
    let mut made = 0;
    let mut k = 1.0;
    while first + every * k <= until + 1e-3 && made < 500 {
        let mut t = base.clone();
        for x in &mut t.trips {
            x.departure += every * k;
        }
        t.number = next_number(tours);
        tours.push(t);
        made += 1;
        k += 1.0;
    }
    made
}

/// A new tour's number: one past the highest that is a number.
pub(crate) fn next_number(tours: &[Tour]) -> String {
    (tours.iter().filter_map(|t| t.number.trim().parse::<i64>().ok()).max().unwrap_or(0) + 1).to_string()
}

/// The chosen map's timetable read (once per map; first shown: the map chosen on the Drive
/// page, not the first of the list).
pub(crate) fn ensure_loaded(l: &mut Launcher) {
    let maps: Vec<String> = l.state.maps.iter().map(|m| m.file.clone()).collect();
    if maps.is_empty() {
        return;
    }
    let tv = &mut l.pages.tt;
    if tv.loaded.is_none() {
        if let Some(k) = maps.iter().position(|m| *m == l.state.choice.map) {
            tv.map = k;
        }
    }
    tv.map = tv.map.min(maps.len() - 1);
    if tv.loaded.as_deref() != Some(maps[tv.map].as_str()) {
        let file = &maps[tv.map];
        let root = PathBuf::from(&l.state.config.root);
        let global = core::content_dir().map(|c| c.join(file)).filter(|p| p.is_file()).unwrap_or_else(|| omsi_cfg::resolve_path(&root, file));
        tv.map_dir = global.parent().map(Path::to_path_buf).unwrap_or_default();
        let mut d = TimetableData::load(&tv.map_dir);
        d.lines.sort_by_key(|x| x.name.to_lowercase());
        tv.data = Some(d);
        tv.loaded = Some(file.clone());
        tv.line = 0;
        tv.tour = 0;
        tv.times_for = None;
        tv.dirty.clear();
    }
    if tv.offset.is_empty() {
        tv.offset = "20".into();
    }
    if tv.until.is_empty() {
        tv.until = "22:00".into();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn times() {
        assert_eq!(parse_time("4:07"), Some(247.0));
        assert_eq!(parse_time("24:10"), Some(1450.0));
        assert_eq!(parse_time("4:7:30"), Some(247.5));
        assert_eq!(parse_time("4"), None);
        assert_eq!(parse_time("4:61"), None);
        assert_eq!(fmt_time(247.0), "4:07");
        assert_eq!(fmt_time(247.5), "4:07:30");
    }

    #[test]
    fn numbers() {
        let t = |n: &str| Tour { number: n.into(), ..Default::default() };
        assert_eq!(next_number(&[t("1"), t("7"), t("x")]), "8");
        assert_eq!(next_number(&[]), "1");
    }
}

#[cfg(test)]
mod repeat_tests {
    use super::*;
    use omsi_timetable::TourTrip;

    #[test]
    fn a_tour_every_twenty_minutes_until_eight() {
        let base = Tour { number: "1".into(), ai_group: "Busses".into(), extra: String::new(), trips: vec![TourTrip { trip: "a".into(), profile: 0, departure: 6.0 * 60.0 }, TourTrip { trip: "b".into(), profile: 0, departure: 6.5 * 60.0 }] };
        let mut tours = vec![base.clone()];
        let made = repeat_tour(&mut tours, &base, 20.0, 8.0 * 60.0);
        // 6:20, 6:40 … 8:00
        assert_eq!(made, 6);
        assert_eq!(tours.last().unwrap().trips[0].departure, 8.0 * 60.0);
        assert_eq!(tours.last().unwrap().trips[1].departure, 8.5 * 60.0);
        assert_eq!(tours.last().unwrap().number, "7");
    }
}
