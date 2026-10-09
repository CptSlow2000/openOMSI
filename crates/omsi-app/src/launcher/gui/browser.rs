//! A phone's storage browser over the page (see `launcher::mobile::Browser`): the places,
//! the folder it is in and what lies there, and what to do with the folder - use it as the
//! OMSI 2 folder, install it as a mod, or tap an archive to install it.

use super::kit::{self, len, lp, tr};
use super::theme::ACCENT;
use super::Msg as Top;
use crate::launcher::mobile::{storage_roots, Purpose};
use crate::launcher::Launcher;
use egui_retained::widgets::{Button, Icon};
use egui_retained::{Color32, Layer, NodeId, ScrollAxes, Ui, Visual, taffy};
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub(in crate::launcher) enum Msg {
    Up,
    Go(PathBuf),
    Entry(String, bool),
    Use,
    Cancel,
}

fn m(x: Msg) -> Top {
    Top::Browser(x)
}

pub(in crate::launcher) struct BrowserView {
    root: NodeId,
    key: u64,
}

impl BrowserView {
    pub fn build(ui: &mut Ui) -> BrowserView {
        let root = ui.column(ui.root(Layer::Overlay));
        ui.style(root, |s| {
            s.position = taffy::Position::Absolute;
            s.inset = taffy::Rect { left: kit::lpa(0.0), right: kit::lpa(0.0), top: kit::lpa(0.0), bottom: kit::lpa(0.0) };
            s.padding = taffy::Rect::length(14.0_f32);
        });
        ui.visual(root, Visual::new().background(Color32::from_black_alpha(184)));
        ui.set_visible(root, false);
        BrowserView { root, key: 0 }
    }

    pub fn sync(&mut self, ui: &mut Ui, l: &Launcher) {
        let Some(b) = l.browser.as_ref() else {
            ui.set_visible(self.root, false);
            self.key = 0;
            return;
        };
        ui.set_visible(self.root, true);
        let key = {
            let mut h = std::collections::hash_map::DefaultHasher::new();
            (&b.dir, b.purpose as u8, b.entries.len(), &b.error, b.is_root).hash(&mut h);
            h.finish() | 1
        };
        if key == self.key {
            return;
        }
        self.key = key;
        ui.clear(self.root);
        let c = ui.column(self.root);
        ui.add_class(c, "dialog");
        kit::grow(ui, c);
        ui.style(c, |s| {
            s.padding = taffy::Rect::length(16.0_f32);
            s.gap = taffy::Size { width: lp(10.0), height: lp(10.0) };
            s.min_size.height = len(0.0);
        });
        let head = ui.row(c);
        let t = kit::text(ui, head, &tr(b.title()), "strong");
        ui.visual(t, Visual::new().font_size(17.0_f32));
        kit::grow(ui, t);
        let x = ui.add(head, Button::new(tr("Cancel")).icon("close").class("ghost"));
        ui.on_click(x, m(Msg::Cancel));
        // the places
        let places = ui.row(c);
        ui.style(places, |s| {
            s.flex_wrap = taffy::FlexWrap::Wrap;
            s.gap = taffy::Size { width: lp(8.0), height: lp(8.0) };
        });
        let up = ui.add(places, Button::new("").icon("drive_folder_upload"));
        ui.set_tooltip(up, Some(tr("Up")));
        ui.on_click(up, m(Msg::Up));
        for (name, path) in storage_roots() {
            let b = ui.add(places, Button::new(name).icon("sd_card"));
            ui.on_click(b, m(Msg::Go(path)));
        }
        let pr = ui.row(c);
        kit::gap(ui, pr, 8.0);
        let ic = ui.add(pr, Icon::new("folder_open"));
        ui.add_class(ic, "dim");
        kit::text(ui, pr, &b.dir.to_string_lossy(), "");
        // what lies there
        let list = ui.column(c);
        kit::grow(ui, list);
        ui.set_scroll(list, ScrollAxes { x: false, y: true });
        ui.add_class(list, "stage");
        ui.style(list, |s| {
            s.min_size.height = len(0.0);
            s.padding = taffy::Rect::length(4.0_f32);
            s.gap = taffy::Size { width: lp(2.0), height: lp(2.0) };
        });
        if let Some(e) = &b.error {
            kit::para(ui, list, &tr(e), "danger-text");
        } else if b.entries.is_empty() {
            kit::para(ui, list, &tr("Nothing here"), "faint");
        }
        for (name, dir, bytes) in &b.entries {
            let r = ui.row(list);
            ui.add_class(r, "list-row");
            kit::gap(ui, r, 10.0);
            ui.style(r, |s| s.min_size.height = len(40.0));
            ui.on_click(r, m(Msg::Entry(name.clone(), *dir)));
            let i = ui.add(r, Icon::new(if *dir { "folder" } else { "inventory_2" }));
            ui.visual(i, Visual::new().color(if *dir { ACCENT } else { Color32::from_gray(190) }));
            let t = kit::text(ui, r, name, "strong");
            kit::grow(ui, t);
            if !*dir {
                kit::text(ui, r, &crate::launcher::state::fmt_bytes(*bytes), "dim");
            }
        }
        // what to do with this folder
        let foot = ui.row(c);
        kit::gap(ui, foot, 10.0);
        match b.purpose {
            Purpose::Root => {
                let t = kit::para(ui, foot, &tr(if b.is_root { "A complete OMSI 2 installation" } else { "Not an OMSI 2 folder (it needs Omsi.exe, maps and Vehicles)" }), if b.is_root { "ok-text" } else { "dim" });
                kit::grow(ui, t);
                let u = ui.add(foot, Button::new(tr("Use this folder")).icon("check").class(if b.is_root { "primary" } else { "button" }));
                ui.on_click(u, m(Msg::Use));
            }
            Purpose::ModFolder => {
                ui.spacer(foot);
                let u = ui.add(foot, Button::new(tr("Install this folder")).icon("download").class("primary"));
                ui.on_click(u, m(Msg::Use));
            }
            Purpose::ModZip => {
                kit::para(ui, foot, &tr("Tap a .zip, .7z or .rar to install it"), "dim");
            }
        }
    }
}

pub(in crate::launcher) fn handle(l: &mut Launcher, msg: Msg) {
    let Some(mut b) = l.browser.take() else { return };
    let mut chosen = None;
    let mut close = false;
    match msg {
        Msg::Up => {
            if let Some(p) = b.dir.parent().map(|p| p.to_path_buf()) {
                b.open(p);
            }
        }
        Msg::Go(p) => b.open(p),
        Msg::Entry(name, dir) => {
            let p = b.dir.join(&name);
            if dir {
                b.open(p);
            } else {
                chosen = Some(p);
            }
        }
        Msg::Use => chosen = Some(b.dir.clone()),
        Msg::Cancel => close = true,
    }
    if let Some(p) = chosen {
        l.browser_chose(b.purpose, &p);
        close = true;
    }
    if !close {
        l.browser = Some(b);
    }
}
