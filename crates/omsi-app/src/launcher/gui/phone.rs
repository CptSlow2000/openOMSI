//! The launcher on a phone or a tablet (or a window too narrow for the sidebar): a tab bar
//! at the foot - Play, Online, Mods, More - instead of the sidebar; a Play screen with the
//! bus large and the duty as big cards over one Start button; every list choice (map, bus,
//! livery, line, tour) on a sheet of its own that fills the screen, with a search and a way
//! back; the rest of the choices on the Drive page's own steps, and the pages a phone needs
//! less often opened from More - all under a bar with a way back.

use super::kit::{self, len, lp, tr};
use super::stage::{Stage, StagePointer};
use super::theme::{ACCENT, OK, WARN};
use super::Msg as Top;
use crate::launcher::phone::{Sheet, Tab};
use crate::launcher::{Launcher, Page};
use egui_retained::widgets::{Button, Icon, Text, TextInput};
use egui_retained::{Color32, Layer, NodeId, ScrollAxes, Ui, Visual, taffy};
use std::cell::RefCell;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

const TABS: [(Tab, &str, &str); 4] = [(Tab::Play, "Play", "directions_bus"), (Tab::Online, "Online", "groups"), (Tab::Mods, "Mods", "extension"), (Tab::More, "More", "menu")];

/// The pages More opens.
const MORE: [(Page, &str, &str, &str); 7] = [
    (Page::Profile, "Profile", "badge", "Your driver, level and records"),
    (Page::Settings, "Settings", "tune", "Graphics, sound, gameplay"),
    (Page::Controls, "Controls", "sports_esports", "Touch, wheels and gamepads"),
    (Page::Sessions, "Sessions", "terminal", "Games running and their logs"),
    (Page::Tutorials, "Tutorials", "help", "Learn to drive the buses"),
    (Page::Timetable, "Timetable", "schedule", "The map's lines and trips"),
    (Page::Setup, "Setup", "folder_open", "The OMSI 2 folder and content"),
];

/// Below this width (points) the launcher is laid out as on a phone.
pub const NARROW: f32 = 760.0;

#[derive(Clone, Debug)]
pub(in crate::launcher) enum Msg {
    Tab(Tab),
    Back,
    Open(Sheet),
    Search(String),
    Map(String),
    Bus(String),
    Paint(String),
    Free,
    Line(String),
    Tour(String),
    More(Page),
    Start,
    Continue,
    Leave,
}

fn m(x: Msg) -> Top {
    Top::Phone(x)
}

/// The sheets that are the Drive page's own step under a bar (the rest are lists).
fn drive_step(s: Sheet) -> Option<usize> {
    match s {
        Sheet::Vehicle => Some(0),
        Sheet::Time => Some(1),
        Sheet::Start | Sheet::Roadbook => Some(2),
        _ => None,
    }
}

fn sheet_title(s: Sheet) -> &'static str {
    match s {
        Sheet::Map => "Choose the map",
        Sheet::Bus => "Choose the bus",
        Sheet::Livery => "Choose the livery",
        Sheet::Vehicle => "Vehicle settings",
        Sheet::Duty => "Line or free drive",
        Sheet::Tour => "Choose the tour",
        Sheet::Time => "Time and weather",
        Sheet::Start => "Start options",
        Sheet::Roadbook => "Roadbook and IBIS",
    }
}

/// What the phone layout shows in the page's place.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Shown {
    Play,
    More,
    /// A page of the desktop's (with a bar and a way back when `back`).
    Page(Page, bool),
}

pub(super) fn shown(l: &Launcher) -> Shown {
    if let Some(step) = l.phone.sheet.and_then(drive_step) {
        let _ = step;
        return Shown::Page(Page::Drive, true);
    }
    match (l.phone.tab, l.phone.page) {
        (Tab::Play, _) => Shown::Play,
        (Tab::Online, _) => Shown::Page(Page::Multiplayer, false),
        (Tab::Mods, _) => Shown::Page(Page::Mods, false),
        (Tab::More, None) => Shown::More,
        (Tab::More, Some(p)) => Shown::Page(p, true),
    }
}

pub(in crate::launcher) struct Phone {
    pub tabbar: NodeId,
    tabs: Vec<(Tab, NodeId)>,
    more_dot: NodeId,
    pub backbar: NodeId,
    back_title: NodeId,
    pub play: NodeId,
    pub more: NodeId,
    stage: NodeId,
    pub stage_ptr: Rc<RefCell<StagePointer>>,
    bus_maker: NodeId,
    bus_name: NodeId,
    livery: NodeId,
    vehicle_btn: NodeId,
    roadbook_btn: NodeId,
    server_chip: NodeId,
    server_name: NodeId,
    cards_box: NodeId,
    cards: Vec<(NodeId, NodeId, NodeId)>,
    cont: NodeId,
    start: NodeId,
    sheet: NodeId,
    sheet_list: NodeId,
    sheet_search: NodeId,
    sheet_title: NodeId,
    sheet_open: Option<Sheet>,
    sheet_key: u64,
}

impl Phone {
    /// The phone's parts in the shell: the tab bar at the foot of `app`, the bar with the
    /// way back over the page `host`, the Play and More screens in `host`.
    pub fn build(ui: &mut Ui, app: NodeId, host: NodeId) -> Phone {
        // the tab bar
        let tabbar = ui.row(app);
        ui.add_class(tabbar, "header");
        ui.style(tabbar, |s| {
            s.justify_content = Some(taffy::JustifyContent::SpaceAround);
            s.padding = taffy::Rect { left: lp(6.0), right: lp(6.0), top: lp(4.0), bottom: lp(6.0) };
            s.flex_shrink = 0.0;
        });
        let mut tabs = Vec::new();
        let mut more_dot = NodeId::dangling();
        for (t, name, icon) in TABS {
            let b = ui.column(tabbar);
            ui.add_class(b, "tab");
            ui.style(b, |s| {
                s.align_items = Some(taffy::AlignItems::Center);
                s.padding = taffy::Rect { left: lp(18.0), right: lp(18.0), top: lp(5.0), bottom: lp(5.0) };
                s.gap = taffy::Size { width: lp(0.0), height: lp(2.0) };
                s.flex_grow = 1.0;
            });
            ui.on_click(b, m(Msg::Tab(t)));
            let i = ui.add(b, Icon::new(icon));
            ui.visual(i, Visual::new().font_size(22.0_f32));
            let r = ui.row(b);
            let tx = ui.add(r, Text::new(tr(name)));
            ui.visual(tx, Visual::new().font_size(11.5_f32));
            if t == Tab::More {
                more_dot = ui.add(r, kit::Dot::new(OK));
            }
            tabs.push((t, b));
        }
        // the bar over a page opened from More (or a step of the Drive page)
        let backbar = ui.insert(host, 0, egui_retained::widgets::Div::row());
        ui.add_class(backbar, "header");
        kit::gap(ui, backbar, 8.0);
        ui.style(backbar, |s| s.flex_shrink = 0.0);
        let back = ui.add(backbar, Button::new("").icon("arrow_back").class("ghost"));
        ui.on_click(back, m(Msg::Back));
        let back_title = kit::text(ui, backbar, "", "strong");
        ui.visual(back_title, Visual::new().font_size(17.0_f32));
        // Play
        let play = ui.column(host);
        ui.style(play, |s| {
            s.flex_grow = 1.0;
            s.min_size.height = len(0.0);
            s.padding = taffy::Rect::length(12.0_f32);
            s.gap = taffy::Size { width: lp(12.0), height: lp(12.0) };
        });
        let pic = ui.column(play);
        ui.style(pic, |s| {
            s.flex_grow = 1.0;
            s.flex_basis = len(0.0);
            s.min_size = taffy::Size { width: len(0.0), height: len(140.0) };
            s.position = taffy::Position::Relative;
        });
        let (st, stage_ptr) = Stage::new(&tr("Loading…"));
        let stage = ui.add(pic, st);
        ui.visual(stage, Visual::new().cursor(egui_retained::Cursor::Grab));
        ui.style(stage, |s| {
            s.flex_grow = 1.0;
            s.min_size.height = len(0.0);
        });
        // over the picture: the bus's name and livery at its foot, its settings and the
        // roadbook top right, the server joined top left
        let foot = ui.row(pic);
        ui.style(foot, |s| {
            s.position = taffy::Position::Absolute;
            s.inset = taffy::Rect { left: kit::lpa(0.0), right: kit::lpa(0.0), top: taffy::LengthPercentageAuto::auto(), bottom: kit::lpa(0.0) };
            s.padding = taffy::Rect { left: lp(14.0), right: lp(10.0), top: lp(8.0), bottom: lp(8.0) };
            s.gap = taffy::Size { width: lp(10.0), height: lp(0.0) };
        });
        ui.visual(foot, Visual::new().background(Color32::from_black_alpha(140)).radius(8.0_f32));
        let names = ui.column(foot);
        kit::grow(ui, names);
        let bus_maker = kit::text(ui, names, "", "dim");
        let bus_name = kit::text(ui, names, "", "strong");
        ui.visual(bus_name, Visual::new().font_size(17.0_f32));
        let livery = ui.add(foot, Button::new("").icon("palette"));
        ui.on_click(livery, m(Msg::Open(Sheet::Livery)));
        let corner = ui.column(pic);
        ui.style(corner, |s| {
            s.position = taffy::Position::Absolute;
            s.inset = taffy::Rect { left: taffy::LengthPercentageAuto::auto(), right: kit::lpa(10.0), top: kit::lpa(10.0), bottom: taffy::LengthPercentageAuto::auto() };
            s.gap = taffy::Size { width: lp(0.0), height: lp(8.0) };
        });
        let vehicle_btn = ui.add(corner, Button::new("").icon("settings"));
        ui.set_tooltip(vehicle_btn, Some(tr("Vehicle settings")));
        ui.on_click(vehicle_btn, m(Msg::Open(Sheet::Vehicle)));
        let roadbook_btn = ui.add(corner, Button::new("").icon("receipt_long"));
        ui.set_tooltip(roadbook_btn, Some(tr("Roadbook and IBIS")));
        ui.on_click(roadbook_btn, m(Msg::Open(Sheet::Roadbook)));
        let server_chip = ui.row(pic);
        ui.style(server_chip, |s| {
            s.position = taffy::Position::Absolute;
            s.inset = taffy::Rect { left: kit::lpa(10.0), right: taffy::LengthPercentageAuto::auto(), top: kit::lpa(10.0), bottom: taffy::LengthPercentageAuto::auto() };
            s.padding = taffy::Rect { left: lp(12.0), right: lp(4.0), top: lp(4.0), bottom: lp(4.0) };
            s.gap = taffy::Size { width: lp(8.0), height: lp(0.0) };
        });
        ui.visual(server_chip, Visual::new().background(Color32::from_black_alpha(150)).radius(17.0_f32));
        let ic = ui.add(server_chip, Icon::new("dns"));
        ui.visual(ic, Visual::new().color(OK));
        let server_name = kit::text(ui, server_chip, "", "ok-text");
        let x = ui.add(server_chip, Button::new("").icon("close").class("ghost"));
        ui.set_tooltip(x, Some(tr("Leave Server")));
        ui.on_click(x, m(Msg::Leave));
        // the duty as cards, Start under them
        let cards_box = ui.column(play);
        ui.style(cards_box, |s| {
            s.gap = taffy::Size { width: lp(8.0), height: lp(8.0) };
            s.flex_shrink = 0.0;
        });
        let mut cards = Vec::new();
        for (icon, label, sheet) in [("map", "Map", Sheet::Map), ("directions_bus", "Bus", Sheet::Bus), ("route", "Duty", Sheet::Duty), ("partly_cloudy_day", "Time & weather", Sheet::Time), ("tune", "Start options", Sheet::Start)] {
            let c = ui.row(cards_box);
            ui.add_class(c, "list-row");
            ui.style(c, |s| {
                s.padding = taffy::Rect { left: lp(14.0), right: lp(10.0), top: lp(8.0), bottom: lp(8.0) };
                s.gap = taffy::Size { width: lp(14.0), height: lp(0.0) };
            });
            ui.visual(c, Visual::new().background(Color32::from_white_alpha(8)).radius(14.0_f32));
            ui.on_click(c, m(Msg::Open(sheet)));
            let i = ui.add(c, Icon::new(icon));
            ui.visual(i, Visual::new().color(ACCENT).font_size(20.0_f32));
            let t = ui.column(c);
            kit::grow(ui, t);
            let lb = kit::text(ui, t, &tr(label), "dim");
            ui.visual(lb, Visual::new().font_size(11.0_f32));
            let v = kit::text(ui, t, "", "strong");
            ui.visual(v, Visual::new().font_size(14.5_f32));
            let ch = ui.add(c, Icon::new("chevron_right"));
            ui.add_class(ch, "faint");
            cards.push((c, i, v));
        }
        let sr = ui.row(cards_box);
        kit::gap(ui, sr, 8.0);
        let cont = ui.add(sr, Button::new(tr("Continue")).icon("history"));
        ui.style(cont, |s| s.min_size.height = len(48.0));
        ui.on_click(cont, m(Msg::Continue));
        let start = ui.add(sr, Button::new(tr("Drive")).icon("play_arrow").class("primary"));
        kit::grow(ui, start);
        ui.style(start, |s| s.min_size.height = len(48.0));
        ui.on_click(start, m(Msg::Start));
        // More
        let more = ui.row(host);
        ui.style(more, |s| {
            s.flex_wrap = taffy::FlexWrap::Wrap;
            s.align_content = Some(taffy::AlignContent::FlexStart);
            s.padding = taffy::Rect::length(12.0_f32);
            s.gap = taffy::Size { width: lp(10.0), height: lp(10.0) };
            s.flex_grow = 1.0;
        });
        ui.set_scroll(more, ScrollAxes { x: false, y: true });
        for (page, name, icon, sub) in MORE {
            let t = ui.column(more);
            ui.add_class(t, "list-row");
            ui.visual(t, Visual::new().background(Color32::from_white_alpha(8)).radius(14.0_f32));
            ui.style(t, |s| {
                s.flex_grow = 1.0;
                s.flex_basis = taffy::Dimension::percent(0.4);
                s.min_size = taffy::Size { width: len(150.0), height: len(104.0) };
                s.padding = taffy::Rect::length(16.0_f32);
                s.gap = taffy::Size { width: lp(0.0), height: lp(4.0) };
            });
            ui.on_click(t, m(Msg::More(page)));
            let i = ui.add(t, Icon::new(icon));
            ui.visual(i, Visual::new().color(ACCENT).font_size(24.0_f32));
            ui.spacer(t);
            let n = kit::text(ui, t, &tr(name), "strong");
            ui.visual(n, Visual::new().font_size(15.0_f32));
            kit::text(ui, t, &tr(sub), "dim");
        }
        // the sheet over everything
        let sheet = ui.column(ui.root(Layer::Overlay));
        ui.add_class(sheet, "page");
        ui.style(sheet, |s| {
            s.position = taffy::Position::Absolute;
            s.inset = taffy::Rect { left: kit::lpa(0.0), right: kit::lpa(0.0), top: kit::lpa(0.0), bottom: kit::lpa(0.0) };
        });
        ui.visual(sheet, Visual::new().background(Color32::from_rgb(14, 15, 18)));
        let sb = ui.row(sheet);
        ui.add_class(sb, "header");
        kit::gap(ui, sb, 8.0);
        let back = ui.add(sb, Button::new("").icon("arrow_back").class("ghost"));
        ui.on_click(back, m(Msg::Back));
        let sheet_title = kit::text(ui, sb, "", "strong");
        ui.visual(sheet_title, Visual::new().font_size(18.0_f32));
        let sheet_search = ui.add(sb, TextInput::new("").hint(tr("Search…")).on_change(|s| m(Msg::Search(s))));
        kit::grow(ui, sheet_search);
        let sheet_list = ui.column(sheet);
        kit::grow(ui, sheet_list);
        ui.set_scroll(sheet_list, ScrollAxes { x: false, y: true });
        ui.style(sheet_list, |s| {
            s.min_size.height = len(0.0);
            s.padding = taffy::Rect::length(8.0_f32);
            s.gap = taffy::Size { width: lp(6.0), height: lp(6.0) };
        });
        ui.set_visible(sheet, false);
        ui.set_visible(tabbar, false);
        ui.set_visible(backbar, false);
        ui.set_visible(play, false);
        ui.set_visible(more, false);
        Phone {
            tabbar,
            tabs,
            more_dot,
            backbar,
            back_title,
            play,
            more,
            stage,
            stage_ptr,
            bus_maker,
            bus_name,
            livery,
            vehicle_btn,
            roadbook_btn,
            server_chip,
            server_name,
            cards_box,
            cards,
            cont,
            start,
            sheet,
            sheet_list,
            sheet_search,
            sheet_title,
            sheet_open: None,
            sheet_key: 0,
        }
    }

    pub fn stage_node(&self) -> NodeId {
        self.stage
    }

    /// The phone's parts shown (`on`) or not, and kept in step.
    pub fn sync(&mut self, ui: &mut Ui, l: &Launcher, on: bool) {
        let what = shown(l);
        ui.set_visible(self.tabbar, on);
        ui.set_visible(self.play, on && what == Shown::Play);
        ui.set_visible(self.more, on && what == Shown::More);
        let back = match (on, what) {
            (true, Shown::Page(p, true)) => Some(l.phone.sheet.filter(|s| drive_step(*s).is_some()).map(sheet_title).or_else(|| MORE.iter().find(|x| x.0 == p).map(|x| x.1)).unwrap_or("")),
            _ => None,
        };
        ui.set_visible(self.backbar, back.is_some());
        if let Some(t) = back {
            ui.set_text(self.back_title, &tr(t));
        }
        let sheet = l.phone.sheet.filter(|s| on && drive_step(*s).is_none());
        ui.set_visible(self.sheet, sheet.is_some());
        if !on {
            return;
        }
        for (t, n) in &self.tabs {
            ui.set_selected(*n, *t == l.phone.tab);
        }
        ui.set_visible(self.more_dot, l.state.instances.iter().any(|i| i.running));
        if what == Shown::Play {
            self.sync_play(ui, l);
        }
        if let Some(s) = sheet {
            self.sync_sheet(ui, l, s);
        } else {
            self.sheet_open = None;
        }
    }

    fn sync_play(&mut self, ui: &mut Ui, l: &Launcher) {
        // a phone held sideways: the bus left, the cards right
        let screen = ui.screen_size();
        let wide = screen.x > 1.35 * screen.y;
        ui.style(self.play, |s| s.flex_direction = if wide { taffy::FlexDirection::Row } else { taffy::FlexDirection::Column });
        ui.style(self.cards_box, |s| {
            s.size.width = if wide { taffy::Dimension::percent(0.46) } else { taffy::Dimension::auto() };
        });
        let (name, maker) = l.state.bus().map(|b| (b.name.clone(), b.manufacturer.clone())).unwrap_or_else(|| (tr("Choose a bus"), String::new()));
        ui.set_text(self.bus_name, &name);
        ui.set_text(self.bus_maker, &maker);
        let paints = l.state.bus().map(|b| b.paints.len()).unwrap_or(0);
        ui.set_visible(self.livery, paints > 0);
        if paints > 0 {
            let label = if l.state.choice.paint.is_empty() { l.state.bus().map(crate::launcher::drive::default_livery_label).unwrap_or("Default paint").to_string() } else { l.state.choice.paint.clone() };
            ui.set_text(self.livery, &label);
        }
        ui.set_visible(self.vehicle_btn, l.state.bus().is_some());
        ui.set_visible(self.roadbook_btn, !l.state.choice.free && l.state.choice.line.is_some() && l.state.choice.tour.is_some());
        ui.set_visible(self.server_chip, l.state.joined_server.is_some());
        if let Some(sn) = &l.state.joined_server {
            let title = l.state.server_info.get(sn).and_then(|x| x.1.as_ref().ok()).map(|i| i.name.clone()).unwrap_or_else(|| sn.clone());
            ui.set_text(self.server_name, &title);
        }
        let map = l.state.map().map(|m| if m.friendly.is_empty() { m.name.clone() } else { m.friendly.clone() }).unwrap_or_else(|| tr("Choose a map"));
        let incomplete = l.state.bus().is_some_and(|b| !b.missing_packs.is_empty());
        let values = [(map, false), (name, incomplete), (duty_text(l), false), (time_text(l), false), (start_text(l), false)];
        for ((_, icon, v), (text, warn)) in self.cards.iter().zip(values) {
            ui.set_text(*v, &text);
            ui.visual(*icon, Visual::new().color(if warn { WARN } else { ACCENT }).font_size(20.0_f32));
        }
        // (on a server its clock and weather are the server's)
        if let Some((c, _, _)) = self.cards.get(3) {
            ui.set_disabled(*c, l.state.joined_server.is_some());
        }
        let cont = l.state.joined_server.is_none() && l.last_situation_names().is_some();
        ui.set_visible(self.cont, cont);
        let free = l.state.choice.free || l.state.choice.line.is_none();
        ui.set_text(self.start, &tr(if l.state.joined_server.is_some() { "Join and drive" } else if free { "Drive" } else { "Start the duty" }));
    }

    fn sync_sheet(&mut self, ui: &mut Ui, l: &Launcher, s: Sheet) {
        if self.sheet_open != Some(s) {
            self.sheet_open = Some(s);
            self.sheet_key = 0;
            ui.set_text(self.sheet_title, &tr(sheet_title(s)));
            ui.with::<TextInput, _>(self.sheet_search, |t| t.set(""));
            ui.set_visible(self.sheet_search, matches!(s, Sheet::Map | Sheet::Bus | Sheet::Duty));
        }
        let q = l.phone.filter.to_lowercase();
        let found = |t: &str| q.is_empty() || t.to_lowercase().contains(&q);
        // (title, line under it, chosen, badge, message)
        let mut rows: Vec<(String, String, bool, Option<(&str, Color32)>, Msg)> = Vec::new();
        let mut note: Option<String> = None;
        match s {
            Sheet::Map => {
                for x in l.state.maps.iter().filter(|x| found(&format!("{} {} {}", x.friendly, x.name, x.file))) {
                    let name = if x.friendly.is_empty() { x.name.clone() } else { x.friendly.clone() };
                    rows.push((name, x.description.lines().next().unwrap_or("").to_string(), x.file == l.state.choice.map, x.installed.then_some(("MOD", ACCENT)), Msg::Map(x.file.clone())));
                }
            }
            Sheet::Bus => {
                let norm = |f: &str| f.replace('\\', "/").to_ascii_lowercase();
                let allowed: Option<std::collections::HashSet<String>> = l.state.host_vehicles().map(|v| v.iter().map(|f| norm(f)).collect());
                for v in l.state.vehicles.iter().filter(|v| found(&format!("{} {} {}", v.name, v.manufacturer, v.file))).filter(|v| allowed.as_ref().is_none_or(|a| a.contains(&norm(&v.file)))) {
                    let mut sub = v.manufacturer.clone();
                    sub = format!("{sub}{}{}", if sub.is_empty() { "" } else { " · " }, crate::launcher::drive::liveries_text(v.paints.len()));
                    let incomplete = !v.missing_packs.is_empty();
                    if incomplete {
                        sub = format!("{sub} · {}", tr("parts missing"));
                    }
                    let badge = if incomplete { Some(("PARTS", WARN)) } else if v.installed { Some(("MOD", ACCENT)) } else { None };
                    rows.push((v.name.clone(), sub, v.file == l.state.choice.bus, badge, Msg::Bus(v.file.clone())));
                }
                if rows.is_empty() {
                    note = Some(tr(if l.state.loading_content { "Reading the buses…" } else { "No bus matches." }));
                }
            }
            Sheet::Livery => {
                let default = l.state.bus().map(crate::launcher::drive::default_livery_label).unwrap_or("Default paint").to_string();
                for p in std::iter::once(String::new()).chain(l.state.bus().map(|b| b.paints.clone()).unwrap_or_default()) {
                    let name = if p.is_empty() { default.clone() } else { p.clone() };
                    rows.push((name, String::new(), p == l.state.choice.paint, None, Msg::Paint(p)));
                }
            }
            Sheet::Duty => {
                let free = l.state.choice.free || l.state.choice.line.is_none();
                rows.push((tr("Free drive"), tr("No timetable: drive where you like, the buses and traffic about you"), free, None, Msg::Free));
                for x in l.state.lines.iter().filter(|x| x.user_allowed).filter(|x| found(&format!("{} {}", x.name, x.termini.join(" ")))) {
                    rows.push((format!("{} {}", tr("Line"), x.name), format!("{} · {} {}", x.termini.join(" – "), x.tours.len(), tr("tours")), !free && l.state.choice.line.as_deref() == Some(x.name.as_str()), None, Msg::Line(x.name.clone())));
                }
                if l.state.loading_lines {
                    note = Some(tr("Reading the timetable…"));
                } else if rows.len() == 1 {
                    note = Some(tr("The map has no lines to drive."));
                }
            }
            Sheet::Tour => match l.state.line() {
                Some(line) => {
                    let mut tours: Vec<&omsi_launcher_lib::TourInfo> = line.tours.iter().collect();
                    tours.sort_by(|a, b| b.runs.cmp(&a.runs).then_with(|| crate::launcher::drive::natural(&a.number).cmp(&crate::launcher::drive::natural(&b.number))));
                    for t in tours {
                        let when = format!("{} – {}", crate::launcher::state::hhmm(t.first), crate::launcher::state::hhmm(t.last));
                        let sub = if t.runs { format!("{when} · {} trips · {}", t.trips.len(), t.days) } else { format!("{when} · {} · runs {}", t.days, t.next_run.clone().unwrap_or_else(|| "never".into())) };
                        rows.push((format!("{} {}", tr("Tour"), t.number), sub, l.state.choice.tour.as_deref() == Some(t.number.as_str()), (!t.runs).then_some(("OTHER DAY", Color32::from_gray(120))), Msg::Tour(t.number.clone())));
                    }
                }
                None => note = Some(tr("Choose a line first.")),
            },
            _ => {}
        }
        let key = {
            let mut h = std::collections::hash_map::DefaultHasher::new();
            (s as u8, &note).hash(&mut h);
            for r in &rows {
                (&r.0, &r.1, r.2, r.3.map(|b| b.0)).hash(&mut h);
            }
            h.finish() | 1
        };
        if key == self.sheet_key {
            return;
        }
        self.sheet_key = key;
        ui.clear(self.sheet_list);
        if let Some(n) = note {
            kit::para(ui, self.sheet_list, &n, "dim");
        }
        for (title, sub, chosen, badge, msg) in rows.into_iter().take(400) {
            let r = ui.row(self.sheet_list);
            ui.add_class(r, "list-row");
            ui.set_selected(r, chosen);
            ui.style(r, |s| {
                s.padding = taffy::Rect { left: lp(16.0), right: lp(14.0), top: lp(10.0), bottom: lp(10.0) };
                s.gap = taffy::Size { width: lp(10.0), height: lp(0.0) };
                s.min_size.height = len(58.0);
            });
            ui.visual(r, Visual::new().background(Color32::from_white_alpha(7)).radius(10.0_f32));
            ui.on_click(r, m(msg));
            let t = ui.column(r);
            kit::grow(ui, t);
            let n = kit::text(ui, t, &title, "strong");
            ui.visual(n, Visual::new().font_size(15.5_f32));
            if !sub.is_empty() {
                kit::text(ui, t, &sub, "dim");
            }
            if let Some((b, c)) = badge {
                let bn = kit::text(ui, r, b, "badge");
                ui.visual(bn, Visual::new().color(c));
            }
            if chosen {
                let i = ui.add(r, Icon::new("check_circle"));
                ui.visual(i, Visual::new().color(ACCENT));
            }
        }
    }
}

fn duty_text(l: &Launcher) -> String {
    match (&l.state.choice.line, &l.state.choice.tour, l.state.choice.free) {
        (_, _, true) | (None, _, _) => tr("Free drive"),
        (Some(line), Some(t), _) => format!("{} {line} · {} {t}", tr("Line"), tr("tour")),
        (Some(line), None, _) => format!("{} {line} · {}", tr("Line"), tr("choose a tour")),
    }
}

fn time_text(l: &Launcher) -> String {
    let (y, mo, d) = crate::launcher::ui::parse_date(&l.state.choice.date);
    let weather = match l.state.choice.weather.strip_prefix("metar:") {
        Some(c) => format!("live {c}"),
        None if l.state.choice.weather == "cycle" => "weather cycle".into(),
        None => match crate::weather_setup::custom_weather(Some(&l.state.choice.weather)) {
            Some(c) => format!("custom · {:.0}°C · {:.0}% RH", c.temp_c, c.humidity),
            None => l.state.weathers.iter().find(|w| w.file == l.state.choice.weather).map(|w| w.name.clone()).unwrap_or_else(|| "map weather".into()),
        },
    };
    format!("{:02}:{:02} · {d} {} {y} · {weather}", l.state.choice.time / 60, l.state.choice.time % 60, &crate::launcher::ui::MONTHS[(mo as usize).clamp(1, 12) - 1][..3])
}

fn start_text(l: &Launcher) -> String {
    let where_ = if l.state.choice.entry < 0 {
        tr("Automatic")
    } else {
        l.state.map().and_then(|m| m.entry_points.get(l.state.choice.entry as usize)).map(|e| if e.name.is_empty() { format!("entry {}", e.index + 1) } else { e.name.clone() }).unwrap_or_else(|| tr("Automatic"))
    };
    format!("{where_} · {:.0} {}", l.state.choice.traffic, tr("cars"))
}

pub(in crate::launcher) fn handle(l: &mut Launcher, msg: Msg) {
    match msg {
        Msg::Tab(t) => {
            l.phone.tab = t;
            l.phone.page = None;
            l.phone.sheet = None;
            match t {
                Tab::Mods => l.go(Page::Mods),
                Tab::Play => l.go(Page::Drive),
                Tab::Online => l.go(Page::Multiplayer),
                Tab::More => {}
            }
            // (go() set the tab by the page: the one tapped wins)
            l.phone.tab = t;
            if t == Tab::More {
                l.phone.page = None;
            }
        }
        Msg::Back => {
            if l.phone.sheet.is_some() {
                l.phone.sheet = None;
                l.phone.filter.clear();
            } else {
                l.phone.page = None;
            }
        }
        Msg::Open(s) => {
            if s == Sheet::Time && l.state.joined_server.is_some() {
                l.state.set_status("On a server its clock and weather are the server's", false);
                return;
            }
            if s == Sheet::Roadbook {
                l.state.load_ibis();
            }
            if let Some(step) = drive_step(s) {
                l.drive.tab = step;
                l.page = Page::Drive;
            }
            l.phone.sheet = Some(s);
            l.phone.filter.clear();
        }
        Msg::Search(s) => l.phone.filter = s,
        Msg::Map(f) => {
            l.state.select_map(&f);
            close(l, None);
        }
        Msg::Bus(f) => {
            l.state.select_bus(&f);
            // a bus with liveries: the livery next
            let next = l.state.bus().is_some_and(|b| !b.paints.is_empty()).then_some(Sheet::Livery);
            close(l, next);
        }
        Msg::Paint(p) => {
            l.state.choice.paint = p;
            l.state.touched();
            close(l, None);
        }
        Msg::Free => {
            l.state.choice.free = true;
            l.state.touched();
            close(l, None);
        }
        Msg::Line(n) => {
            l.state.choice.free = false;
            if l.state.choice.line.as_deref() != Some(n.as_str()) {
                l.state.choice.line = Some(n);
                l.state.choice.tour = None;
            }
            l.state.touched();
            // a line chosen: its tours next
            close(l, Some(Sheet::Tour));
        }
        Msg::Tour(num) => {
            let other_day = l.state.line().and_then(|x| x.tours.iter().find(|t| t.number == num)).filter(|t| !t.runs).and_then(|t| t.next_run.clone());
            // a tour of another day moves the date to the next day it runs
            if let Some(n) = other_day {
                l.state.choice.date = n;
                l.state.load_lines();
            }
            l.state.choice.tour = Some(num);
            l.state.touched();
            close(l, None);
        }
        Msg::More(p) => {
            l.go(p);
            l.phone.tab = Tab::More;
            l.phone.page = Some(p);
        }
        Msg::Start => crate::launcher::drive::start_from_phone(l),
        Msg::Continue => l.state.launch_last_situation(),
        Msg::Leave => l.state.leave_server(),
    }
}

fn close(l: &mut Launcher, next: Option<Sheet>) {
    l.phone.sheet = next;
    l.phone.filter.clear();
}
