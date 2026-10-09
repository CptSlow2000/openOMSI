//! The Setup page: where the original OMSI 2 and this game are, and the support package for a
//! bug report.

use super::kit::{self, len, tr};
use super::Msg as Top;
use crate::launcher::Launcher;
use egui_retained::widgets::{Button, TextInput};
use egui_retained::{NodeId, ScrollAxes, Ui, taffy};
use omsi_launcher_lib as core;

#[derive(Clone, Debug)]
pub(in crate::launcher) enum Msg {
    Root(String),
    Game(String),
    BrowseRoot,
    BrowseGame,
    Save,
    Diagnostics,
}

fn m(x: Msg) -> Top {
    Top::Setup(x)
}

const ABOUT: &str = "A ZIP for a bug report: the program and system versions, the graphics card, its driver and the graphics settings, the controllers, and the map and bus of the last game. No folders, names, chat, LAN codes or addresses; of the logs only which events happened. It is saved on this computer and its folder opened, so you can look inside before you attach it to an issue.";

pub(in crate::launcher) struct SetupPage {
    pub root: NodeId,
    root_field: NodeId,
    game_row: NodeId,
    game_field: NodeId,
    shown: (String, String),
    edited: (Option<String>, Option<String>),
}

impl SetupPage {
    pub fn build(ui: &mut Ui, host: NodeId) -> SetupPage {
        let root = ui.column(host);
        ui.add_class(root, "content");
        kit::grow(ui, root);
        ui.set_scroll(root, ScrollAxes { x: false, y: true });
        kit::page_title(ui, root, "Setup", "Where the original game and this one are.");
        let c = kit::titled_card(ui, root, "Folders");
        ui.style(c, |s| s.max_size.width = len(860.0));
        let r = kit::labelled(ui, c, "OMSI 2 folder", 150.0);
        let root_field = ui.add(r, TextInput::new("").hint("/path/to/OMSI 2").on_change(|s| m(Msg::Root(s))));
        kit::grow(ui, root_field);
        let b = ui.add(r, Button::new(tr("Browse")));
        ui.on_click(b, m(Msg::BrowseRoot));
        ui.set_name(b, "setup-browse");
        let game_row = kit::labelled(ui, c, "Game binary", 150.0);
        let game_field = ui.add(game_row, TextInput::new("").hint("openomsi").on_change(|s| m(Msg::Game(s))));
        kit::grow(ui, game_field);
        let b = ui.add(game_row, Button::new(tr("Browse")));
        ui.on_click(b, m(Msg::BrowseGame));
        let text = if core::IN_PROCESS_GAMES {
            "Copy the whole OMSI 2 folder (with maps and Vehicles in it) onto the phone - by cable, from a PC or a USB stick - for example as openOMSI/OMSI 2 in the internal storage, then choose it here with Browse. Mods go into openOMSI/Mods or are installed from the Mods page."
        } else {
            "The OMSI 2 folder is the one with maps and Vehicles in it (any complete installation). The game binary is the openomsi program; it is found by itself when it sits next to the launcher."
        };
        kit::para(ui, c, &tr(text), "dim");
        let save = ui.add(c, Button::new(tr("Save")).icon("save").class("primary"));
        ui.style(save, |s| s.align_self = Some(taffy::AlignSelf::FlexStart));
        ui.on_click(save, m(Msg::Save));
        let s = kit::titled_card(ui, root, "Support package");
        ui.style(s, |st| st.max_size.width = len(860.0));
        kit::para(ui, s, &tr(ABOUT), "dim");
        let d = ui.add(s, Button::new(tr("Export diagnostics")).icon("download"));
        ui.style(d, |st| st.align_self = Some(taffy::AlignSelf::FlexStart));
        ui.on_click(d, m(Msg::Diagnostics));
        SetupPage { root, root_field, game_row, game_field, shown: (String::new(), String::new()), edited: (None, None) }
    }

    pub fn sync(&mut self, ui: &mut Ui, l: &Launcher) {
        // (a folder the storage browser chose)
        if let Some(r) = l.pages.setup_root.as_ref().filter(|r| self.edited.0.as_ref() != Some(*r)) {
            self.edited.0 = Some(r.clone());
        }
        let root = self.edited.0.clone().unwrap_or_else(|| l.state.config.root.clone());
        let game = self.edited.1.clone().unwrap_or_else(|| l.state.config.game.clone());
        if self.shown.0 != root && ui.focused() != Some(self.root_field) {
            ui.with::<TextInput, _>(self.root_field, |t| t.set(&root));
            self.shown.0 = root;
        }
        if self.shown.1 != game && ui.focused() != Some(self.game_field) {
            ui.with::<TextInput, _>(self.game_field, |t| t.set(&game));
            self.shown.1 = game;
        }
        ui.set_visible(self.game_row, !core::IN_PROCESS_GAMES);
    }
}

pub(in crate::launcher) fn handle(l: &mut Launcher, msg: Msg) {
    fn page(l: &mut Launcher) -> Option<&mut SetupPage> {
        l.gui.as_mut().and_then(|g| g.setup.as_mut())
    }
    match msg {
        Msg::Root(s) => {
            l.pages.setup_root = None;
            if let Some(p) = page(l) {
                p.edited.0 = Some(s);
            }
        }
        Msg::Game(s) => {
            if let Some(p) = page(l) {
                p.edited.1 = Some(s);
            }
        }
        Msg::BrowseRoot => {
            if crate::launcher::mobile::mobile() {
                let start = page(l).and_then(|p| p.edited.0.clone()).unwrap_or_else(|| l.state.config.root.clone());
                l.browse(crate::launcher::mobile::Purpose::Root, &start);
            } else if let Some(p) = core::pick_folder("The OMSI 2 folder (with maps and Vehicles in it)") {
                if let Some(pg) = page(l) {
                    pg.edited.0 = Some(p.to_string_lossy().to_string());
                    pg.shown.0.clear();
                }
            }
        }
        Msg::BrowseGame => {
            if let Some(p) = core::pick_file("The openomsi program") {
                if let Some(pg) = page(l) {
                    pg.edited.1 = Some(p.to_string_lossy().to_string());
                    pg.shown.1.clear();
                }
            }
        }
        Msg::Save => {
            let (root, game) = page(l).map(|p| p.edited.clone()).unwrap_or_default();
            let chosen = root.unwrap_or_else(|| l.state.config.root.clone()).trim().to_string();
            let game = game.unwrap_or_else(|| l.state.config.game.clone());
            // (a folder that is no complete installation is said so, with what it lacks, and
            // nothing is saved, #1656)
            let missing = if chosen.is_empty() { Vec::new() } else { omsi_cfg::missing_original_essentials(std::path::Path::new(&chosen)) };
            if !missing.is_empty() {
                let shown: Vec<&str> = missing.iter().map(String::as_str).take(6).collect();
                let more = if missing.len() > shown.len() { format!(" and {} more", missing.len() - shown.len()) } else { String::new() };
                l.state.set_status(format!("Not saved: {chosen} is not a complete OMSI 2 installation - it lacks {}{more}", shown.join(", ")), true);
                return;
            }
            l.state.config.root = chosen.clone();
            l.state.config.game = game.trim().to_string();
            match core::save_config(&l.state.config) {
                Ok(()) => {
                    omsi_cfg::content_changed();
                    l.state.config = core::load_config();
                    if let Some(p) = page(l) {
                        p.edited = (None, None);
                        p.shown = (String::new(), String::new());
                    }
                    l.pages.setup_root = None;
                    let same = chosen.is_empty() || std::path::Path::new(&chosen) == std::path::Path::new(&l.state.config.root);
                    if same {
                        l.state.set_status("Saved. Reading the content again…", false);
                    } else {
                        l.state.set_status(format!("Saved, but the OMSI 2 folder used is {}", l.state.config.root), true);
                    }
                    #[cfg(target_os = "android")]
                    crate::android::hide_content_from_gallery();
                    l.state.load_content();
                }
                Err(e) => l.state.set_status(format!("{e:#}"), true),
            }
        }
        Msg::Diagnostics => {
            let c = &l.state.choice;
            let snapshot = crate::support_bundle::snapshot(l.renderer.as_ref(), &crate::settings::Settings::load(), l.pages.pads.io.as_ref().map(|d| d.connected()), Some((&c.map, Some(&c.bus), c.line.as_deref(), c.tour.as_deref())), "launcher_selection");
            let out = crate::support_bundle::default_output();
            l.state.spawn(move || crate::launcher::state::Msg::Diagnostics(crate::support_bundle::export(&out, &snapshot).map(|_| out).map_err(|e| format!("{e:#}"))));
        }
    }
}
