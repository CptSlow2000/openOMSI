//! The Mods page: installing a mod (a folder or an archive, chosen or dropped on the window),
//! the installs under way and done, and what the content folder holds.

use super::kit::{self, len, lp, tr};
use super::theme::{ACCENT, DANGER, OK, WARN};
use super::Msg as Top;
use crate::launcher::state::fmt_bytes;
use crate::launcher::Launcher;
use egui_retained::widgets::{Button, Icon, Progress, Text};
use egui_retained::{Color32, Element, NodeId, PaintCx, Pos2, ScrollAxes, Ui, Vec2, Visual, taffy};
use omsi_launcher_lib as core;

#[derive(Clone, Debug)]
pub(in crate::launcher) enum Msg {
    Folder,
    Archive,
    Mode(usize),
    ClearFinished,
    Cancel(u64),
}

fn m(x: Msg) -> Top {
    Top::Mods(x)
}

/// Where a mod can be dropped: a dashed box that lights up while a file is held over the window.
pub struct DropZone {
    pub hot: bool,
}

impl Element for DropZone {
    fn measure(&mut self, _cx: &mut egui_retained::MeasureCx<'_>, _k: [Option<f32>; 2], _a: [Option<f32>; 2]) -> Vec2 {
        Vec2::new(200.0, 110.0)
    }
    fn paint(&mut self, cx: &mut PaintCx<'_>) {
        let r = cx.rect;
        let a = if self.hot { 0.9 } else { 0.4 };
        cx.painter.rect_filled(r, 12.0, ACCENT.gamma_multiply(if self.hot { 0.16 } else { 0.05 }));
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
        cx.icon("upload", Pos2::new(r.center().x, r.top() + 38.0), 28.0, ACCENT.gamma_multiply(a + 0.1));
        let g = cx.layout_text(&tr("…or drop a mod folder or .zip, .7z or .rar onto this window"), Some(r.width() - 24.0));
        let c = cx.look.color;
        cx.painter.galley(Pos2::new(r.center().x - g.size().x * 0.5, r.top() + 62.0), g, c);
    }
    fn hit_test(&self) -> bool {
        false
    }
}

pub(in crate::launcher) struct ModsPage {
    pub root: NodeId,
    modes: Vec<NodeId>,
    drop: NodeId,
    path: NodeId,
    info: NodeId,
    inbox: NodeId,
    jobs: NodeId,
    jobs_key: String,
    folder: NodeId,
    folder_key: String,
}

impl ModsPage {
    pub fn build(ui: &mut Ui, host: NodeId) -> ModsPage {
        let root = ui.column(host);
        ui.add_class(root, "content");
        kit::grow(ui, root);
        kit::page_title(ui, root, "Mods", "A bus, a map, scenery, a whole OMSI folder - as a folder or a .zip, .7z or .rar. The original OMSI 2 folder is never written to.");
        let cols = ui.row(root);
        kit::grow(ui, cols);
        kit::gap(ui, cols, 14.0);
        ui.style(cols, |s| s.align_items = Some(taffy::AlignItems::Stretch));
        let col = |ui: &mut Ui, title: &str| {
            let c = kit::titled_card(ui, cols, title);
            kit::grow(ui, c);
            ui.set_scroll(c, ScrollAxes { x: false, y: true });
            c
        };
        // install
        let c0 = col(ui, "Install a mod");
        let r = ui.row(c0);
        kit::gap(ui, r, 8.0);
        let f = ui.add(r, Button::new(tr("Choose a folder")).icon("folder_open").class("primary"));
        kit::grow(ui, f);
        ui.on_click(f, m(Msg::Folder));
        let a = ui.add(r, Button::new(tr("Choose archive")).icon("inventory_2"));
        kit::grow(ui, a);
        ui.on_click(a, m(Msg::Archive));
        kit::text(ui, c0, &tr("Archive install mode"), "dim");
        let mr = ui.row(c0);
        kit::gap(ui, mr, 4.0);
        let modes = ["Auto", "Unpacked", "Used in place"]
            .iter()
            .enumerate()
            .map(|(k, l)| {
                let b = ui.add(mr, Button::new(tr(l)).class("tab"));
                kit::grow(ui, b);
                ui.on_click(b, m(Msg::Mode(k)));
                b
            })
            .collect();
        let drop = ui.add(c0, DropZone { hot: false });
        ui.add_class(drop, "dim");
        let path = kit::para(ui, c0, "", "faint");
        let info = kit::para(ui, c0, "", "dim");
        let inbox = ui.column(c0);
        kit::gap(ui, inbox, 6.0);
        // installs
        let c1 = col(ui, "Installs");
        let clear = ui.add(c1, Button::new(tr("Clear finished")).class("ghost"));
        ui.style(clear, |s| s.align_self = Some(taffy::AlignSelf::FlexEnd));
        ui.on_click(clear, m(Msg::ClearFinished));
        let jobs = ui.column(c1);
        kit::gap(ui, jobs, 10.0);
        // the content folder
        let c2 = col(ui, "Content folder");
        let folder = ui.column(c2);
        kit::gap(ui, folder, 6.0);
        ModsPage { root, modes, drop, path, info, inbox, jobs, jobs_key: String::new(), folder, folder_key: String::new() }
    }

    pub fn sync(&mut self, ui: &mut Ui, l: &Launcher) {
        for (k, n) in self.modes.iter().enumerate() {
            ui.set_selected(*n, k == l.state.mod_mode);
        }
        let hot = l.pages.drop_hover;
        ui.update::<DropZone>(self.drop, |d| std::mem::replace(&mut d.hot, hot) != hot);
        ui.set_text(self.path, &l.state.mod_path);
        ui.set_visible(self.path, !l.state.mod_path.is_empty());
        let (info, class) = match l.state.mod_info.clone() {
            Some(Ok(i)) if i.is_archive => {
                let fit = if i.fits { format!("fits ({} free)", fmt_bytes(i.free_bytes)) } else { format!("does not fit: needs {}, {} free", fmt_bytes(i.needed_bytes), fmt_bytes(i.free_bytes)) };
                let place = if i.in_place_ok { "can be used in place".to_string() } else { i.in_place.clone() };
                (format!("{} archive, {} files, {} unpacked - {fit}; {place}", fmt_bytes(i.archive_bytes), i.files, fmt_bytes(i.unpacked_bytes)), if i.fits { "dim" } else { "warn-text" })
            }
            Some(Err(e)) => (e, "danger-text"),
            _ => (String::new(), "dim"),
        };
        ui.set_text(self.info, &info);
        ui.set_visible(self.info, !info.is_empty());
        for c in ["dim", "warn-text", "danger-text"] {
            ui.set_class(self.info, c, c == class);
        }
        // the installs, built again when one moved on
        let jkey = l.state.jobs.iter().map(|j| format!("{}:{}:{}:{}:{}", j.id, j.state, j.files_done, j.bytes_done, j.message.len())).collect::<Vec<_>>().join("|");
        if jkey != self.jobs_key || self.jobs_key.is_empty() {
            self.jobs_key = format!("{jkey}#");
            ui.clear(self.jobs);
            if l.state.jobs.is_empty() {
                kit::para(ui, self.jobs, &tr("Nothing installed since the launcher started. Big archives are checked against the free disk space before anything is unpacked; a cancelled or failed install leaves nothing behind."), "dim");
            }
            for j in &l.state.jobs {
                let running = j.finished.is_none();
                let card = ui.column(self.jobs);
                ui.style(card, |s| {
                    s.padding = taffy::Rect::length(12.0_f32);
                    s.gap = taffy::Size { width: lp(6.0), height: lp(6.0) };
                    s.flex_shrink = 0.0;
                });
                ui.visual(card, Visual::new().background(Color32::from_white_alpha(6)).radius(10.0_f32));
                let top = ui.row(card);
                kit::gap(ui, top, 8.0);
                let n = ui.add(top, Text::new(j.name.clone()));
                ui.add_class(n, "strong");
                kit::grow(ui, n);
                let badge = kit::text(ui, top, &j.state.to_uppercase(), "badge");
                let sc = match j.state.as_str() {
                    "done" => OK,
                    "failed" => DANGER,
                    "cancelled" => Color32::from_gray(120),
                    _ => ACCENT,
                };
                ui.visual(badge, Visual::new().background(sc.gamma_multiply(0.25)).color(sc));
                if running {
                    let frac = if j.bytes_total > 0 { j.bytes_done as f32 / j.bytes_total as f32 } else if j.files_total > 0 { j.files_done as f32 / j.files_total as f32 } else { 0.0 };
                    ui.add(card, Progress { value: Some(frac) });
                    kit::text(ui, card, &format!("{} / {} files · {} / {}", j.files_done, j.files_total, fmt_bytes(j.bytes_done), fmt_bytes(j.bytes_total)), "faint");
                }
                kit::para(ui, card, &j.message, if j.state == "failed" { "danger-text" } else { "dim" });
                for w in &j.warnings {
                    let r = ui.row(card);
                    kit::gap(ui, r, 6.0);
                    let i = ui.add(r, Icon::new("warning"));
                    ui.visual(i, Visual::new().color(WARN));
                    kit::para(ui, r, w, "warn-text");
                }
                if running {
                    let c = ui.add(card, Button::new(tr("Cancel")).class("danger"));
                    ui.style(c, |s| s.align_self = Some(taffy::AlignSelf::FlexStart));
                    ui.on_click(c, m(Msg::Cancel(j.id)));
                }
            }
        }
        // the content folder and the Mods inbox
        let fkey = format!("{:?}", l.state.mods.as_ref().map(|x| (&x.content_dir, x.free_bytes / 1_000_000, &x.folders, &x.archives, &x.waiting, &x.inbox_items)));
        if fkey != self.folder_key {
            self.folder_key = fkey;
            ui.clear(self.folder);
            ui.clear(self.inbox);
            match l.state.mods.clone() {
                None => {
                    kit::text(ui, self.folder, &tr("Reading…"), "dim");
                }
                Some(md) => {
                    kit::text(ui, self.inbox, &tr("The Mods folder"), "heading");
                    kit::para(ui, self.inbox, &format!("Anything put into {} is installed by itself once it has finished copying.", md.inbox), "dim");
                    if !md.inbox_items.is_empty() {
                        kit::para(ui, self.inbox, &format!("In it now: {}", md.inbox_items.join(", ")), "");
                    }
                    kit::para(ui, self.folder, &md.content_dir, "strong");
                    kit::text(ui, self.folder, &format!("{} free on this disk", fmt_bytes(md.free_bytes)), "accent-text");
                    for (f, n) in &md.folders {
                        let r = ui.row(self.folder);
                        kit::gap(ui, r, 8.0);
                        ui.add(r, Icon::new("folder_open"));
                        let t = ui.add(r, Text::new(f.clone()));
                        kit::grow(ui, t);
                        kit::text(ui, r, &format!("{n} {}", if *n == 1 { "entry" } else { "entries" }), "dim");
                    }
                    if !md.archives.is_empty() {
                        kit::text(ui, self.folder, &tr("Archives used in place"), "heading");
                        for (n, b) in &md.archives {
                            kit::text(ui, self.folder, &format!("{n}  ({})", fmt_bytes(*b)), "dim");
                        }
                    }
                    if !md.waiting.is_empty() {
                        kit::text(ui, self.folder, &tr("Waiting for their bus"), "heading");
                        for w in &md.waiting {
                            kit::text(ui, self.folder, w, "dim");
                        }
                    }
                }
            }
        }
        let _ = len;
    }
}

pub(in crate::launcher) fn handle(l: &mut Launcher, msg: Msg) {
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
    }
}
