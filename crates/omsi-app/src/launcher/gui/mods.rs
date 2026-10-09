//! The Mods page: every mod of the content folder in one list - found by a search, sorted by
//! kind - each switched off and on, or deleted, by itself; installing one (an archive or a
//! folder, chosen or dropped on the window) with what the install does as it goes.

use super::kit::{self, len, lp, tr};
use super::theme::{ACCENT, DANGER, OK, WARN};
use super::Msg as Top;
use crate::launcher::state::fmt_bytes;
use crate::launcher::Launcher;
use egui_retained::widgets::{Button, Checkbox, Icon, Progress, Text, TextInput};
use egui_retained::{Color32, Element, Layer, NodeId, PaintCx, Pos2, ScrollAxes, Ui, Vec2, Visual, taffy};
use omsi_launcher_lib as core;
use omsi_launcher_lib::mods::{Kind, Mod};

#[derive(Clone, Debug)]
pub(in crate::launcher) enum Msg {
    Folder,
    Archive,
    Mode(usize),
    ClearFinished,
    Cancel(u64),
    Search(String),
    Filter(usize),
    Toggle(String, bool),
    AskRemove(String),
    Remove(String),
    KeepIt,
    OpenFolder,
}

fn m(x: Msg) -> Top {
    Top::Mods(x)
}

/// The filters over the list: everything, a kind, or what is switched off.
const FILTERS: [(&str, &str); 6] = [("All", "apps"), ("Buses", "directions_bus"), ("Maps", "map"), ("Archives", "inventory_2"), ("Other", "extension"), ("Switched off", "power_settings_new")];

fn passes(md: &Mod, filter: usize) -> bool {
    match filter {
        1 => md.kind == Kind::Bus,
        2 => md.kind == Kind::Map,
        3 => md.kind == Kind::Archive,
        4 => md.kind == Kind::Other,
        5 => !md.enabled,
        _ => true,
    }
}

fn icon_of(k: Kind) -> &'static str {
    match k {
        Kind::Bus => "directions_bus",
        Kind::Map => "map",
        Kind::Archive => "inventory_2",
        Kind::Other => "extension",
    }
}

/// Where a mod can be dropped: a dashed strip that lights up while a file is held over the
/// window.
pub struct DropZone {
    pub hot: bool,
}

impl Element for DropZone {
    fn measure(&mut self, _cx: &mut egui_retained::MeasureCx<'_>, _k: [Option<f32>; 2], _a: [Option<f32>; 2]) -> Vec2 {
        Vec2::new(200.0, 64.0)
    }
    fn paint(&mut self, cx: &mut PaintCx<'_>) {
        let r = cx.rect;
        let a = if self.hot { 0.9 } else { 0.4 };
        cx.painter.rect_filled(r, 10.0, ACCENT.gamma_multiply(if self.hot { 0.16 } else { 0.04 }));
        let per = 2.0 * (r.width() + r.height());
        let n = (per / 14.0) as usize;
        for k in 0..n {
            let s = k as f32 * per / n as f32;
            let p = if s < r.width() {
                Pos2::new(r.left() + s, r.top())
            } else if s < r.width() + r.height() {
                Pos2::new(r.right(), r.top() + s - r.width())
            } else if s < 2.0 * r.width() + r.height() {
                Pos2::new(r.right() - (s - r.width() - r.height()), r.bottom())
            } else {
                Pos2::new(r.left(), r.bottom() - (s - 2.0 * r.width() - r.height()))
            };
            cx.painter.circle_filled(p, 1.3, ACCENT.gamma_multiply(a));
        }
        let text = tr(if self.hot { "Let go to install it" } else { "Drop a mod folder or a .zip, .7z or .rar onto this window to install it" });
        let g = cx.layout_text(&text, Some(r.width() - 80.0));
        let w = g.size().x + 34.0;
        let x = r.center().x - w * 0.5;
        cx.icon("upload", Pos2::new(x + 11.0, r.center().y), 22.0, ACCENT.gamma_multiply(a + 0.1));
        let c = cx.look.color;
        cx.painter.galley(Pos2::new(x + 34.0, r.center().y - g.size().y * 0.5), g, c);
    }
    fn hit_test(&self) -> bool {
        false
    }
}

pub(in crate::launcher) struct ModsPage {
    pub root: NodeId,
    modes: Vec<NodeId>,
    drop: NodeId,
    info: NodeId,
    jobs_card: NodeId,
    jobs: NodeId,
    jobs_key: String,
    count: NodeId,
    filters: Vec<NodeId>,
    list: NodeId,
    list_key: u64,
    folder: NodeId,
    folder_key: String,
    search: String,
    filter: usize,
    /// The deletion asked about, over the page.
    confirm: NodeId,
    confirm_for: Option<String>,
}

impl ModsPage {
    pub fn build(ui: &mut Ui, host: NodeId) -> ModsPage {
        let root = ui.column(host);
        ui.add_class(root, "content");
        kit::grow(ui, root);
        ui.set_scroll(root, ScrollAxes { x: false, y: true });
        // the title, and the ways to install on its right
        let head = ui.row(root);
        kit::gap(ui, head, 10.0);
        ui.style(head, |s| {
            s.align_items = Some(taffy::AlignItems::FlexStart);
            s.flex_shrink = 0.0;
        });
        let titles = ui.column(head);
        kit::grow(ui, titles);
        kit::text(ui, titles, &tr("Mods"), "title");
        kit::para(ui, titles, &tr("Buses, maps, scenery - switch each off and on, or delete it. The original OMSI 2 folder is never written to."), "subtitle");
        let a = ui.add(head, Button::new(tr("Install an archive")).icon("inventory_2").class("primary"));
        ui.on_click(a, m(Msg::Archive));
        let f = ui.add(head, Button::new(tr("Install a folder")).icon("folder_open"));
        ui.on_click(f, m(Msg::Folder));
        // dropping, and how archives go in
        let drop = ui.add(root, DropZone { hot: false });
        ui.add_class(drop, "dim");
        ui.style(drop, |s| s.flex_shrink = 0.0);
        let opts = ui.row(root);
        kit::gap(ui, opts, 6.0);
        ui.style(opts, |s| {
            s.flex_wrap = taffy::FlexWrap::Wrap;
            s.flex_shrink = 0.0;
        });
        let l = kit::text(ui, opts, &tr("Archives are"), "dim");
        ui.style(l, |s| s.margin.right = taffy::LengthPercentageAuto::length(4.0));
        let modes = ["Unpacked when they fit", "Always unpacked", "Used in place (.zip)"]
            .iter()
            .enumerate()
            .map(|(k, label)| {
                let b = ui.add(opts, Button::new(tr(label)).class("tab"));
                ui.on_click(b, m(Msg::Mode(k)));
                b
            })
            .collect();
        let info = kit::para(ui, root, "", "dim");
        // the installs under way and just done
        let jobs_card = kit::card(ui, root);
        let jh = ui.row(jobs_card);
        kit::text(ui, jh, &tr("Installs"), "heading");
        ui.spacer(jh);
        let clear = ui.add(jh, Button::new(tr("Clear finished")).class("ghost"));
        ui.on_click(clear, m(Msg::ClearFinished));
        let jobs = ui.column(jobs_card);
        kit::gap(ui, jobs, 8.0);
        // the mods
        let card = kit::card(ui, root);
        ui.style(card, |s| s.gap = taffy::Size { width: lp(10.0), height: lp(10.0) });
        let lh = ui.row(card);
        kit::gap(ui, lh, 10.0);
        let count = kit::text(ui, lh, "", "heading");
        kit::grow(ui, count);
        let s = ui.add(lh, TextInput::new("").hint(tr("Search mods…")).on_change(|s| m(Msg::Search(s))));
        ui.style(s, |st| st.size.width = len(260.0));
        let fr = ui.row(card);
        kit::gap(ui, fr, 4.0);
        ui.style(fr, |s| s.flex_wrap = taffy::FlexWrap::Wrap);
        let filters = FILTERS
            .iter()
            .enumerate()
            .map(|(k, (t, i))| {
                let b = ui.add(fr, Button::new(tr(t)).icon(*i).class("tab"));
                ui.on_click(b, m(Msg::Filter(k)));
                b
            })
            .collect();
        let list = ui.column(card);
        kit::gap(ui, list, 4.0);
        // the content folder
        let folder = kit::card(ui, root);
        // the question before a deletion
        let confirm = ui.column(ui.root(Layer::Overlay));
        ui.style(confirm, |s| {
            s.position = taffy::Position::Absolute;
            s.inset = taffy::Rect { left: kit::lpa(0.0), right: kit::lpa(0.0), top: kit::lpa(0.0), bottom: kit::lpa(0.0) };
            s.align_items = Some(taffy::AlignItems::Center);
            s.justify_content = Some(taffy::JustifyContent::Center);
        });
        ui.visual(confirm, Visual::new().background(Color32::from_black_alpha(158)));
        ui.set_visible(confirm, false);
        ModsPage {
            root,
            modes,
            drop,
            info,
            jobs_card,
            jobs,
            jobs_key: String::new(),
            count,
            filters,
            list,
            list_key: 0,
            folder,
            folder_key: String::new(),
            search: String::new(),
            filter: 0,
            confirm,
            confirm_for: None,
        }
    }

    pub fn sync(&mut self, ui: &mut Ui, l: &Launcher) {
        for (k, n) in self.modes.iter().enumerate() {
            ui.set_selected(*n, k == l.state.mod_mode);
        }
        let hot = l.pages.drop_hover;
        ui.update::<DropZone>(self.drop, |d| std::mem::replace(&mut d.hot, hot) != hot);
        // what the archive being installed holds
        let (info, class) = match l.state.mod_info.clone() {
            Some(Ok(i)) if i.is_archive => {
                let fit = if i.fits { format!("fits ({} free)", fmt_bytes(i.free_bytes)) } else { format!("does not fit: needs {}, {} free", fmt_bytes(i.needed_bytes), fmt_bytes(i.free_bytes)) };
                let place = if i.in_place_ok { "can be used in place".to_string() } else { i.in_place.clone() };
                (format!("{}: {} archive, {} files, {} unpacked - {fit}; {place}", l.state.mod_path.rsplit(['/', '\\']).next().unwrap_or(""), fmt_bytes(i.archive_bytes), i.files, fmt_bytes(i.unpacked_bytes)), if i.fits { "dim" } else { "warn-text" })
            }
            Some(Err(e)) => (e, "danger-text"),
            _ => (String::new(), "dim"),
        };
        ui.set_text(self.info, &info);
        ui.set_visible(self.info, !info.is_empty());
        for c in ["dim", "warn-text", "danger-text"] {
            ui.set_class(self.info, c, c == class);
        }
        self.sync_jobs(ui, l);
        self.sync_list(ui, l);
        self.sync_folder(ui, l);
        self.sync_confirm(ui, l);
    }

    fn sync_jobs(&mut self, ui: &mut Ui, l: &Launcher) {
        ui.set_visible(self.jobs_card, !l.state.jobs.is_empty());
        let jkey = l.state.jobs.iter().map(|j| format!("{}:{}:{}:{}:{}", j.id, j.state, j.files_done, j.bytes_done, j.message.len())).collect::<Vec<_>>().join("|");
        if jkey == self.jobs_key {
            return;
        }
        self.jobs_key = jkey;
        ui.clear(self.jobs);
        for j in &l.state.jobs {
            let running = j.finished.is_none();
            let row = ui.column(self.jobs);
            ui.style(row, |s| {
                s.padding = taffy::Rect::length(10.0_f32);
                s.gap = taffy::Size { width: lp(6.0), height: lp(6.0) };
                s.flex_shrink = 0.0;
            });
            ui.visual(row, Visual::new().background(Color32::from_white_alpha(6)).radius(8.0_f32));
            let top = ui.row(row);
            kit::gap(ui, top, 8.0);
            let n = ui.add(top, Text::new(j.name.clone()));
            ui.add_class(n, "strong");
            kit::grow(ui, n);
            let (state, sc) = match j.state.as_str() {
                "done" => (tr("Installed"), OK),
                "failed" => (tr("Failed"), DANGER),
                "cancelled" => (tr("Cancelled"), Color32::from_gray(140)),
                other => (tr(other), ACCENT),
            };
            let badge = kit::text(ui, top, &state, "badge");
            ui.visual(badge, Visual::new().background(sc.gamma_multiply(0.22)).color(sc));
            if running {
                let c = ui.add(top, Button::new(tr("Cancel")).class("ghost"));
                ui.on_click(c, m(Msg::Cancel(j.id)));
                let frac = if j.bytes_total > 0 { j.bytes_done as f32 / j.bytes_total as f32 } else if j.files_total > 0 { j.files_done as f32 / j.files_total as f32 } else { 0.0 };
                ui.add(row, Progress { value: Some(frac) });
                kit::text(ui, row, &format!("{} / {} files · {} / {}", j.files_done, j.files_total, fmt_bytes(j.bytes_done), fmt_bytes(j.bytes_total)), "faint");
            }
            kit::para(ui, row, &j.message, if j.state == "failed" { "danger-text" } else { "dim" });
            for w in &j.warnings {
                let r = ui.row(row);
                kit::gap(ui, r, 6.0);
                let i = ui.add(r, Icon::new("warning"));
                ui.visual(i, Visual::new().color(WARN));
                let t = kit::para(ui, r, w, "warn-text");
                kit::grow(ui, t);
            }
        }
    }

    fn sync_list(&mut self, ui: &mut Ui, l: &Launcher) {
        for (k, n) in self.filters.iter().enumerate() {
            ui.set_selected(*n, k == self.filter);
        }
        let mods: Vec<Mod> = l.state.mods.as_ref().map(|x| x.installed.clone()).unwrap_or_default();
        let key = {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            (&self.search, self.filter, &l.state.mod_busy, l.state.mods.is_some()).hash(&mut h);
            for md in &mods {
                (&md.id, md.enabled, md.bytes).hash(&mut h);
            }
            h.finish() | 1
        };
        if key == self.list_key {
            return;
        }
        self.list_key = key;
        let on = mods.iter().filter(|x| x.enabled).count();
        ui.set_text(self.count, &match mods.len() {
            0 => tr("Installed mods"),
            n => format!("{} · {n} ({on} {})", tr("Installed mods"), tr("on")),
        });
        // the filters say how many each holds
        for (k, n) in self.filters.iter().enumerate() {
            let c = mods.iter().filter(|x| passes(x, k)).count();
            ui.set_text(*n, &format!("{}  {c}", tr(FILTERS[k].0)));
        }
        ui.clear(self.list);
        if l.state.mods.is_none() {
            kit::para(ui, self.list, &tr("Reading the content folder…"), "dim");
            return;
        }
        let q = self.search.to_lowercase();
        let mut shown: Vec<&Mod> = mods.iter().filter(|x| passes(x, self.filter)).filter(|x| q.is_empty() || x.name.to_lowercase().contains(&q) || x.paths.iter().any(|p| p.to_lowercase().contains(&q))).collect();
        // (switched-on first, then by name)
        shown.sort_by(|a, b| b.enabled.cmp(&a.enabled).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
        if shown.is_empty() {
            let e = ui.row(self.list);
            kit::gap(ui, e, 12.0);
            ui.style(e, |s| s.padding = taffy::Rect::length(14.0_f32));
            let i = ui.add(e, Icon::new("extension"));
            ui.visual(i, Visual::new().font_size(26.0_f32));
            ui.add_class(i, "faint");
            let t = kit::para(ui, e, &tr(if mods.is_empty() { "No mods yet. Install an archive or a folder above, or drop one onto the window." } else { "No mod matches." }), "dim");
            kit::grow(ui, t);
        }
        for md in shown {
            let busy = l.state.mod_busy.as_deref() == Some(md.id.as_str());
            let r = ui.row(self.list);
            kit::gap(ui, r, 12.0);
            ui.style(r, |s| {
                s.padding = taffy::Rect { left: lp(12.0), right: lp(8.0), top: lp(9.0), bottom: lp(9.0) };
                s.flex_shrink = 0.0;
            });
            ui.visual(r, Visual::new().background(Color32::from_white_alpha(if md.enabled { 6 } else { 2 })).radius(8.0_f32));
            let ic = ui.add(r, Icon::new(icon_of(md.kind)));
            ui.visual(ic, Visual::new().font_size(20.0_f32).color(if md.enabled { ACCENT } else { Color32::from_gray(110) }));
            let tc = ui.column(r);
            kit::grow(ui, tc);
            kit::gap(ui, tc, 2.0);
            let tl = ui.row(tc);
            kit::gap(ui, tl, 8.0);
            let n = kit::text(ui, tl, &md.name, if md.enabled { "strong" } else { "dim" });
            ui.style(n, |s| s.flex_shrink = 1.0);
            if !md.enabled {
                kit::text(ui, tl, &tr("OFF"), "badge");
            }
            let mut sub = vec![md.paths.join(", "), fmt_bytes(md.bytes)];
            if md.installed > 0 {
                sub.push(format!("{} {}", tr("installed"), crate::launcher::pages::chrono_like(md.installed)));
            } else if !md.noted {
                sub.push(tr("found in the content folder"));
            }
            kit::text(ui, tc, &sub.join(" · "), "faint");
            if busy {
                ui.add(r, egui_retained::widgets::Spinner);
                continue;
            }
            let id = md.id.clone();
            let sw = ui.add(r, Checkbox::switch(md.enabled, "").on_change(move |v| m(Msg::Toggle(id.clone(), v))));
            ui.set_name(sw, &format!("mod-switch-{}", md.name));
            ui.set_tooltip(sw, Some(tr(if md.enabled { "Switch it off: the game and the lists no longer see it, nothing is deleted" } else { "Switch it on again" })));
            let d = ui.add(r, Button::new("").icon("delete").class("ghost"));
            ui.set_tooltip(d, Some(tr("Delete this mod")));
            ui.set_name(d, &format!("mod-delete-{}", md.name));
            ui.on_click(d, m(Msg::AskRemove(md.id.clone())));
        }
    }

    fn sync_folder(&mut self, ui: &mut Ui, l: &Launcher) {
        let fkey = format!("{:?}", l.state.mods.as_ref().map(|x| (&x.content_dir, x.free_bytes / 1_000_000, &x.waiting, &x.inbox_items)));
        if fkey == self.folder_key {
            return;
        }
        self.folder_key = fkey;
        ui.clear(self.folder);
        ui.set_visible(self.folder, l.state.mods.is_some());
        let Some(md) = l.state.mods.clone() else { return };
        let h = ui.row(self.folder);
        kit::gap(ui, h, 10.0);
        let i = ui.add(h, Icon::new("folder_open"));
        ui.add_class(i, "dim");
        let tc = ui.column(h);
        kit::grow(ui, tc);
        kit::text(ui, tc, &tr("Content folder"), "heading");
        kit::text(ui, tc, &format!("{} · {} {}", md.content_dir, fmt_bytes(md.free_bytes), tr("free")), "dim");
        let o = ui.add(h, Button::new(tr("Open")).icon("open_in_new"));
        ui.on_click(o, m(Msg::OpenFolder));
        kit::para(ui, self.folder, &format!("{} {} {}", tr("Anything copied into"), md.inbox, tr("is installed by itself once it has finished copying.")), "faint");
        if !md.inbox_items.is_empty() {
            kit::para(ui, self.folder, &format!("{} {}", tr("In it now:"), md.inbox_items.join(", ")), "");
        }
        if !md.waiting.is_empty() {
            kit::para(ui, self.folder, &format!("{} {}", tr("Waiting for their bus:"), md.waiting.join(", ")), "warn-text");
        }
    }

    fn sync_confirm(&mut self, ui: &mut Ui, l: &Launcher) {
        let ask = self.confirm_for.clone();
        ui.set_visible(self.confirm, ask.is_some());
        let Some(id) = ask else { return };
        if !ui.children(self.confirm).is_empty() {
            return;
        }
        let Some(md) = l.state.mods.as_ref().and_then(|x| x.installed.iter().find(|x| x.id == id)).cloned() else { return };
        let c = ui.column(self.confirm);
        ui.add_class(c, "dialog");
        ui.style(c, |s| {
            s.size.width = len(520.0);
            s.max_size.width = taffy::Dimension::percent(0.94);
            s.padding = taffy::Rect { left: lp(24.0), right: lp(24.0), top: lp(20.0), bottom: lp(20.0) };
            s.gap = taffy::Size { width: lp(12.0), height: lp(12.0) };
        });
        let h = ui.row(c);
        kit::gap(ui, h, 12.0);
        let i = ui.add(h, Icon::new("delete"));
        ui.visual(i, Visual::new().font_size(26.0_f32).color(DANGER));
        let t = kit::text(ui, h, &format!("{} {}?", tr("Delete"), md.name), "strong");
        ui.visual(t, Visual::new().font_size(18.0_f32));
        kit::grow(ui, t);
        kit::para(ui, c, &format!("{} {} ({}). {}", tr("These are deleted from the content folder:"), md.paths.join(", "), fmt_bytes(md.bytes), tr("This cannot be undone - switch it off instead to keep it.")), "dim");
        let b = ui.row(c);
        kit::gap(ui, b, 10.0);
        ui.spacer(b);
        let k = ui.add(b, Button::new(tr("Cancel")));
        ui.on_click(k, m(Msg::KeepIt));
        let d = ui.add(b, Button::new(tr("Delete")).icon("delete").class("danger"));
        ui.set_name(d, "mods-delete-yes");
        ui.on_click(d, m(Msg::Remove(id)));
    }
}

impl Launcher {
    /// The Mods page asks for the mods again once an install has finished.
    pub(super) fn gui_mods_tick(&mut self) {
        if self.page != crate::launcher::Page::Mods {
            return;
        }
        // (asked for once the page is shown, and again when an install has finished)
        if self.state.mods.is_none() && !self.state.mods_asked {
            self.state.load_mods();
        }
        let done = self.state.jobs.iter().filter(|j| j.finished.is_some()).count();
        if done != self.state.mods_jobs_seen {
            self.state.mods_jobs_seen = done;
            self.state.load_mods();
        }
    }
}

pub(in crate::launcher) fn handle(l: &mut Launcher, msg: Msg) {
    fn page(l: &mut Launcher) -> Option<&mut ModsPage> {
        l.gui.as_mut().and_then(|g| g.mods.as_mut())
    }
    match msg {
        Msg::Folder => {
            if crate::launcher::mobile::mobile() {
                l.browse(crate::launcher::mobile::Purpose::ModFolder, "");
            } else if let Some(p) = core::pick_mod(false) {
                l.state.install(p.to_string_lossy().to_string());
            }
        }
        Msg::Archive => {
            if crate::launcher::mobile::mobile() {
                l.browse(crate::launcher::mobile::Purpose::ModZip, "");
            } else if let Some(p) = core::pick_mod(true) {
                l.state.install(p.to_string_lossy().to_string());
            }
        }
        Msg::Mode(k) => l.state.mod_mode = k,
        Msg::ClearFinished => {
            core::install::clear_finished();
            l.state.poll_now();
        }
        Msg::Cancel(id) => {
            core::install::cancel(id);
            l.state.poll_now();
        }
        Msg::Search(s) => {
            if let Some(p) = page(l) {
                p.search = s;
            }
        }
        Msg::Filter(k) => {
            if let Some(p) = page(l) {
                p.filter = k;
            }
        }
        Msg::Toggle(id, on) => l.state.mod_toggle(id, on),
        Msg::AskRemove(id) => {
            if let Some(g) = l.gui.as_mut() {
                if let Some(p) = g.mods.as_mut() {
                    p.confirm_for = Some(id);
                    g.ui.clear(p.confirm);
                }
            }
        }
        Msg::KeepIt | Msg::Remove(_) => {
            if let Some(g) = l.gui.as_mut() {
                if let Some(p) = g.mods.as_mut() {
                    p.confirm_for = None;
                    g.ui.clear(p.confirm);
                }
            }
            if let Msg::Remove(id) = msg {
                l.state.mod_remove(id);
            }
        }
        Msg::OpenFolder => {
            if let Some(d) = l.state.mods.as_ref().map(|x| x.content_dir.clone()) {
                crate::updater::open_url(&d);
            }
        }
    }
}
