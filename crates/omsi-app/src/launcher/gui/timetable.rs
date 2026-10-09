//! The Timetable page: a map's lines, their tours and when each trip leaves (see
//! `launcher::timetable` for where it is saved and how the reset works).

use super::kit::{self, len, lp, tr};
use super::theme::DANGER;
use super::Msg as Top;
use crate::launcher::timetable::{ensure_loaded, fmt_time, next_number, parse_time, repeat_tour, reset_timetable, save_all, TimetableView};
use crate::launcher::{Launcher, Page};
use egui_retained::widgets::{Button, Select, Text, TextInput};
use egui_retained::{NodeId, ScrollAxes, Ui, Visual, taffy};
use omsi_timetable::{Line, Tour, TourTrip};
use std::hash::{Hash, Hasher};

#[derive(Clone, Debug)]
pub(in crate::launcher) enum Msg {
    Map(usize),
    Line(usize),
    NewLineName(String),
    AddLine,
    Reset,
    Tour(usize),
    NewTour,
    Offset(String),
    CopyTour,
    Until(String),
    Repeat,
    DeleteTour,
    Departure(usize, String),
    Trip(usize, usize),
    Profile(usize, usize),
    RemoveTrip(usize),
    AddTrip,
    Shift(f32),
    Save,
}

fn m(x: Msg) -> Top {
    Top::Timetable(x)
}

pub(in crate::launcher) struct TimetablePage {
    pub root: NodeId,
    empty: NodeId,
    cols: NodeId,
    map_select: NodeId,
    lines: NodeId,
    lines_key: u64,
    new_line: NodeId,
    reset: NodeId,
    tours_title: NodeId,
    tours: NodeId,
    tours_key: u64,
    trips_title: NodeId,
    trips: NodeId,
    trips_key: u64,
    departures: Vec<NodeId>,
    save: NodeId,
    /// Bumped by what changes the departures from outside their fields (a shift, a reread).
    gen: u64,
}

/// The trips a line can run (those naming it, else those it runs, else all), each with its
/// profiles and terminus.
fn trips_of(tv: &TimetableView) -> Vec<(String, Vec<String>, String)> {
    let Some(data) = tv.data.as_ref() else { return Vec::new() };
    let Some(line) = data.lines.get(tv.line) else { return Vec::new() };
    let mut trips: Vec<String> = data.trips.iter().filter(|t| t.line.trim().eq_ignore_ascii_case(line.name.trim())).map(|t| t.name.clone()).collect();
    for t in line.tours.iter().flat_map(|t| t.trips.iter()) {
        if !trips.iter().any(|x| x.eq_ignore_ascii_case(&t.trip)) {
            trips.push(t.trip.clone());
        }
    }
    if trips.is_empty() {
        trips = data.trips.iter().map(|t| t.name.clone()).collect();
    }
    trips.sort_by_key(|t| t.to_lowercase());
    trips
        .into_iter()
        .map(|t| {
            let profs = data.trip(&t).map(|x| x.profiles.iter().map(|p| p.name.clone()).collect::<Vec<_>>()).filter(|p| !p.is_empty()).unwrap_or_else(|| vec!["0".into()]);
            let term = data.trip(&t).map(|x| x.terminus.clone()).unwrap_or_default();
            (t, profs, term)
        })
        .collect()
}

fn col(ui: &mut Ui, parent: NodeId, width: Option<f32>) -> NodeId {
    let c = kit::card(ui, parent);
    ui.style(c, |s| {
        match width {
            Some(w) => {
                s.size.width = len(w);
                s.flex_shrink = 0.0;
            }
            None => {
                s.flex_grow = 1.0;
                s.flex_basis = len(0.0);
            }
        }
        s.min_size = taffy::Size { width: len(0.0), height: len(0.0) };
    });
    c
}

fn list(ui: &mut Ui, parent: NodeId) -> NodeId {
    let l = ui.column(parent);
    kit::grow(ui, l);
    ui.set_scroll(l, ScrollAxes { x: false, y: true });
    ui.style(l, |s| {
        s.min_size.height = len(0.0);
        s.gap = taffy::Size { width: lp(3.0), height: lp(3.0) };
    });
    l
}

impl TimetablePage {
    pub fn build(ui: &mut Ui, host: NodeId) -> TimetablePage {
        let root = ui.column(host);
        ui.add_class(root, "content");
        kit::grow(ui, root);
        kit::page_title(ui, root, "Timetable", "A map's lines: their tours and when each trip leaves. Saved as the line's .ttl (in the content folder; OMSI 2's own files stay as they are).");
        let empty = kit::para(ui, root, &tr("No maps found."), "dim");
        let cols = ui.row(root);
        kit::grow(ui, cols);
        ui.style(cols, |s| {
            s.gap = taffy::Size { width: lp(14.0), height: lp(14.0) };
            s.align_items = Some(taffy::AlignItems::Stretch);
            s.min_size.height = len(0.0);
        });
        // the map and its lines
        let a = col(ui, cols, Some(290.0));
        kit::text(ui, a, &tr("Lines"), "heading");
        let map_select = ui.add(a, Select::new(Vec::new(), None).on_change(|i| m(Msg::Map(i))));
        let lines = list(ui, a);
        let nr = ui.row(a);
        kit::gap(ui, nr, 8.0);
        let new_line = ui.add(nr, TextInput::new("").hint(tr("New line (name)")).on_change(|s| m(Msg::NewLineName(s))));
        kit::grow(ui, new_line);
        let b = ui.add(nr, Button::new(tr("Add")).icon("add"));
        ui.on_click(b, m(Msg::AddLine));
        let reset = ui.add(a, Button::new(tr("Reset timetable")).icon("restart_alt"));
        ui.on_click(reset, m(Msg::Reset));
        // its tours
        let b = col(ui, cols, Some(270.0));
        let tours_title = kit::text(ui, b, "", "heading");
        let tours = list(ui, b);
        let nt = ui.add(b, Button::new(tr("New tour")).icon("add"));
        ui.on_click(nt, m(Msg::NewTour));
        let r = ui.row(b);
        kit::gap(ui, r, 8.0);
        let offset = ui.add(r, TextInput::new("20").hint("min").on_change(|s| m(Msg::Offset(s))));
        kit::grow(ui, offset);
        let cb = ui.add(r, Button::new(tr("Copy +min")).icon("content_copy"));
        ui.on_click(cb, m(Msg::CopyTour));
        let r = ui.row(b);
        kit::gap(ui, r, 8.0);
        let until = ui.add(r, TextInput::new("22:00").hint(tr("until h:mm")).on_change(|s| m(Msg::Until(s))));
        kit::grow(ui, until);
        let rb = ui.add(r, Button::new(tr("Repeat")).icon("autorenew"));
        ui.on_click(rb, m(Msg::Repeat));
        let db = ui.add(b, Button::new(tr("Delete tour")).icon("delete").class("danger"));
        ui.on_click(db, m(Msg::DeleteTour));
        // the tour's trips
        let c = col(ui, cols, None);
        let trips_title = kit::text(ui, c, "", "heading");
        let head = ui.row(c);
        kit::gap(ui, head, 8.0);
        for (t, w) in [("Departs", Some(76.0)), ("Trip", None), ("Profile", Some(110.0)), ("To", Some(140.0)), ("", Some(34.0))] {
            let n = kit::text(ui, head, &tr(t), "faint");
            ui.style(n, |s| match w {
                Some(w) => s.size.width = len(w),
                None => {
                    s.flex_grow = 1.0;
                    s.flex_basis = len(0.0);
                }
            });
        }
        let trips = list(ui, c);
        let f = ui.row(c);
        kit::gap(ui, f, 8.0);
        let at = ui.add(f, Button::new(tr("Add trip")).icon("add"));
        ui.on_click(at, m(Msg::AddTrip));
        for (t, d) in [("−1 min", -1.0f32), ("+1 min", 1.0)] {
            let b = ui.add(f, Button::new(tr(t)));
            ui.on_click(b, m(Msg::Shift(d)));
        }
        ui.spacer(f);
        let save = ui.add(f, Button::new(tr("Saved")).icon("save"));
        ui.on_click(save, m(Msg::Save));
        TimetablePage { root, empty, cols, map_select, lines, lines_key: 1, new_line, reset, tours_title, tours, tours_key: 1, trips_title, trips, trips_key: 1, departures: Vec::new(), save, gen: 0 }
    }

    pub fn sync(&mut self, ui: &mut Ui, l: &Launcher) {
        let has_maps = !l.state.maps.is_empty();
        ui.set_visible(self.empty, !has_maps);
        ui.set_visible(self.cols, has_maps);
        let tv = &l.pages.tt;
        let Some(data) = tv.data.as_ref() else { return };
        let names: Vec<String> = l.state.maps.iter().map(|x| x.friendly.clone()).collect();
        ui.set_select(self.map_select, Some(names), Some(tv.map));
        ui.set_text(self.reset, &tr(if tv.reset_armed { "Press again to reset" } else { "Reset timetable" }));
        ui.set_class(self.reset, "danger", tv.reset_armed);
        // the lines
        let key = {
            let mut h = std::collections::hash_map::DefaultHasher::new();
            (&tv.loaded, tv.line, &tv.dirty).hash(&mut h);
            for x in &data.lines {
                (&x.name, x.tours.len()).hash(&mut h);
            }
            h.finish()
        };
        if key != self.lines_key {
            self.lines_key = key;
            ui.clear(self.lines);
            if data.lines.is_empty() {
                kit::para(ui, self.lines, &tr("This map has no timetable (no TTData lines)."), "dim");
            }
            for (i, x) in data.lines.iter().enumerate() {
                let r = ui.row(self.lines);
                ui.add_class(r, "list-row");
                ui.set_selected(r, i == tv.line);
                ui.on_click(r, m(Msg::Line(i)));
                let t = kit::text(ui, r, &format!("{}{}", x.name, if tv.dirty.contains(&x.name) { " •" } else { "" }), "strong");
                kit::grow(ui, t);
                kit::text(ui, r, &format!("{} {}", x.tours.len(), tr("tours")), "dim");
            }
        }
        let Some(line) = data.lines.get(tv.line) else {
            ui.set_text(self.tours_title, "");
            ui.clear(self.tours);
            ui.clear(self.trips);
            self.tours_key = 1;
            self.trips_key = 1;
            return;
        };
        ui.set_text(self.tours_title, &format!("{} {}", tr("Line"), line.name));
        // its tours
        let key = {
            let mut h = std::collections::hash_map::DefaultHasher::new();
            (&tv.loaded, tv.line, tv.tour).hash(&mut h);
            for t in &line.tours {
                (&t.number, t.trips.len(), t.trips.first().map(|x| x.departure.to_bits())).hash(&mut h);
            }
            h.finish()
        };
        if key != self.tours_key {
            self.tours_key = key;
            ui.clear(self.tours);
            for (i, t) in line.tours.iter().enumerate() {
                let r = ui.row(self.tours);
                ui.add_class(r, "list-row");
                ui.set_selected(r, i == tv.tour);
                ui.on_click(r, m(Msg::Tour(i)));
                let n = kit::text(ui, r, &format!("{} {}", tr("Tour"), t.number), "strong");
                kit::grow(ui, n);
                kit::text(ui, r, &format!("{} · {}", t.trips.first().map(|x| fmt_time(x.departure)).unwrap_or_default(), t.trips.len()), "dim");
            }
        }
        // the trips
        let n = tv.dirty.len();
        ui.set_text(self.save, &match n {
            0 => tr("Saved"),
            1 => tr("Save"),
            n => format!("{} ({n})", tr("Save all")),
        });
        ui.set_class(self.save, "primary", n > 0);
        let Some(tour) = line.tours.get(tv.tour) else {
            ui.set_text(self.trips_title, "");
            ui.clear(self.trips);
            self.trips_key = 1;
            return;
        };
        ui.set_text(self.trips_title, &format!("{} {} - {}", tr("Tour"), tour.number, tour.ai_group));
        let options = trips_of(tv);
        let key = {
            let mut h = std::collections::hash_map::DefaultHasher::new();
            (&tv.loaded, tv.line, tv.tour, self.gen).hash(&mut h);
            for t in &tour.trips {
                (&t.trip, t.profile).hash(&mut h);
            }
            h.finish()
        };
        if key != self.trips_key {
            self.trips_key = key;
            ui.clear(self.trips);
            self.departures.clear();
            let names: Vec<String> = options.iter().map(|o| o.0.clone()).collect();
            for (i, t) in tour.trips.iter().enumerate() {
                let r = ui.row(self.trips);
                kit::gap(ui, r, 8.0);
                ui.style(r, |s| s.flex_shrink = 0.0);
                let d = ui.add(r, TextInput::new(tv.times.get(i).cloned().unwrap_or_else(|| fmt_time(t.departure))).hint("h:mm").on_change(move |s| m(Msg::Departure(i, s))));
                ui.style(d, |s| s.size.width = len(76.0));
                self.departures.push(d);
                let ti = names.iter().position(|n| n.eq_ignore_ascii_case(&t.trip)).unwrap_or(0);
                let s = ui.add(r, Select::new(names.clone(), Some(ti)).on_change(move |k| m(Msg::Trip(i, k))));
                ui.style(s, |s| {
                    s.flex_grow = 1.0;
                    s.flex_basis = len(0.0);
                    s.min_size.width = len(0.0);
                });
                let (_, profs, dest) = options.iter().find(|o| o.0.eq_ignore_ascii_case(&t.trip)).cloned().unwrap_or_default();
                let profs = if profs.is_empty() { vec![t.profile.to_string()] } else { profs };
                let pi = (t.profile.max(0) as usize).min(profs.len() - 1);
                let p = ui.add(r, Select::new(profs, Some(pi)).on_change(move |k| m(Msg::Profile(i, k))));
                ui.style(p, |s| s.size.width = len(110.0));
                let to = ui.add(r, Text::new(dest));
                ui.style(to, |s| s.size.width = len(140.0));
                let x = ui.add(r, Button::new("").icon("close").class("ghost"));
                ui.set_tooltip(x, Some(tr("Remove this trip")));
                ui.on_click(x, m(Msg::RemoveTrip(i)));
            }
        }
        // a departure that is not a time: its field marked
        for (i, d) in self.departures.iter().enumerate() {
            let bad = tv.times.get(i).is_some_and(|t| parse_time(t).is_none());
            ui.visual(*d, if bad { Visual::new().border(egui_retained::epaint::Stroke::new(1.5, DANGER)) } else { Visual::new() });
        }
    }
}

impl Launcher {
    /// The Timetable page's data read when it is shown (and the map changed).
    pub(super) fn gui_timetable_tick(&mut self) {
        if self.page != Page::Timetable {
            return;
        }
        let before = self.pages.tt.loaded.clone();
        ensure_loaded(self);
        let tv = &mut self.pages.tt;
        if let Some(tour) = tv.data.as_ref().and_then(|d| d.lines.get(tv.line)).and_then(|x| x.tours.get(tv.tour)) {
            if tv.times_for != Some((tv.line, tv.tour)) || tv.times.len() != tour.trips.len() {
                tv.times = tour.trips.iter().map(|t| fmt_time(t.departure)).collect();
                tv.times_for = Some((tv.line, tv.tour));
                if let Some(p) = self.gui.as_mut().and_then(|g| g.timetable.as_mut()) {
                    p.gen += 1;
                }
            }
        }
        if before != self.pages.tt.loaded {
            if let Some(p) = self.gui.as_mut().and_then(|g| g.timetable.as_mut()) {
                p.gen += 1;
            }
        }
    }
}

pub(in crate::launcher) fn handle(l: &mut Launcher, msg: Msg) {
    let mut status: Option<(String, bool)> = None;
    let tv = &mut l.pages.tt;
    let line_name = tv.data.as_ref().and_then(|d| d.lines.get(tv.line)).map(|x| x.name.clone()).unwrap_or_default();
    match msg {
        Msg::Map(k) => {
            if k != tv.map {
                // (another map: this one's changes are saved first, not lost)
                if !tv.dirty.is_empty() {
                    let (saved, err) = save_all(tv);
                    status = Some(match err {
                        Some(e) => (format!("Not saved: {e}"), true),
                        None => (format!("{saved} line(s) saved"), false),
                    });
                }
                tv.map = k;
                tv.loaded = None;
            }
        }
        Msg::Line(i) => {
            // (the line left keeps its changes: they are saved with the others)
            tv.line = i;
            tv.tour = 0;
            tv.times_for = None;
        }
        Msg::NewLineName(s) => tv.new_line = s,
        Msg::AddLine => {
            let name = tv.new_line.trim().to_string();
            let Some(data) = tv.data.as_mut() else { return };
            if name.is_empty() || name.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|']) {
                status = Some(("Type the new line's name (it becomes the file name, so no / \\ : * ? \" < > |)".into(), true));
            } else if data.lines.iter().any(|x| x.name.eq_ignore_ascii_case(&name)) {
                status = Some((format!("Line {name} is there already"), true));
            } else {
                let path = omsi_cfg::resolve_path(&tv.map_dir, "TTData").join(format!("{name}.ttl"));
                data.lines.push(Line { path, name: name.clone(), user_allowed: true, priority: 0, tours: Vec::new() });
                data.lines.sort_by_key(|x| x.name.to_lowercase());
                tv.line = data.lines.iter().position(|x| x.name == name).unwrap_or(0);
                tv.tour = 0;
                tv.times_for = None;
                tv.dirty.insert(name.clone());
                tv.new_line.clear();
                if let Some(g) = l.gui.as_mut() {
                    if let Some(n) = g.timetable.as_ref().map(|p| p.new_line) {
                        g.ui.with::<TextInput, _>(n, |t| t.set(""));
                    }
                }
                status = Some((format!("Line {name} added: give it tours, then save"), false));
            }
        }
        Msg::Reset => {
            if tv.reset_armed {
                tv.reset_armed = false;
                let folder = tv.map_dir.file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default();
                match reset_timetable(&tv.map_dir, &folder) {
                    Ok(msg) => {
                        omsi_cfg::content_changed();
                        tv.loaded = None;
                        status = Some((msg, false));
                    }
                    Err(e) => status = Some((e, true)),
                }
            } else {
                tv.reset_armed = true;
                status = Some(("Press \"Reset timetable\" again to put the map's own timetable back (the changes made here are lost)".into(), false));
            }
        }
        Msg::Tour(i) => tv.tour = i,
        Msg::NewTour => {
            let first = trips_of(tv).first().map(|t| t.0.clone()).unwrap_or_default();
            let Some(line) = tv.data.as_mut().and_then(|d| d.lines.get_mut(tv.line)) else { return };
            let ai_group = line.tours.last().map(|t| t.ai_group.clone()).unwrap_or_else(|| "Busses".into());
            let extra = line.tours.last().map(|t| t.extra.clone()).unwrap_or_default();
            line.tours.push(Tour { number: next_number(&line.tours), ai_group, extra, trips: vec![TourTrip { trip: first, profile: 0, departure: 6.0 * 60.0 }] });
            tv.tour = line.tours.len() - 1;
            tv.dirty.insert(line_name);
        }
        Msg::Offset(s) => tv.offset = s,
        Msg::Until(s) => tv.until = s,
        Msg::CopyTour => {
            let off = tv.offset.trim().parse::<f32>().ok();
            let Some(line) = tv.data.as_mut().and_then(|d| d.lines.get_mut(tv.line)) else { return };
            match (off, line.tours.get(tv.tour).cloned()) {
                (Some(off), Some(mut t)) => {
                    for x in &mut t.trips {
                        x.departure += off;
                    }
                    t.number = next_number(&line.tours);
                    line.tours.push(t);
                    tv.tour = line.tours.len() - 1;
                    tv.dirty.insert(line_name);
                }
                (None, _) => status = Some(("Type the minutes the copy runs later (negative for earlier).".into(), true)),
                _ => {}
            }
        }
        Msg::Repeat => {
            let off = tv.offset.trim().parse::<f32>().ok();
            let until = parse_time(&tv.until);
            let Some(line) = tv.data.as_mut().and_then(|d| d.lines.get_mut(tv.line)) else { return };
            match (off.filter(|o| *o >= 1.0), until, line.tours.get(tv.tour).cloned()) {
                (Some(every), Some(until), Some(base)) => {
                    let made = repeat_tour(&mut line.tours, &base, every, until);
                    if made > 0 {
                        tv.tour = line.tours.len() - 1;
                        tv.dirty.insert(line_name);
                    }
                    status = Some((format!("{made} tour(s) made, every {every} min up to {}", fmt_time(until)), false));
                }
                (None, _, _) => status = Some(("Type the minutes between the tours (1 or more) in the field above".into(), true)),
                (_, None, _) => status = Some(("Type the last departure as h:mm".into(), true)),
                _ => {}
            }
        }
        Msg::DeleteTour => {
            let Some(line) = tv.data.as_mut().and_then(|d| d.lines.get_mut(tv.line)) else { return };
            if tv.tour < line.tours.len() {
                line.tours.remove(tv.tour);
                tv.tour = tv.tour.saturating_sub(1);
                tv.times_for = None;
                tv.dirty.insert(line_name);
            }
        }
        Msg::Departure(i, s) => {
            if let Some(t) = tv.times.get_mut(i) {
                *t = s.clone();
            }
            if let (Some(min), Some(trip)) = (parse_time(&s), tv.data.as_mut().and_then(|d| d.lines.get_mut(tv.line)).and_then(|x| x.tours.get_mut(tv.tour)).and_then(|t| t.trips.get_mut(i))) {
                trip.departure = min;
                tv.dirty.insert(line_name);
            }
        }
        Msg::Trip(i, k) => {
            let names = trips_of(tv);
            if let (Some(name), Some(trip)) = (names.get(k).map(|x| x.0.clone()), tv.data.as_mut().and_then(|d| d.lines.get_mut(tv.line)).and_then(|x| x.tours.get_mut(tv.tour)).and_then(|t| t.trips.get_mut(i))) {
                trip.trip = name;
                trip.profile = 0;
                tv.dirty.insert(line_name);
            }
        }
        Msg::Profile(i, k) => {
            if let Some(trip) = tv.data.as_mut().and_then(|d| d.lines.get_mut(tv.line)).and_then(|x| x.tours.get_mut(tv.tour)).and_then(|t| t.trips.get_mut(i)) {
                trip.profile = k as i32;
                tv.dirty.insert(line_name);
            }
        }
        Msg::RemoveTrip(i) => {
            if let Some(tour) = tv.data.as_mut().and_then(|d| d.lines.get_mut(tv.line)).and_then(|x| x.tours.get_mut(tv.tour)) {
                if i < tour.trips.len() {
                    tour.trips.remove(i);
                    tv.times_for = None;
                    tv.dirty.insert(line_name);
                }
            }
        }
        Msg::AddTrip => {
            let first = trips_of(tv).first().map(|t| t.0.clone()).unwrap_or_default();
            if let Some(tour) = tv.data.as_mut().and_then(|d| d.lines.get_mut(tv.line)).and_then(|x| x.tours.get_mut(tv.tour)) {
                // the next of the alternation (there and back), as far after as the last gap
                let n = tour.trips.len();
                let t = match n {
                    0 => TourTrip { trip: first, profile: 0, departure: 6.0 * 60.0 },
                    1 => TourTrip { departure: tour.trips[0].departure + 30.0, ..tour.trips[0].clone() },
                    _ => TourTrip { departure: tour.trips[n - 1].departure + (tour.trips[n - 1].departure - tour.trips[n - 2].departure).max(1.0), ..tour.trips[n - 2].clone() },
                };
                tour.trips.push(t);
                tv.times_for = None;
                tv.dirty.insert(line_name);
            }
        }
        Msg::Shift(d) => {
            if let Some(tour) = tv.data.as_mut().and_then(|x| x.lines.get_mut(tv.line)).and_then(|x| x.tours.get_mut(tv.tour)) {
                for t in &mut tour.trips {
                    t.departure += d;
                }
                tv.times_for = None;
                tv.dirty.insert(line_name);
            }
        }
        Msg::Save => {
            if tv.dirty.is_empty() {
                return;
            }
            if tv.times.iter().any(|t| parse_time(t).is_none()) {
                status = Some(("A departure is not a time (h:mm).".into(), true));
            } else {
                let (saved, err) = save_all(tv);
                tv.times_for = None;
                status = Some(match err {
                    Some(e) => (format!("Not saved: {e}"), true),
                    None => (format!("{saved} line(s) saved"), false),
                });
            }
        }
    }
    if let Some((s, err)) = status {
        l.state.set_status(s, err);
    }
}
