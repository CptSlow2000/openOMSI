//! The map and the duty: the map (or the server's), free driving or a line and a tour, the
//! place to start; and beside the map the roadbook - the tour's trips, their stops, the IBIS.

use super::super::kit::{self, len, lp, tr};
use super::super::theme::ACCENT;
use super::{Msg, m};
use crate::launcher::drive as logic;
use crate::launcher::state::hhmm;
use crate::launcher::Launcher;
use egui_retained::widgets::{Button, Checkbox, Icon, Select, Text, TextInput};
use egui_retained::{NodeId, ScrollAxes, Ui, Visual, taffy};

#[derive(Clone, Debug)]
pub(in crate::launcher) enum DutyMsg {
    Map(usize),
    LeaveServer,
    Free(bool),
    LineFilter(String),
    Line(String),
    TourFilter(String),
    ShowEnded(bool),
    Tour(String),
    Entry(usize),
    Trip(usize, f64),
    Book(bool),
}

fn d(x: DutyMsg) -> super::super::Msg {
    m(Msg::Duty(x))
}

pub(super) struct DutyStep {
    line_filter: String,
    tour_filter: String,
    show_ended: bool,
    book_open: bool,
    book_shut: bool,
    map_files: Vec<String>,
    server: NodeId,
    server_text: NodeId,
    own_map: NodeId,
    map: NodeId,
    free: NodeId,
    free_note: NodeId,
    duty: NodeId,
    lines: NodeId,
    lines_key: String,
    tour_head: NodeId,
    ended: NodeId,
    book_button: NodeId,
    pick_line: NodeId,
    tour_box: NodeId,
    tours: NodeId,
    tours_key: String,
    entry: NodeId,
    // the roadbook
    book: NodeId,
    book_title: NodeId,
    book_sub: NodeId,
    trips: NodeId,
    trips_key: String,
    ibis: NodeId,
    ibis_key: String,
}

impl DutyStep {
    pub fn build(ui: &mut Ui, p: NodeId) -> DutyStep {
        // the server's map, not to be changed
        let server = ui.column(p);
        kit::gap(ui, server, 8.0);
        let r = ui.row(server);
        kit::gap(ui, r, 8.0);
        ui.add(r, Icon::new("lock"));
        let server_text = kit::text(ui, r, "", "strong");
        let leave = ui.add(server, Button::new(tr("Leave the server")).icon("logout"));
        ui.on_click(leave, d(DutyMsg::LeaveServer));
        ui.set_tooltip(leave, Some(tr("Back to driving alone: the map, the clock and the weather are your own again")));
        let own_map = kit::labelled(ui, p, "Map", 70.0);
        let map = ui.add(own_map, Select::new(Vec::new(), None).on_change(|i| d(DutyMsg::Map(i))));
        kit::grow(ui, map);
        let free = ui.add(p, Checkbox::switch(false, tr("Free drive (no timetable duty)")).on_change(|v| d(DutyMsg::Free(v))));
        let free_note = kit::para(ui, p, &tr("Free driving: the bus starts where you chose, with the traffic and the timetable's buses around it, but no line of your own. Pick the place below or click an entry point on the map."), "dim");
        // the line and the tour
        let duty = ui.column(p);
        kit::grow(ui, duty);
        kit::gap(ui, duty, 8.0);
        kit::text(ui, duty, &tr("Line"), "heading");
        ui.add(duty, TextInput::new("").hint(tr("Filter lines…")).on_change(|s| d(DutyMsg::LineFilter(s))));
        let lines = ui.column(duty);
        ui.add_class(lines, "list");
        ui.set_scroll(lines, ScrollAxes { x: false, y: true });
        ui.style(lines, |s| {
            s.flex_grow = 2.0;
            s.flex_basis = len(0.0);
            s.min_size.height = len(110.0);
        });
        let tour_head = ui.row(duty);
        kit::gap(ui, tour_head, 8.0);
        kit::text(ui, tour_head, &tr("Tour"), "heading");
        ui.spacer(tour_head);
        let ended = ui.add(tour_head, Checkbox::switch(false, tr("Ended tours")).on_change(|v| d(DutyMsg::ShowEnded(v))));
        ui.set_tooltip(ended, Some(tr("Tours whose last trip has already left at the chosen time")));
        let book_button = ui.add(tour_head, Button::new(tr("Roadbook")).icon("receipt_long"));
        ui.on_click(book_button, d(DutyMsg::Book(true)));
        ui.set_tooltip(book_button, Some(tr("The trips of the chosen tour, where they call, and what to type into the IBIS")));
        let pick_line = kit::para(ui, duty, &tr("Pick a line first. As in OMSI, the start time and date then say where in the tour the bus is: the trip under way, or the next to leave."), "dim");
        let tour_box = ui.column(duty);
        kit::grow(ui, tour_box);
        kit::gap(ui, tour_box, 8.0);
        ui.style(tour_box, |s| s.flex_grow = 3.0);
        ui.add(tour_box, TextInput::new("").hint(tr("Filter tours: number, route, stop…")).on_change(|s| d(DutyMsg::TourFilter(s))));
        let tours = ui.column(tour_box);
        ui.add_class(tours, "list");
        kit::grow(ui, tours);
        ui.set_scroll(tours, ScrollAxes { x: false, y: true });
        ui.add(p, egui_retained::widgets::Separator);
        let er = kit::labelled(ui, p, "Start at", 70.0);
        let entry = ui.add(er, Select::new(Vec::new(), None).on_change(|i| d(DutyMsg::Entry(i))));
        kit::grow(ui, entry);
        ui.set_tooltip(er, Some(tr("Where the bus is put down. The orange marks on the map are the same places: click one to take it.")));
        DutyStep {
            line_filter: String::new(),
            tour_filter: String::new(),
            show_ended: false,
            book_open: false,
            book_shut: false,
            map_files: Vec::new(),
            server,
            server_text,
            own_map,
            map,
            free,
            free_note,
            duty,
            lines,
            lines_key: String::new(),
            tour_head,
            ended,
            book_button,
            pick_line,
            tour_box,
            tours,
            tours_key: String::new(),
            entry,
            book: NodeId::dangling(),
            book_title: NodeId::dangling(),
            book_sub: NodeId::dangling(),
            trips: NodeId::dangling(),
            trips_key: String::new(),
            ibis: NodeId::dangling(),
            ibis_key: String::new(),
        }
    }

    /// The roadbook card beside the map.
    pub fn build_book(&mut self, ui: &mut Ui, parent: NodeId) -> NodeId {
        let book = ui.column(parent);
        ui.add_class(book, "card");
        ui.style(book, |s| {
            s.size.width = taffy::Dimension::percent(0.38);
            s.min_size.width = len(290.0);
            s.max_size.width = len(400.0);
            s.flex_shrink = 0.0;
            s.min_size.height = len(0.0);
        });
        let head = ui.row(book);
        kit::gap(ui, head, 8.0);
        ui.add(head, Icon::new("receipt_long"));
        kit::text(ui, head, &tr("Roadbook"), "heading");
        ui.spacer(head);
        let close = ui.add(head, Button::new("").icon("close").class("ghost"));
        ui.on_click(close, d(DutyMsg::Book(false)));
        ui.set_tooltip(close, Some(tr("Put the roadbook away")));
        self.book_title = kit::text(ui, book, "", "strong");
        self.book_sub = kit::para(ui, book, "", "faint");
        self.trips = ui.column(book);
        kit::grow(ui, self.trips);
        ui.set_scroll(self.trips, ScrollAxes { x: false, y: true });
        kit::gap(ui, self.trips, 2.0);
        self.ibis = ui.column(book);
        ui.add_class(self.ibis, "list");
        ui.style(self.ibis, |s| {
            s.padding = taffy::Rect::length(10.0_f32);
            s.gap = taffy::Size { width: lp(6.0), height: lp(6.0) };
            s.flex_shrink = 0.0;
        });
        self.book = book;
        book
    }

    pub fn book_open(&self, l: &Launcher) -> bool {
        self.book_open || (!self.book_shut && l.state.tour().is_some() && !l.state.choice.free)
    }

    pub fn sync(&mut self, ui: &mut Ui, l: &Launcher) {
        let joined = logic::joined_server_name(l);
        ui.set_visible(self.server, joined.is_some());
        ui.set_visible(self.own_map, joined.is_none());
        if let Some(name) = &joined {
            let mname = l.state.map().map(|mp| if mp.friendly.is_empty() { mp.name.clone() } else { mp.friendly.clone() }).unwrap_or_else(|| l.state.choice.map.clone());
            ui.set_text(self.server_text, &format!("{mname} · {name}"));
        } else {
            self.map_files = l.state.maps.iter().map(|mp| mp.file.clone()).collect();
            let names: Vec<String> = l.state.maps.iter().map(|mp| format!("{}{}{}", if l.state.fresh.contains_key(&mp.file) { "★ NEW · " } else { "" }, if mp.friendly.is_empty() { &mp.name } else { &mp.friendly }, if mp.installed { "  (mod)" } else { "" })).collect();
            let sel = l.state.maps.iter().position(|mp| mp.file == l.state.choice.map);
            ui.set_select(self.map, Some(names), sel);
        }
        let free = l.state.choice.free;
        ui.set_checked(self.free, free);
        ui.set_visible(self.free_note, free);
        ui.set_visible(self.duty, !free);
        // where to start
        if let Some(mp) = l.state.map() {
            let mut labels = vec![tr(if free { "Automatic (the map's first)" } else { "Automatic (nearest to the first stop)" })];
            labels.extend(mp.entry_points.iter().map(|e| if e.name.is_empty() { format!("entry {}", e.index + 1) } else { e.name.clone() }));
            let es = if l.state.choice.entry < 0 { 0 } else { (l.state.choice.entry as usize + 1).min(labels.len() - 1) };
            ui.set_select(self.entry, Some(labels), Some(es));
        }
        ui.set_visible(self.book_button, !self.book_open(l) && l.state.line().is_some());
        if !free {
            self.sync_lines(ui, l);
            self.sync_tours(ui, l);
        }
        if self.book_open(l) {
            self.sync_book(ui, l);
        }
    }

    fn sync_lines(&mut self, ui: &mut Ui, l: &Launcher) {
        let q = self.line_filter.to_lowercase();
        let lines: Vec<(String, String, usize)> = l.state.lines.iter().filter(|x| x.user_allowed).filter(|x| q.is_empty() || x.name.to_lowercase().contains(&q)).map(|x| (x.name.clone(), x.termini.join(" · "), x.tours.len())).collect();
        let chosen = l.state.choice.line.clone();
        let key = format!("{q}|{chosen:?}|{}|{}|{}", lines.len(), l.state.loading_lines, l.state.lines_for.0);
        if key == self.lines_key {
            return;
        }
        let first = self.lines_key.is_empty();
        self.lines_key = key;
        ui.clear(self.lines);
        if lines.is_empty() {
            kit::para(ui, self.lines, &tr(if l.state.loading_lines { "Reading the timetable…" } else { "No lines on this date." }), "dim");
        }
        let mut chosen_row = None;
        for (name, termini, tours) in lines {
            let on = chosen.as_deref() == Some(name.as_str());
            let row = ui.column(self.lines);
            ui.add_class(row, "list-row");
            kit::gap(ui, row, 3.0);
            ui.set_selected(row, on);
            ui.on_click(row, d(DutyMsg::Line(name.clone())));
            let top = ui.row(row);
            kit::gap(ui, top, 8.0);
            let badge = kit::text(ui, top, &name, "badge");
            if on {
                ui.add_class(badge, "badge-accent");
                chosen_row = Some(row);
            }
            ui.spacer(top);
            let ic = ui.add(top, Icon::new("event"));
            ui.visual(ic, Visual::new().font_size(11.0_f32));
            let c = kit::text(ui, top, &tours.to_string(), "faint");
            ui.set_tooltip(c, Some(tr("Tours of this line on the chosen day")));
            kit::text(ui, row, &termini, "faint");
        }
        if let (Some(r), true) = (chosen_row, first || true) {
            ui.defer(move |ui| ui.scroll_into_view(r));
        }
    }

    fn sync_tours(&mut self, ui: &mut Ui, l: &Launcher) {
        let line = l.state.line().cloned();
        ui.set_visible(self.pick_line, line.is_none());
        ui.set_visible(self.tour_box, line.is_some());
        ui.set_visible(self.ended, line.is_some());
        let Some(line) = line else { return };
        let now = l.state.choice.time as f64 * 60.0;
        let q = self.tour_filter.trim().to_lowercase();
        let mut ended_count = 0;
        let mut tours = Vec::new();
        for t in &line.tours {
            let (ended, trip) = logic::tour_trip(t, now, &q);
            if trip.is_none() && !t.trips.is_empty() {
                continue;
            }
            if ended {
                ended_count += 1;
                if !self.show_ended {
                    continue;
                }
            }
            tours.push((t, ended, trip));
        }
        let departure = |t: &(&omsi_launcher_lib::TourInfo, bool, Option<usize>)| t.2.and_then(|k| t.0.trips.get(k)).map_or(f64::MAX, |x| x.departure);
        tours.sort_by(|a, b| b.0.runs.cmp(&a.0.runs).then(a.1.cmp(&b.1)).then_with(|| if q.is_empty() { std::cmp::Ordering::Equal } else { departure(a).total_cmp(&departure(b)) }).then_with(|| logic::natural(&a.0.number).cmp(&logic::natural(&b.0.number))));
        let label = format!("{} ({ended_count})", tr("Ended tours"));
        let show = self.show_ended;
        ui.update::<Checkbox>(self.ended, |c| {
            let changed = c.text != label || c.checked != show;
            c.text = label.clone();
            c.checked = show;
            changed
        });
        let chosen = l.state.choice.tour.clone();
        let key = format!("{q}|{}|{chosen:?}|{}|{}|{}|{}", self.show_ended, line.name, l.state.choice.time, l.state.choice.date, tours.len());
        if key == self.tours_key {
            return;
        }
        self.tours_key = key;
        ui.clear(self.tours);
        let searching = !q.is_empty();
        if tours.is_empty() {
            kit::para(ui, self.tours, &tr(if searching { "No tour has a trip still to come that matches." } else { "No tour left at this time: show the ended tours, or start earlier." }), "dim");
        }
        let mut chosen_row = None;
        for (t, ended, k) in tours {
            let trip = k.and_then(|i| t.trips.get(i)).cloned();
            let service = || t.trips.iter().filter(|x| !logic::depot_run(x));
            let from = service().next().or(t.trips.first()).map(|x| x.from.clone()).unwrap_or_default();
            let terminus = service().last().or(t.trips.last()).map(|x| x.terminus.clone()).unwrap_or_default();
            let on = chosen.as_deref() == Some(t.number.as_str());
            let live = t.runs && !ended;
            let row = ui.column(self.tours);
            ui.add_class(row, "list-row");
            kit::gap(ui, row, 3.0);
            ui.set_selected(row, on);
            if on {
                chosen_row = Some(row);
            }
            let pick_trip = trip.as_ref().filter(|_| searching).map(|x| (x.index, x.departure));
            let num = t.number.clone();
            ui.on(row, egui_retained::EventKind::Click, move |cx, _| {
                cx.emit(d(DutyMsg::Tour(num.clone())));
                if let Some((i, dep)) = pick_trip {
                    cx.emit(d(DutyMsg::Trip(i, dep)));
                }
                true
            });
            let top = ui.row(row);
            let n = kit::text(ui, top, &t.number, if live { "strong" } else { "faint" });
            ui.visual(n, Visual::new().font_size(14.5_f32));
            if let Some(x) = &trip {
                let w = kit::right_text(ui, top, &format!("{} - {}", hhmm(x.departure), hhmm(x.arrival)), if live { "accent-text" } else { "faint" });
                let _ = w;
            }
            let route = match &trip {
                Some(x) if searching => format!("{} · {} → {}", x.name, x.from, x.terminus),
                _ if from.is_empty() && terminus.is_empty() => String::new(),
                _ => format!("{from} → {terminus}"),
            };
            if !route.is_empty() {
                kit::text(ui, row, &route, if live { "dim" } else { "faint" });
            }
            let n = t.trips.len();
            let mut sub = format!("{n} trips · {}", t.days);
            if let Some(x) = &trip {
                sub = format!("{sub} · {} {}", logic::trip_duration(x.departure, x.arrival), tr("a trip"));
            }
            if ended {
                sub = format!("{n} trips · {} · ended", t.days);
            }
            if !t.runs {
                sub = match &t.next_run {
                    Some(nr) => format!("{n} trips · {} · runs {nr}", t.days),
                    None => format!("{n} trips · never within a year"),
                };
            }
            kit::text(ui, row, &sub, "faint");
        }
        if let Some(r) = chosen_row {
            ui.defer(move |ui| ui.scroll_into_view(r));
        }
    }

    fn sync_book(&mut self, ui: &mut Ui, l: &Launcher) {
        let (line, tour) = (l.state.line().cloned(), l.state.tour().cloned());
        let from = l.state.first_trip().unwrap_or(0);
        match (&line, &tour, l.state.choice.free) {
            (Some(line), Some(tour), false) => {
                ui.set_text(self.book_title, &format!("{} {} · {} {} · {} {}", tr("Line"), line.name, tr("tour"), tour.number, tr("from"), hhmm(l.state.choice.time as f64 * 60.0)));
                ui.set_text(self.book_sub, &tr("Click a trip to start the tour there."));
            }
            _ => {
                let mp = l.state.map().map(|mp| if mp.friendly.is_empty() { mp.name.clone() } else { mp.friendly.clone() }).unwrap_or_default();
                ui.set_text(self.book_title, &tr("No duty chosen"));
                ui.set_text(self.book_sub, &format!("Free driving on {mp}. Pick a line and a tour to see the roadbook here."));
            }
        }
        let key = format!("{:?}|{:?}|{from}", line.as_ref().map(|x| x.name.clone()), tour.as_ref().map(|t| t.number.clone()));
        if key != self.trips_key {
            self.trips_key = key;
            ui.clear(self.trips);
            let mut first_row = None;
            if let (Some(tour), false) = (&tour, l.state.choice.free) {
                for (i, t) in tour.trips.iter().enumerate() {
                    let head = ui.column(self.trips);
                    ui.add_class(head, "list-row");
                    kit::gap(ui, head, 2.0);
                    let earlier = i < from;
                    let k = i.saturating_sub(from);
                    if !earlier && k == 0 {
                        ui.set_selected(head, true);
                        first_row = Some(head);
                    } else {
                        ui.on_click(head, d(DutyMsg::Trip(t.index, t.departure)));
                    }
                    let what = if earlier { tr("Earlier") } else if k == 0 { tr("Your first trip") } else { tr("Then") };
                    kit::text(ui, head, &format!("{what} · {} → {}", if t.from.is_empty() { "?" } else { &t.from }, t.terminus), if earlier { "faint" } else { "strong" });
                    let what_line = if t.line.is_empty() { "depot run".to_string() } else { format!("line {}", t.line) };
                    kit::text(ui, head, &format!("{} - {} · {:.1} km · {what_line} · {}", hhmm(t.departure), hhmm(t.arrival), t.km, t.name), "faint");
                    if earlier {
                        continue;
                    }
                    // the stops on a timeline
                    let n = t.stops.len();
                    for (s, st) in t.stops.iter().enumerate() {
                        let r = ui.row(self.trips);
                        ui.style(r, |st| {
                            st.padding = taffy::Rect { left: lp(10.0), right: lp(6.0), top: lp(1.0), bottom: lp(1.0) };
                            st.gap = taffy::Size { width: lp(10.0), height: lp(0.0) };
                            st.flex_shrink = 0.0;
                        });
                        let end = s == 0 || s + 1 == n;
                        let tm = kit::text(ui, r, &hhmm(st.arr), if end { "strong" } else { "dim" });
                        ui.style(tm, |x| x.size.width = len(42.0));
                        let dot = ui.add(r, super::super::kit::Dot::new(if end { ACCENT } else { egui_retained::Color32::from_gray(110) }));
                        let _ = dot;
                        let nm = ui.add(r, Text::new(st.name.clone()));
                        kit::grow(ui, nm);
                        if end {
                            ui.add_class(nm, "strong");
                        }
                        if s == 0 {
                            kit::text(ui, r, &format!("dep {}", hhmm(st.dep)), "faint");
                        }
                    }
                }
            }
            if let Some(r) = first_row {
                ui.defer(move |ui| ui.scroll_into_view(r));
            }
        }
        // the IBIS
        let ikey = format!("{:?}", l.state.ibis.as_ref().map(|(k, r)| (k.clone(), r.is_ok())));
        if ikey != self.ibis_key {
            self.ibis_key = ikey;
            ui.clear(self.ibis);
            let h = ui.row(self.ibis);
            kit::gap(ui, h, 8.0);
            ui.add(h, Icon::new("keyboard"));
            kit::text(ui, h, "IBIS", "heading");
            match l.state.ibis.clone() {
                None => {
                    kit::para(ui, self.ibis, &tr("Pick a line to see what to type into the IBIS. Shift+U in the game types it for you and puts the bus into service."), "dim");
                }
                Some((_, Err(e))) => {
                    kit::para(ui, self.ibis, &e, "danger-text");
                }
                Some((_, Ok(i))) if i.routes.is_empty() => {
                    kit::para(ui, self.ibis, &format!("Depot file {} has no entries for this line. Type line {} and the terminus code by hand, or press Shift+U.", i.hof, i.line_code), "dim");
                }
                Some((_, Ok(i))) => {
                    for rt in i.routes.iter().take(3) {
                        let code = rt.code.clone();
                        let (lc, rc) = if code.len() > 2 { (code[..code.len() - 2].to_string(), code[code.len() - 2..].to_string()) } else { (i.line_code.clone(), code.clone()) };
                        let r = ui.row(self.ibis);
                        kit::gap(ui, r, 6.0);
                        let nm = ui.add(r, Text::new(if rt.name.is_empty() { rt.terminus.clone() } else { rt.name.clone() }));
                        kit::grow(ui, nm);
                        for (label, v) in [("line", lc), ("route", rc)] {
                            kit::text(ui, r, label, "faint");
                            let b = kit::text(ui, r, &v, "badge");
                            ui.add_class(b, "badge-accent");
                        }
                    }
                    kit::para(ui, self.ibis, &format!("Depot file {} · Shift+U in the game types it for you", i.hof), "faint");
                }
            }
        }
    }
}

pub(super) fn handle(l: &mut Launcher, msg: DutyMsg) {
    let step = |l: &mut Launcher, f: &mut dyn FnMut(&mut DutyStep)| {
        if let Some(dp) = l.gui.as_mut().and_then(|g| g.drive_mut()) {
            f(dp.duty_mut());
        }
    };
    match msg {
        DutyMsg::Map(i) => {
            let f = l.gui.as_mut().and_then(|g| g.drive_mut()).and_then(|dp| dp.duty_mut().map_files.get(i).cloned());
            if let Some(f) = f {
                l.state.select_map(&f);
            }
        }
        DutyMsg::LeaveServer => l.state.leave_server(),
        DutyMsg::Free(v) => {
            l.state.choice.free = v;
            l.state.touched();
        }
        DutyMsg::LineFilter(s) => step(l, &mut |st| st.line_filter = s.clone()),
        DutyMsg::TourFilter(s) => step(l, &mut |st| st.tour_filter = s.clone()),
        DutyMsg::ShowEnded(v) => step(l, &mut |st| st.show_ended = v),
        DutyMsg::Line(n) => {
            if l.state.choice.line.as_deref() != Some(n.as_str()) {
                l.state.choice.line = Some(n);
                l.state.choice.tour = None;
                l.state.touched();
            }
        }
        DutyMsg::Tour(num) => {
            let runs = l.state.line().and_then(|ln| ln.tours.iter().find(|t| t.number == num)).map(|t| (t.runs, t.next_run.clone()));
            if let Some((false, Some(n))) = runs {
                l.state.choice.date = n;
                l.state.load_lines();
            }
            l.state.choice.tour = Some(num);
            l.state.touched();
            l.state.load_ibis();
            step(l, &mut |st| {
                if !st.book_shut {
                    st.book_open = true;
                }
            });
        }
        DutyMsg::Entry(es) => {
            l.state.choice.entry = es as i32 - 1;
            l.state.touched();
        }
        DutyMsg::Trip(i, dep) => l.state.pick_trip(i, dep),
        DutyMsg::Book(open) => {
            if open {
                l.state.load_ibis();
            }
            step(l, &mut |st| {
                st.book_open = open;
                st.book_shut = !open;
            });
        }
    }
}
