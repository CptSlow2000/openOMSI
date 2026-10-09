//! The Controls page: the keyboard's bindings (two lists, a key given by pressing it, the
//! picker of every action a key can be given), and the game controllers - the devices of
//! `gamectrler.cfg`, what their axes and buttons do, the set-up assistant and the force
//! feedback's direction test. What the data is and how it is saved lives with the previous
//! interface (`pages.rs`, `keybind_picker.rs`); this is how it is shown and changed.

use super::kit::{self, len, lp, tr};
use super::theme::{ACCENT, DANGER, OK};
use super::Msg as Top;
use crate::controllers::{self, Connected, DeviceCfg, Func};
use crate::launcher::pages::keybind_picker::{bus_usage_label, controller_action_choices, ensure_key_action_catalog, filter_action_options, normalize_source_query, source_suggestions};
use crate::launcher::pages::{self, Wizard, WIZARD_STEPS};
use crate::launcher::Launcher;
use egui_retained::widgets::{Button, Checkbox, Icon, Select, Slider, Text, TextInput};
use egui_retained::{Color32, Element, Layer, MeasureCx, NodeId, PaintCx, ScrollAxes, Ui, Vec2, Visual, taffy};
use serde_json::{Value, json};
use std::hash::{Hash, Hasher};

const SECTIONS: [(&str, &str, &str, &str); 2] = [("Driving & the bus", "The bus's own keys", "vehicles", "directions_bus"), ("The game", "Menus, views, pausing", "game", "sports_esports")];
const AXES: [&str; 8] = ["X axis", "Y axis", "Z axis", "X rotation", "Y rotation", "Z rotation", "Slider 1", "Slider 2"];
/// The picker shows this many actions at most (a big installation has thousands; the search
/// finds the rest).
const PICKER_ROWS: usize = 150;

#[derive(Clone, Debug)]
pub(in crate::launcher) enum Msg {
    Tab(usize),
    UseCustom,
    Filter(usize, String),
    OpenPicker(usize),
    AddCustom(usize),
    Capture(usize, usize),
    More(usize, usize),
    Clear(usize, usize),
    Remove(usize, usize),
    PickerQuery(String),
    PickerSource(String),
    PickerSuggestion(String),
    Pick(String),
    Sources(String),
    SourcesBack,
    PickerCancel,
    PadSelect(usize),
    PadAdd(String),
    Deadzone(f32),
    PadSave,
    PadOn(bool),
    PadWizard,
    Axis(usize, usize),
    AxisReversed(usize, bool),
    AxisShape(usize, usize),
    ButtonAction(usize, usize),
    Latching(usize, bool),
    FfSteering(f32),
    FfVibration(f32),
    FfInvert(bool),
    AddButton,
    RemoveDevice,
    WizCancel,
    WizNext,
    WizSkip,
    FfStrength(f32),
    FfTest,
    FfManual(bool),
    FfFinish,
}

fn m(x: Msg) -> Top {
    Top::Controls(x)
}

/// An axis as it stands: a track and a mark at the value (-1 to 1).
pub struct AxisBar {
    pub value: Option<f32>,
}

impl Element for AxisBar {
    fn measure(&mut self, _cx: &mut MeasureCx<'_>, known: [Option<f32>; 2], _a: [Option<f32>; 2]) -> Vec2 {
        Vec2::new(known[0].unwrap_or(80.0), 16.0)
    }
    fn layout(&self, s: &mut taffy::Style) {
        s.flex_grow = 1.0;
        s.min_size.width = len(30.0);
    }
    fn paint(&mut self, cx: &mut PaintCx<'_>) {
        let r = cx.content;
        let track = egui_retained::Rect::from_center_size(r.center(), Vec2::new(r.width(), 8.0));
        cx.painter.rect_filled(track, 4.0, Color32::from_white_alpha(16));
        if let Some(v) = self.value {
            let x = track.left() + (v.clamp(-1.0, 1.0) + 1.0) * 0.5 * track.width();
            cx.painter.rect_filled(egui_retained::Rect::from_center_size(egui_retained::pos2(x, track.center().y), Vec2::new(4.0, 16.0)), 2.0, ACCENT);
        }
    }
    fn hit_test(&self) -> bool {
        false
    }
}

pub(in crate::launcher) struct ControlsPage {
    pub root: NodeId,
    subtitle: NodeId,
    tabs: [NodeId; 2],
    keys: NodeId,
    pads: NodeId,
    // the keyboard
    banner: NodeId,
    banner_text: NodeId,
    filters: [NodeId; 2],
    shown_filter: [String; 2],
    lists: [NodeId; 2],
    list_keys: [u64; 2],
    error: NodeId,
    // the picker over the page
    picker: NodeId,
    picker_key: u64,
    picker_query: NodeId,
    picker_source: NodeId,
    shown_source: String,
    picker_status: NodeId,
    picker_list: NodeId,
    picker_suggestions: NodeId,
    // the game controllers
    pads_key: u64,
    /// What the devices do now (polled each frame on this tab).
    pub(super) connected: Vec<Connected>,
    live: Vec<(usize, NodeId)>,
    button_rows: Vec<NodeId>,
    reveal: Option<usize>,
    ff_status: NodeId,
}

fn section_key(sec: usize) -> &'static str {
    SECTIONS[sec].2
}

impl ControlsPage {
    pub fn build(ui: &mut Ui, host: NodeId) -> ControlsPage {
        let root = ui.column(host);
        ui.add_class(root, "content");
        kit::grow(ui, root);
        let head = ui.row(root);
        ui.style(head, |s| s.align_items = Some(taffy::AlignItems::FlexStart));
        let titles = ui.column(head);
        kit::grow(ui, titles);
        let t = kit::text(ui, titles, &tr("Controls"), "title");
        let _ = t;
        let subtitle = kit::para(ui, titles, "", "subtitle");
        let seg = ui.row(head);
        kit::gap(ui, seg, 4.0);
        let mut tabs = [NodeId::dangling(); 2];
        for (k, name) in ["Keyboard", "Game controllers"].iter().enumerate() {
            let b = ui.add(seg, Button::new(tr(name)).icon(if k == 0 { "keyboard" } else { "sports_esports" }).class("tab"));
            ui.on_click(b, m(Msg::Tab(k)));
            tabs[k] = b;
        }
        // --- the keyboard
        let keys = ui.column(root);
        kit::grow(ui, keys);
        kit::gap(ui, keys, 12.0);
        let banner = ui.row(keys);
        ui.style(banner, |s| {
            s.padding = taffy::Rect { left: lp(14.0), right: lp(10.0), top: lp(10.0), bottom: lp(10.0) };
            s.gap = taffy::Size { width: lp(12.0), height: lp(0.0) };
            s.flex_shrink = 0.0;
        });
        ui.visual(banner, Visual::new().background(ACCENT.gamma_multiply(0.1)).border(egui_retained::epaint::Stroke::new(1.0, ACCENT.gamma_multiply(0.45))).radius(8.0_f32));
        let i = ui.add(banner, Icon::new("info"));
        ui.visual(i, Visual::new().color(ACCENT).font_size(20.0_f32));
        let banner_text = kit::para(ui, banner, "", "");
        kit::grow(ui, banner_text);
        let use_b = ui.add(banner, Button::new(tr("Use these keys")).icon("keyboard").class("primary"));
        ui.on_click(use_b, m(Msg::UseCustom));
        let cols = ui.row(keys);
        kit::grow(ui, cols);
        ui.style(cols, |s| {
            s.gap = taffy::Size { width: lp(14.0), height: lp(14.0) };
            s.align_items = Some(taffy::AlignItems::Stretch);
            s.min_size.height = len(0.0);
        });
        let mut filters = [NodeId::dangling(); 2];
        let mut lists = [NodeId::dangling(); 2];
        for (sec, (title, sub, _, icon)) in SECTIONS.iter().enumerate() {
            let c = kit::card(ui, cols);
            ui.style(c, |s| {
                s.flex_grow = 1.0;
                s.flex_basis = len(0.0);
                s.min_size = taffy::Size { width: len(0.0), height: len(0.0) };
            });
            let h = ui.row(c);
            kit::gap(ui, h, 8.0);
            let ic = ui.add(h, Icon::new(*icon));
            ui.add_class(ic, "dim");
            kit::text(ui, h, &tr(title), "heading");
            kit::text(ui, c, &tr(sub), "dim");
            let fr = ui.row(c);
            kit::gap(ui, fr, 8.0);
            let f = ui.add(fr, TextInput::new("").hint(tr("Filter…")).on_change(move |s| m(Msg::Filter(sec, s))));
            kit::grow(ui, f);
            filters[sec] = f;
            if sec == 0 {
                let a = ui.add(fr, Button::new(tr("Add binding")).icon("add"));
                ui.on_click(a, m(Msg::OpenPicker(sec)));
                ui.set_name(a, "kb-add");
            }
            let list = ui.column(c);
            kit::grow(ui, list);
            ui.set_scroll(list, ScrollAxes { x: false, y: true });
            ui.style(list, |s| {
                s.min_size.height = len(0.0);
                s.gap = taffy::Size { width: lp(4.0), height: lp(4.0) };
            });
            lists[sec] = list;
        }
        let error = kit::text(ui, keys, "", "danger-text");
        // --- the game controllers
        let pads = ui.row(root);
        kit::grow(ui, pads);
        ui.style(pads, |s| {
            s.gap = taffy::Size { width: lp(14.0), height: lp(14.0) };
            s.align_items = Some(taffy::AlignItems::Stretch);
            s.min_size.height = len(0.0);
        });
        // --- the picker, over the page
        let picker = ui.column(ui.root(Layer::Overlay));
        ui.style(picker, |s| {
            s.position = taffy::Position::Absolute;
            s.inset = taffy::Rect { left: kit::lpa(0.0), right: kit::lpa(0.0), top: kit::lpa(0.0), bottom: kit::lpa(0.0) };
            s.align_items = Some(taffy::AlignItems::Center);
            s.justify_content = Some(taffy::JustifyContent::Center);
        });
        ui.visual(picker, Visual::new().background(Color32::from_black_alpha(170)));
        ui.set_visible(picker, false);
        let none = NodeId::dangling();
        ControlsPage {
            root,
            subtitle,
            tabs,
            keys,
            pads,
            banner,
            banner_text,
            filters,
            shown_filter: Default::default(),
            lists,
            list_keys: [0; 2],
            error,
            picker,
            picker_key: 0,
            picker_query: none,
            picker_source: none,
            shown_source: String::new(),
            picker_status: none,
            picker_list: none,
            picker_suggestions: none,
            pads_key: 0,
            connected: Vec::new(),
            live: Vec::new(),
            button_rows: Vec::new(),
            reveal: None,
            ff_status: none,
        }
    }

    pub fn sync(&mut self, ui: &mut Ui, l: &Launcher) {
        let tab = l.pages.controls_tab.min(1);
        for (k, n) in self.tabs.iter().enumerate() {
            ui.set_selected(*n, k == tab);
        }
        ui.set_visible(self.keys, tab == 0);
        ui.set_visible(self.pads, tab == 1);
        ui.set_text(self.subtitle, &tr(if tab == 0 { "Click a key and press the new one (hold Shift, Ctrl or Alt for a combination); Escape leaves it as it is." } else { "What each axis and button of a wheel, pedals or joystick does - OMSI 2's gamectrler.cfg, kept in the content folder." }));
        if tab == 0 {
            self.sync_keys(ui, l);
        } else {
            self.sync_pads(ui, l);
        }
        let picking = tab == 0 && l.pages.kb_picker.is_some();
        ui.set_visible(self.picker, picking);
        if picking {
            self.sync_picker(ui, l);
        } else {
            self.picker_key = 0;
        }
    }

    // --- the keyboard ------------------------------------------------------------------------

    fn sync_keys(&mut self, ui: &mut Ui, l: &Launcher) {
        let preset = l.state.settings.get("drive_keys").and_then(|v| v.as_str()).unwrap_or("simple");
        ui.set_visible(self.banner, preset != "omsi");
        if preset != "omsi" {
            let name = match preset {
                "wasd" => "W A S D only",
                "arrows" => "Arrow keys only",
                _ => "W A S D + arrows",
            };
            ui.set_text(self.banner_text, &format!("Driving keys: {name} (Settings). Those keys drive the bus and win over the list below. Change any key here and your own layout (Custom controls) is used from then on."));
        }
        ui.set_text(self.error, &l.state.keybindings_error);
        ui.set_visible(self.error, !l.state.keybindings_error.is_empty());
        let names = pages::control_names(l);
        for sec in 0..2 {
            let filter = &l.pages.kb_filter[sec];
            if *filter != self.shown_filter[sec] && ui.focused() != Some(self.filters[sec]) {
                ui.with::<TextInput, _>(self.filters[sec], |t| t.set(filter));
            }
            self.shown_filter[sec] = filter.clone();
            let key = section_key(sec);
            let bindings = l.state.keybindings.get(key).cloned().unwrap_or(Value::Null);
            let h = {
                let mut h = std::collections::hash_map::DefaultHasher::new();
                filter.hash(&mut h);
                bindings.to_string().hash(&mut h);
                l.pages.capturing.hash(&mut h);
                h.finish()
            };
            if h == self.list_keys[sec] {
                continue;
            }
            self.list_keys[sec] = h;
            let list_node = self.lists[sec];
            ui.clear(list_node);
            let q = filter.to_lowercase();
            let list: Vec<(usize, String, i64, i64)> = bindings
                .as_array()
                .map(|a| a.iter().enumerate().map(|(i, b)| (i, b.get("action").and_then(|x| x.as_str()).unwrap_or("").to_string(), b.get("scan_code").and_then(|x| x.as_i64()).unwrap_or(0), b.get("modifier").and_then(|x| x.as_i64()).unwrap_or(0))).collect())
                .unwrap_or_default();
            let mut shown: Vec<(usize, String, String, bool)> = list
                .iter()
                .filter(|(_, a, s, md)| q.is_empty() || crate::launcher::ui::matches(&pages::action_text(names, a), &q) || a.to_lowercase().contains(&q) || crate::keys::key_name(*s, *md).to_lowercase().contains(&q))
                .map(|(i, a, s, md)| {
                    let clash = *s != 0 && list.iter().any(|(j, _, s2, m2)| j != i && s2 == s && m2 == md);
                    (*i, pages::action_text(names, a), crate::keys::key_name(*s, *md), clash)
                })
                .collect();
            if sec == 1 {
                shown.sort_by_key(|(_, label, _, _)| !label.starts_with("VR:"));
            }
            // a name the list does not have: added as a key of its own (#854)
            let new_action = filter.trim();
            if shown.is_empty() && new_action.len() > 1 && new_action.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                let b = ui.add(list_node, Button::new(format!("{} \"{new_action}\"", tr("Add and give it a key:"))).icon("add"));
                ui.on_click(b, m(Msg::AddCustom(sec)));
            } else if shown.is_empty() {
                kit::para(ui, list_node, &tr("No key matches the filter."), "dim");
            }
            for (i, label, keyn, clash) in shown {
                let r = ui.row(list_node);
                ui.style(r, |s| {
                    s.padding = taffy::Rect { left: lp(10.0), right: lp(4.0), top: lp(3.0), bottom: lp(3.0) };
                    s.gap = taffy::Size { width: lp(6.0), height: lp(0.0) };
                    s.flex_shrink = 0.0;
                });
                ui.visual(r, Visual::new().background(Color32::from_white_alpha(7)).radius(8.0_f32));
                let t = ui.add(r, Text::new(label));
                ui.style(t, |s| {
                    s.flex_grow = 1.0;
                    s.flex_basis = len(0.0);
                    s.min_size.width = len(0.0);
                });
                let more = ui.add(r, Button::new("").icon("add").class("ghost"));
                ui.set_tooltip(more, Some(tr("Add another key for this action")));
                ui.on_click(more, m(Msg::More(sec, i)));
                let waiting = l.pages.capturing == Some((sec, i));
                let kb = ui.add(r, Button::new(if waiting { tr("press a key…") } else { keyn }).class(if waiting { "primary" } else if clash { "danger" } else { "button" }));
                ui.style(kb, |s| s.size.width = len(150.0));
                ui.on_click(kb, m(Msg::Capture(sec, i)));
                if clash {
                    ui.set_tooltip(kb, Some(tr("Another action has the same key")));
                }
                let clear = ui.add(r, Button::new("").icon("delete").class("ghost"));
                ui.set_tooltip(clear, Some(tr("Clear the key (the entry stays)")));
                ui.on_click(clear, m(Msg::Clear(sec, i)));
                let x = ui.add(r, Button::new("").icon("close").class("ghost"));
                ui.set_tooltip(x, Some(tr("Remove the entry")));
                ui.on_click(x, m(Msg::Remove(sec, i)));
            }
        }
    }

    fn sync_picker(&mut self, ui: &mut Ui, l: &Launcher) {
        let section = l.pages.kb_picker.unwrap_or(0);
        let sources = l.pages.kb_source_action.clone();
        let filtered: &[_] = l.pages.kb_filtered_options.as_ref().map(|(_, _, o)| o.as_slice()).unwrap_or(&[]);
        let h = {
            let mut h = std::collections::hash_map::DefaultHasher::new();
            sources.hash(&mut h);
            l.pages.kb_picker_filter.hash(&mut h);
            l.pages.kb_picker_source_filter.hash(&mut h);
            filtered.len().hash(&mut h);
            l.pages.kb_script_scan_complete.hash(&mut h);
            l.pages.kb_script_total_buses.hash(&mut h);
            h.finish() | 1
        };
        // the dialog itself: built once per opening (the fields keep their focus)
        if self.picker_key == 0 || sources.is_some() != (self.picker_query == NodeId::dangling()) {
            ui.clear(self.picker);
            let none = NodeId::dangling();
            (self.picker_query, self.picker_source, self.picker_status, self.picker_list, self.picker_suggestions) = (none, none, none, none, none);
            let c = ui.column(self.picker);
            ui.add_class(c, "dialog");
            ui.style(c, |s| {
                s.size = taffy::Size { width: len(720.0), height: len(760.0) };
                s.max_size = taffy::Size { width: taffy::Dimension::percent(0.95), height: taffy::Dimension::percent(0.94) };
                s.padding = taffy::Rect::length(20.0_f32);
                s.gap = taffy::Size { width: lp(10.0), height: lp(10.0) };
            });
            if let Some((action, paths)) = &sources {
                let t = kit::text(ui, c, &tr("Bus files using this action"), "strong");
                ui.visual(t, Visual::new().font_size(17.0_f32));
                kit::text(ui, c, action, "dim");
                let list = ui.column(c);
                kit::grow(ui, list);
                ui.set_scroll(list, ScrollAxes { x: false, y: true });
                ui.style(list, |s| {
                    s.min_size.height = len(0.0);
                    s.gap = taffy::Size { width: lp(2.0), height: lp(2.0) };
                });
                for p in paths {
                    let n = kit::text(ui, list, p, "dim");
                    ui.style(n, |s| s.padding = taffy::Rect { left: lp(10.0), right: lp(10.0), top: lp(5.0), bottom: lp(5.0) });
                    ui.visual(n, Visual::new().background(Color32::from_white_alpha(9)).radius(4.0_f32));
                }
                let br = ui.row(c);
                ui.spacer(br);
                let b = ui.add(br, Button::new(tr("Back")));
                ui.on_click(b, m(Msg::SourcesBack));
            } else {
                let t = kit::text(ui, c, &tr("Add a key binding"), "strong");
                ui.visual(t, Visual::new().font_size(19.0_f32));
                kit::text(ui, c, &tr(if section == 0 { "Driving & the bus" } else { "The game" }), "dim");
                let fr = ui.row(c);
                kit::gap(ui, fr, 12.0);
                self.picker_query = ui.add(fr, TextInput::new(l.pages.kb_picker_filter.clone()).hint(tr("Search actions…")).on_change(|s| m(Msg::PickerQuery(s))));
                kit::grow(ui, self.picker_query);
                self.picker_source = ui.add(fr, TextInput::new(l.pages.kb_picker_source_filter.clone()).hint(tr("Filter by bus folder or bus file (.bus)")).on_change(|s| m(Msg::PickerSource(s))));
                kit::grow(ui, self.picker_source);
                self.shown_source = l.pages.kb_picker_source_filter.clone();
                self.picker_suggestions = ui.column(c);
                ui.set_scroll(self.picker_suggestions, ScrollAxes { x: false, y: true });
                ui.style(self.picker_suggestions, |s| s.max_size.height = len(140.0));
                self.picker_status = kit::text(ui, c, "", "faint");
                self.picker_list = ui.column(c);
                kit::grow(ui, self.picker_list);
                ui.set_scroll(self.picker_list, ScrollAxes { x: false, y: true });
                ui.style(self.picker_list, |s| {
                    s.min_size.height = len(0.0);
                    s.gap = taffy::Size { width: lp(2.0), height: lp(2.0) };
                });
                let br = ui.row(c);
                ui.spacer(br);
                let b = ui.add(br, Button::new(tr("Cancel")).class("ghost"));
                ui.on_click(b, m(Msg::PickerCancel));
                ui.focus(Some(self.picker_query));
            }
            self.picker_key = 0;
        }
        if sources.is_some() || h == self.picker_key {
            return;
        }
        self.picker_key = h;
        if self.shown_source != l.pages.kb_picker_source_filter && ui.focused() != Some(self.picker_source) {
            ui.with::<TextInput, _>(self.picker_source, |t| t.set(&l.pages.kb_picker_source_filter));
        }
        self.shown_source = l.pages.kb_picker_source_filter.clone();
        // the bus folders and files that match what is typed
        ui.clear(self.picker_suggestions);
        let typed = normalize_source_query(&l.pages.kb_picker_source_filter);
        let sugg = source_suggestions(&l.pages.kb_source_paths, &l.pages.kb_picker_source_filter);
        if !typed.is_empty() && !sugg.iter().any(|p| normalize_source_query(p) == typed) {
            for p in sugg.iter().take(40) {
                let b = ui.add(self.picker_suggestions, Button::new(p.clone()).class("ghost").align(egui_retained::widgets::TextAlign::Left));
                ui.on_click(b, m(Msg::PickerSuggestion(p.clone())));
            }
        }
        let (done, total, current) = &l.pages.kb_script_scan;
        let n = filtered.len();
        let status = if l.pages.kb_script_scan_complete {
            format!("{n} results · scanned {total} vehicle files")
        } else if *total > 0 {
            format!("{n} results · scanning {done} / {total} · {current}")
        } else {
            format!("{n} results · finding installed bus scripts…")
        };
        ui.set_text(self.picker_status, &status);
        ui.clear(self.picker_list);
        if filtered.is_empty() {
            kit::para(ui, self.picker_list, &tr("No matching actions."), "dim");
        }
        let total_buses = l.pages.kb_script_total_buses;
        for o in filtered.iter().take(PICKER_ROWS) {
            let r = ui.row(self.picker_list);
            ui.add_class(r, "list-row");
            ui.on_click(r, m(Msg::Pick(o.action.clone())));
            let tc = ui.column(r);
            kit::grow(ui, tc);
            kit::text(ui, tc, &o.label, "strong");
            let a = kit::text(ui, tc, &o.action, "faint");
            ui.visual(a, Visual::new().font_size(10.5_f32));
            if !o.bus_paths.is_empty() {
                let b = ui.add(r, Button::new(bus_usage_label(o.bus_paths.len(), total_buses)).icon("list").class("ghost"));
                ui.on_click(b, m(Msg::Sources(o.action.clone())));
            }
        }
        if filtered.len() > PICKER_ROWS {
            kit::para(ui, self.picker_list, &format!("{} more - search to find them.", filtered.len() - PICKER_ROWS), "faint");
        }
    }

    // --- the game controllers ----------------------------------------------------------------

    fn sync_pads(&mut self, ui: &mut Ui, l: &Launcher) {
        let pv = &l.pages.pads;
        let devices = pv.devices.clone().unwrap_or_default();
        let lit = pv.last_pressed.filter(|(_, t)| t.elapsed().as_secs_f32() < 4.0).map(|(b, _)| b);
        let armed = pv.confirm_remove.is_some_and(|t| t.elapsed().as_secs() < 4);
        let h = {
            let mut h = std::collections::hash_map::DefaultHasher::new();
            format!("{devices:?}").hash(&mut h);
            pv.selected.hash(&mut h);
            pv.dirty.hash(&mut h);
            pv.capturing.hash(&mut h);
            pv.revealed_button.hash(&mut h);
            lit.hash(&mut h);
            armed.hash(&mut h);
            pv.feedback_test.hash(&mut h);
            if let Some(w) = &pv.wizard {
                (w.step, &w.error, w.ff_choice, w.calibration.as_ref().map(|c| format!("{:?}", c.1.result))).hash(&mut h);
                w.test_strength.to_bits().hash(&mut h);
            }
            for c in &self.connected {
                (&c.name, c.buttons, c.ff_capable, c.gamepad, c.axes.iter().map(|a| a.0).collect::<Vec<_>>()).hash(&mut h);
            }
            for k in ["ctrl_off", "ctrl_deadzone", "ff_invert", "momentary_gears"] {
                l.state.settings.get(k).map(|v| v.to_string()).hash(&mut h);
            }
            l.pages.controller_action_choices.as_ref().map(|c| c.0.len()).hash(&mut h);
            h.finish()
        };
        if h != self.pads_key {
            self.pads_key = h;
            self.build_pads(ui, l, &devices, lit, armed);
        }
        // what changes each frame: the axes
        let live: Vec<(usize, f32)> = devices.get(pv.selected).and_then(|d| controllers::find_connected(&self.connected, &d.name)).map(|c| c.axes.clone()).unwrap_or_default();
        for (a, n) in &self.live {
            let v = live.iter().find(|(k, _)| k == a).map(|x| x.1);
            ui.update::<AxisBar>(*n, |b| std::mem::replace(&mut b.value, v) != v);
        }
        if let Some(b) = self.reveal.take().and_then(|b| self.button_rows.get(b).copied()) {
            ui.scroll_into_view(b);
        }
    }

    fn build_pads(&mut self, ui: &mut Ui, l: &Launcher, devices: &[DeviceCfg], lit: Option<usize>, armed: bool) {
        let pv = &l.pages.pads;
        ui.clear(self.pads);
        self.live.clear();
        self.button_rows.clear();
        self.ff_status = NodeId::dangling();
        // the devices
        let left = kit::card(ui, self.pads);
        ui.style(left, |s| {
            s.size.width = len(340.0);
            s.flex_shrink = 0.0;
        });
        let h = ui.row(left);
        kit::gap(ui, h, 8.0);
        let ic = ui.add(h, Icon::new("sports_esports"));
        ui.add_class(ic, "dim");
        kit::text(ui, h, &tr("Devices"), "heading");
        if !l.state.settings.get("momentary_gears").and_then(|v| v.as_bool()).unwrap_or(false) && crate::hpattern::has_held_bindings(devices) {
            kit::para(ui, left, &tr("H-pattern gears are assigned. Enable return to neutral under Settings → Driving → Game controllers if your shifter has no neutral button."), "dim");
        }
        let list = ui.column(left);
        kit::grow(ui, list);
        ui.set_scroll(list, ScrollAxes { x: false, y: true });
        ui.style(list, |s| {
            s.min_size.height = len(0.0);
            s.gap = taffy::Size { width: lp(4.0), height: lp(4.0) };
        });
        let offs = offs(l);
        for (i, d) in devices.iter().enumerate() {
            let on = self.connected.iter().any(|c| controllers::names_match(&d.name, &c.name));
            let off = offs.iter().any(|o| o.eq_ignore_ascii_case(&d.name));
            let r = ui.row(list);
            ui.add_class(r, "list-row");
            kit::gap(ui, r, 8.0);
            ui.set_selected(r, pv.selected == i);
            ui.on_click(r, m(Msg::PadSelect(i)));
            let t = kit::text(ui, r, &d.name, if on { "strong" } else { "dim" });
            kit::grow(ui, t);
            if off {
                kit::text(ui, r, &tr("off"), "faint");
            } else {
                let ic = ui.add(r, Icon::new(if on { "check_circle" } else { "remove" }));
                ui.visual(ic, Visual::new().color(if on { OK } else { Color32::from_gray(110) }));
            }
        }
        for c in self.connected.iter().filter(|c| !devices.iter().any(|d| controllers::names_match(&d.name, &c.name))) {
            let b = ui.add(list, Button::new(format!("{} {}", tr("Set up"), c.name)).icon("add").class("primary"));
            ui.on_click(b, m(Msg::PadAdd(c.name.clone())));
        }
        if devices.is_empty() && self.connected.is_empty() {
            kit::para(ui, list, &tr("No game controller is connected, and none is set up. Connect a wheel, pedals or a joystick; a gamepad works without setting up (left stick steers, the triggers are the pedals)."), "dim");
        }
        let dz = l.state.settings.get("ctrl_deadzone").and_then(|x| x.as_f64()).unwrap_or(0.0) as f32;
        let r = kit::labelled(ui, left, "Dead zone", 90.0);
        let s = ui.add(r, Slider::new(dz, 0.0, 0.3).step(0.01).on_change(|v| m(Msg::Deadzone(v))));
        kit::grow(ui, s);
        kit::text(ui, r, &format!("{:.0} %", dz * 100.0), "strong");
        let save = ui.add(left, Button::new(tr(if pv.dirty { "Save" } else { "Saved" })).icon("save").class(if pv.dirty { "primary" } else { "button" }));
        ui.on_click(save, m(Msg::PadSave));
        // the device shown
        let right = kit::card(ui, self.pads);
        ui.style(right, |s| {
            s.flex_grow = 1.0;
            s.flex_basis = len(0.0);
            s.min_size = taffy::Size { width: len(0.0), height: len(0.0) };
        });
        let Some(d) = devices.get(pv.selected) else {
            kit::para(ui, right, &tr("Choose a device on the left, or connect one."), "dim");
            return;
        };
        let connected = self.connected.clone();
        let live_dev = controllers::find_connected(&connected, &d.name);
        let buttons_only = pv.io.as_ref().is_some_and(|io| io.buttons_only(&d.name));
        let head = ui.row(right);
        kit::gap(ui, head, 10.0);
        let ic = ui.add(head, Icon::new("tune"));
        ui.add_class(ic, "dim");
        let t = kit::text(ui, head, &d.name, "heading");
        kit::grow(ui, t);
        if let Some(w) = &pv.wizard {
            self.build_wizard(ui, l, right, w, d, live_dev);
            return;
        }
        let on = !offs.iter().any(|o| o.eq_ignore_ascii_case(&d.name));
        ui.add(head, Checkbox::switch(on, tr("Use this device")).on_change(|v| m(Msg::PadOn(v))));
        if !buttons_only {
            let b = ui.add(head, Button::new(tr("Set up step by step")).icon("touch_app"));
            ui.on_click(b, m(Msg::PadWizard));
        }
        let body = ui.column(right);
        kit::grow(ui, body);
        ui.set_scroll(body, ScrollAxes { x: false, y: true });
        ui.style(body, |s| {
            s.min_size.height = len(0.0);
            s.gap = taffy::Size { width: lp(6.0), height: lp(6.0) };
        });
        if live_dev.is_some_and(|c| c.ff_capable) || d.ff_invert.is_some() {
            let (sf, vb) = d.ff_scale.unwrap_or((1.0, 1.0));
            for (label, v, msg) in [("Steering force", sf, Msg::FfSteering as fn(f32) -> Msg), ("Vibration", vb, Msg::FfVibration as fn(f32) -> Msg)] {
                let r = kit::labelled(ui, body, label, 130.0);
                let s = ui.add(r, Slider::new(v, 0.0, 2.0).step(0.05).on_change(move |x| m(msg(x))));
                kit::grow(ui, s);
                let t = kit::text(ui, r, &format!("{:.0}%", v * 100.0), "strong");
                ui.style(t, |s| s.min_size.width = len(50.0));
            }
            if !live_dev.is_some_and(|c| c.gamepad) {
                let inv = d.ff_invert.unwrap_or_else(|| l.state.settings.get("ff_invert").and_then(|v| v.as_bool()).unwrap_or(false));
                ui.add(body, Checkbox::switch(inv, tr("Invert force feedback")).on_change(|v| m(Msg::FfInvert(v))));
            }
        }
        if !buttons_only {
            let funcs: Vec<String> = Func::LABELS.iter().map(|s| tr(s)).collect();
            let shapes: Vec<String> = controllers::AXIS_SHAPES.iter().map(|s| tr(s.0)).collect();
            for a in 0..8 {
                let r = ui.row(body);
                kit::gap(ui, r, 10.0);
                let lab = kit::text(ui, r, &tr(AXES[a]), "dim");
                ui.style(lab, |s| s.size.width = len(96.0));
                let bar = ui.add(r, AxisBar { value: None });
                self.live.push((a, bar));
                let sel = (Func::code(d.axes[a].map(|x| x.0)) + 1) as usize;
                let slot = ui.row(r);
                ui.style(slot, |s| {
                    s.size.width = len(450.0);
                    s.flex_shrink = 0.0;
                    s.gap = taffy::Size { width: lp(10.0), height: lp(0.0) };
                });
                let r = slot;
                let s = ui.add(r, Select::new(funcs.clone(), Some(sel)).on_change(move |i| m(Msg::Axis(a, i))));
                ui.style(s, |s| s.size.width = len(180.0));
                if let Some((_, inv)) = d.axes[a] {
                    ui.add(r, Checkbox::switch(inv, tr("Reversed")).on_change(move |v| m(Msg::AxisReversed(a, v))));
                    let curve = d.axis_flags[a] & (4 | 8 | 0x10);
                    let shp = controllers::AXIS_SHAPES.iter().position(|s| s.1 == curve).unwrap_or(0);
                    let s = ui.add(r, Select::new(shapes.clone(), Some(shp)).on_change(move |i| m(Msg::AxisShape(a, i))));
                    ui.style(s, |s| s.size.width = len(140.0));
                }
            }
        }
        let bt = kit::text(ui, body, &tr("Buttons"), "heading");
        ui.style(bt, |s| s.margin.top = taffy::LengthPercentageAuto::length(8.0));
        let choices = l.pages.controller_action_choices.clone();
        let (actions, labels) = choices.as_deref().map(|c| (c.0.clone(), c.1.clone())).unwrap_or_default();
        let shown = pages::shown_button_count(&d.buttons, live_dev.map(|c| c.buttons).unwrap_or(0), pv.revealed_button);
        let grid = ui.row(body);
        ui.style(grid, |s| {
            s.flex_wrap = taffy::FlexWrap::Wrap;
            s.gap = taffy::Size { width: lp(14.0), height: lp(4.0) };
            s.align_items = Some(taffy::AlignItems::Center);
        });
        for (b, (act, _)) in d.buttons.iter().take(shown).enumerate() {
            let r = ui.row(grid);
            kit::gap(ui, r, 8.0);
            ui.style(r, |s| {
                s.flex_grow = 1.0;
                s.flex_basis = taffy::Dimension::percent(0.45);
                s.min_size.width = len(380.0);
                s.padding = taffy::Rect { left: lp(6.0), right: lp(6.0), top: lp(2.0), bottom: lp(2.0) };
            });
            if lit == Some(b) {
                ui.visual(r, Visual::new().background(ACCENT.gamma_multiply(0.28)).radius(6.0_f32));
            }
            let label = match b.checked_sub(controllers::HAT_BUTTONS) {
                Some(hh) => format!("Hat {} {}", hh / 4 + 1, ["up", "right", "down", "left"][hh % 4]),
                None => format!("{} {}", tr("Button"), b + 1),
            };
            let lt = kit::text(ui, r, &label, "dim");
            ui.style(lt, |s| s.size.width = len(84.0));
            let sel = actions.iter().position(|a| a.eq_ignore_ascii_case(act)).unwrap_or(0);
            let s = ui.add(r, Select::new(labels.clone(), Some(sel)).on_change(move |i| m(Msg::ButtonAction(b, i))));
            kit::grow(ui, s);
            ui.add(r, Checkbox::switch(d.latching.contains(&b), tr("Latching")).on_change(move |v| m(Msg::Latching(b, v))));
            self.button_rows.push(r);
        }
        if let Some(b) = pv.revealed_button {
            self.reveal = Some(b);
        }
        let foot = ui.row(right);
        kit::gap(ui, foot, 10.0);
        let add = ui.add(foot, Button::new(tr(if pv.capturing { "Press a button on the device…" } else { "Add a button" })).icon("add").class(if pv.capturing { "primary" } else { "button" }));
        ui.on_click(add, m(Msg::AddButton));
        ui.spacer(foot);
        let rm = ui.add(foot, Button::new(tr(if armed { "Click again to remove" } else { "Remove this device" })).icon("delete").class("danger"));
        ui.on_click(rm, m(Msg::RemoveDevice));
    }

    fn build_wizard(&mut self, ui: &mut Ui, l: &Launcher, c: NodeId, w: &Wizard, d: &DeviceCfg, live_dev: Option<&Connected>) {
        let body = ui.column(c);
        kit::grow(ui, body);
        ui.set_scroll(body, ScrollAxes { x: false, y: true });
        ui.style(body, |s| {
            s.min_size.height = len(0.0);
            s.gap = taffy::Size { width: lp(10.0), height: lp(10.0) };
        });
        let foot = |ui: &mut Ui, finish: Option<(&str, &str, Msg)>, skip: bool| {
            let f = ui.row(c);
            kit::gap(ui, f, 10.0);
            let b = ui.add(f, Button::new(tr("Cancel")).class("ghost"));
            ui.on_click(b, m(Msg::WizCancel));
            ui.spacer(f);
            if skip {
                let b = ui.add(f, Button::new(tr("Skip")));
                ui.on_click(b, m(Msg::WizSkip));
            }
            if let Some((t, i, msg)) = finish {
                let b = ui.add(f, Button::new(tr(t)).icon(i).class("primary"));
                ui.on_click(b, m(msg));
            }
        };
        if w.step < WIZARD_STEPS.len() {
            let (title, text) = WIZARD_STEPS[w.step];
            let t = kit::text(ui, body, &format!("{} {} / {}: {}", tr("Step"), w.step + 1, WIZARD_STEPS.len(), tr(title)), "strong");
            ui.visual(t, Visual::new().font_size(17.0_f32));
            kit::para(ui, body, &tr(text), "");
            if live_dev.is_none() {
                kit::para(ui, body, &tr("The device is not connected: plug it in (the list on the left marks it green)."), "danger-text");
            }
            if let Some(e) = &w.error {
                kit::para(ui, body, &tr(e), "danger-text");
            }
            for (k, _) in live_dev.map(|c| c.axes.clone()).unwrap_or_default() {
                let r = ui.row(body);
                kit::gap(ui, r, 10.0);
                let lab = kit::text(ui, r, ["X", "Y", "Z", "Rx", "Ry", "Rz", "Slider 1", "Slider 2"][k.min(7)], "dim");
                ui.style(lab, |s| s.size.width = len(84.0));
                let bar = ui.add(r, AxisBar { value: None });
                self.live.push((k, bar));
            }
            let feedback = live_dev.is_some_and(|c| c.ff_capable && !c.gamepad);
            let last = w.step + 1 == WIZARD_STEPS.len() && !feedback;
            foot(ui, Some((if last { "Finish" } else { "Next" }, "chevron_right", Msg::WizNext)), w.step >= 2);
            return;
        }
        // the force feedback's direction
        let t = kit::text(ui, body, &tr("Force feedback direction"), "strong");
        ui.visual(t, Visual::new().font_size(17.0_f32));
        let warn = ui.row(body);
        ui.style(warn, |s| {
            s.padding = taffy::Rect::length(12.0_f32);
            s.gap = taffy::Size { width: lp(12.0), height: lp(0.0) };
        });
        ui.visual(warn, Visual::new().background(DANGER.gamma_multiply(0.15)).border(egui_retained::epaint::Stroke::new(1.5, DANGER)).radius(6.0_f32));
        let ic = ui.add(warn, Icon::new("warning"));
        ui.visual(ic, Visual::new().color(DANGER).font_size(26.0_f32));
        let wt = kit::para(ui, warn, &tr("INJURY RISK: TAKE YOUR HANDS OFF THE WHEEL. Keep hands and fingers clear before starting and throughout the test."), "danger-text");
        ui.visual(wt, Visual::new().font_size(14.5_f32));
        kit::grow(ui, wt);
        kit::para(ui, body, &tr("The test applies two short forces in opposite directions. Finish and press Save to keep the detected direction for this wheel."), "");
        let active = l.pages.pads.feedback_test;
        if let Some((_, test)) = &w.calibration {
            let (msg, class) = match &test.result {
                Some(Ok(false)) => ("Direction detected: normal".to_string(), "ok-text"),
                Some(Ok(true)) => ("Direction detected: inverted".to_string(), "ok-text"),
                Some(Err(e)) => (e.to_string(), "danger-text"),
                None => ("Testing: keep your hands off the wheel…".to_string(), ""),
            };
            self.ff_status = kit::para(ui, body, &tr(&msg), class);
        }
        if !active {
            let r = kit::labelled(ui, body, "Test strength", 120.0);
            let s = ui.add(r, Slider::new(w.test_strength, crate::ffb_calibration::PULSE_FORCE, crate::ffb_calibration::MAX_PULSE_FORCE).step(0.01).on_change(|v| m(Msg::FfStrength(v))));
            kit::grow(ui, s);
            kit::text(ui, r, &format!("{:.0}%", w.test_strength * 100.0), "strong");
            kit::para(ui, body, &tr("If the wheel barely moves, increase Test strength and retry. Keep your hands clear."), "dim");
            let b = ui.add(body, Button::new(tr("Start test")).icon("play_arrow").class("primary"));
            ui.style(b, |s| s.align_self = Some(taffy::AlignSelf::FlexStart));
            ui.on_click(b, m(Msg::FfTest));
            let invert = w.ff_choice.or(d.ff_invert).unwrap_or_else(|| l.state.settings.get("ff_invert").and_then(|v| v.as_bool()).unwrap_or(false));
            ui.add(body, Checkbox::switch(invert, tr("Invert force feedback")).on_change(|v| m(Msg::FfManual(v))));
            kit::para(ui, body, &tr("If detection is inconclusive, retry or choose the direction manually. You can change it later on this device's page."), "dim");
        }
        if let Some(e) = &w.error {
            kit::para(ui, body, &tr(e), "danger-text");
        }
        foot(ui, (!active).then_some(("Finish", "check", Msg::FfFinish)), false);
    }
}

fn offs(l: &Launcher) -> Vec<String> {
    l.state.settings.get("ctrl_off").and_then(|v| v.as_str()).unwrap_or("").split('|').map(str::to_string).filter(|s| !s.is_empty()).collect()
}

fn new_wizard() -> Wizard {
    Wizard { step: 0, rest: [None; 8], at: Vec::new(), error: None, calibration: None, ff_choice: None, test_strength: crate::ffb_calibration::PULSE_FORCE }
}

impl Launcher {
    /// What the Controls page does each frame: the keyboard's key waited for, the picker's
    /// scan of the buses taken in, the controllers polled (a button pressed on the device
    /// shown is lit in its list) and the force feedback test driven.
    pub(super) fn gui_controls_tick(&mut self) {
        let on_page = self.page == crate::launcher::Page::Controls;
        if !on_page || self.pages.controls_tab != 1 {
            self.pages.pads.cancel_feedback_test();
        }
        let key = self.gui_key.take();
        if !on_page {
            return;
        }
        if self.pages.controls_tab == 0 {
            if let (Some((sec, idx)), Some((code, chord))) = (self.pages.capturing, key) {
                key_pressed(self, sec, idx, code, chord);
            }
            if self.pages.kb_picker.is_some() {
                ensure_key_action_catalog(self);
                let (q, s) = (self.pages.kb_picker_filter.clone(), self.pages.kb_picker_source_filter.clone());
                if self.pages.kb_filtered_options.as_ref().is_none_or(|(a, b, _)| *a != q || *b != s) {
                    let all = self.pages.kb_action_options.as_deref().unwrap_or_default();
                    self.pages.kb_filtered_options = Some((q.clone(), s.clone(), filter_action_options(all, &q, &s)));
                }
            }
            return;
        }
        let names = pages::control_names(self);
        if self.pages.controller_action_choices.is_none() {
            self.pages.controller_action_choices = Some(std::sync::Arc::new(controller_action_choices(names, &self.state.keybindings)));
        }
        let hwnd = self.window.as_deref().and_then(controllers::window_handle);
        let root = std::path::PathBuf::from(&self.state.config.root);
        let pv = &mut self.pages.pads;
        if pv.io.is_none() {
            pv.io = Some(controllers::Devices::new(hwnd, false));
        }
        if pv.devices.is_none() {
            pv.devices = Some(controllers::read_cfg(&root));
        }
        let mut pressed: Vec<(String, usize)> = Vec::new();
        let mut connected = Vec::new();
        if let Some(io) = pv.io.as_mut() {
            for (name, n, down) in io.poll() {
                if down {
                    pressed.push((name, n));
                }
            }
            connected = io.connected();
        }
        let devices = pv.devices.get_or_insert_with(Vec::new);
        let mut status = None;
        if let Some(d) = devices.get_mut(pv.selected) {
            // every button the device has gets its line
            if let Some(n) = controllers::find_connected(&connected, &d.name).map(|c| c.buttons).filter(|n| *n > d.buttons.len()) {
                d.buttons.resize(n, (String::new(), "0".into()));
            }
            // a button pressed on the device shown: its line lit (added up to it)
            if pv.wizard.is_none() {
                if let Some((name, n)) = pressed.into_iter().find(|(name, n)| controllers::names_match(&d.name, name) && *n < controllers::HAT_BUTTONS + 16) {
                    while d.buttons.len() <= n {
                        d.buttons.push((String::new(), "0".into()));
                        pv.dirty = true;
                    }
                    pv.capturing = false;
                    pv.revealed_button = Some(n);
                    pv.last_pressed = Some((n, std::time::Instant::now()));
                    let now = d.buttons.get(n).map(|b| b.0.clone()).filter(|a| !a.is_empty());
                    let label = match n.checked_sub(controllers::HAT_BUTTONS) {
                        Some(h) => format!("hat {} {}", h / 4 + 1, ["up", "right", "down", "left"][h % 4]),
                        None => format!("button {}", n + 1),
                    };
                    status = Some(match now {
                        Some(a) => format!("{name}: {label} - {} (lit in the list: choose another there)", pages::action_text(names, &a)),
                        None => format!("{name}: {label} - nothing yet (lit in the list: choose what it does)"),
                    });
                }
            }
            // the force feedback test, driven
            if let Some(w) = pv.wizard.as_mut().filter(|w| w.step == WIZARD_STEPS.len()) {
                let axes = pages::wizard_result(&w.rest, &w.at);
                let axis = axes.iter().position(|a| matches!(a, Some((Func::Steering, _))));
                let dev = controllers::find_connected(&connected, &d.name);
                let live: Vec<(usize, f32)> = dev.map(|c| c.axes.clone()).unwrap_or_default();
                if let Some((started, test)) = w.calibration.as_mut() {
                    if pv.feedback_test {
                        let position = axis.and_then(|a| live.iter().find(|(k, _)| *k == a).map(|(_, x)| *x));
                        if let Some(force) = test.update(started.elapsed().as_secs_f32(), position) {
                            if !dev.zip(axis).zip(pv.io.as_mut()).is_some_and(|((dev, axis), io)| io.calibration_pulse(&dev.name, axis, force)) {
                                test.fail("Force feedback is unavailable. Choose the direction manually.");
                            }
                        }
                        if let Some(result) = test.result {
                            pages::release_feedback(&mut pv.io, &mut pv.feedback_test);
                            if let Ok(invert) = result {
                                w.ff_choice = Some(invert);
                            }
                        }
                    }
                }
            }
        }
        if let Some(s) = status {
            self.state.set_status(s, false);
        }
        if let Some(p) = self.gui.as_mut().and_then(|g| g.controls.as_mut()) {
            p.connected = connected;
        }
    }
}

/// A key pressed while binding `idx` of section `sec` waits for one.
fn key_pressed(l: &mut Launcher, sec: usize, idx: usize, code: winit::keyboard::KeyCode, chord: i32) {
    use winit::keyboard::KeyCode as K;
    if code == K::Escape {
        l.pages.capturing = None;
        return;
    }
    if matches!(code, K::ShiftLeft | K::ShiftRight | K::ControlLeft | K::ControlRight | K::AltLeft | K::AltRight | K::SuperLeft | K::SuperRight) {
        return;
    }
    match crate::keys::dik_code(code) {
        Some(scan) => {
            let section = section_key(sec);
            let vr = vr_binding(l, section, idx);
            if let Some(b) = l.state.keybindings.get_mut(section).and_then(|a| a.as_array_mut()).and_then(|a| a.get_mut(idx)) {
                // (the entry's "held" bit is the action's, not the key's: it stays)
                let hold = b.get("modifier").and_then(|x| x.as_i64()).unwrap_or(0) & omsi_content::input::KEY_HOLD as i64;
                b["scan_code"] = json!(scan);
                b["modifier"] = json!(chord as i64 | hold);
            }
            pages::save_keys(l, vr);
        }
        None => l.state.set_status(format!("{code:?} has no DirectInput scan code the game understands."), true),
    }
    l.pages.capturing = None;
}

fn vr_binding(l: &Launcher, section: &str, i: usize) -> bool {
    l.state.keybindings.get(section).and_then(|a| a.as_array()).and_then(|a| a.get(i)).and_then(|b| b.get("action")).and_then(|a| a.as_str()).is_some_and(|a| a.starts_with("vr_"))
}

/// The assistant's Next (or Skip): where the axes stand taken, the step done; Some(true)
/// once the device is set up, Some(false) never (Cancel is its own message).
fn wizard_next(w: &mut Wizard, d: &mut DeviceCfg, live: &[(usize, f32)], skip: bool, feedback: bool) -> bool {
    let mut cur = [None; 8];
    for (k, v) in live {
        if *k < 8 {
            cur[*k] = Some(*v);
        }
    }
    w.error = None;
    if w.step == 0 {
        if live.is_empty() {
            w.error = Some("The device shows no axis yet: move the wheel and the pedals a little, let go, and press Next again.".into());
            return false;
        }
        w.rest = cur;
    } else if skip {
        w.at.push([None; 8]);
    } else {
        // the axis that moved most since everything was let go (one taken before is not
        // taken again - but the throttle's axis may turn out to be the brake's too)
        let used: Vec<usize> = w.at.iter().filter_map(|a| pages::moved_most(&w.rest, a, &[]).map(|x| x.0)).collect();
        let exclude: Vec<usize> = if w.step == 3 { used.iter().copied().take(1).collect() } else { used };
        if pages::moved_most(&w.rest, &cur, &exclude).is_none() {
            w.error = Some("Nothing moved far enough. Hold it all the way, then press Next (or Skip).".into());
            return false;
        }
        w.at.push(cur);
    }
    w.step += 1;
    if w.step < WIZARD_STEPS.len() {
        return false;
    }
    let axes = pages::wizard_result(&w.rest, &w.at);
    if feedback && axes.iter().any(|a| matches!(a, Some((Func::Steering, _)))) {
        // (on to the force feedback's direction)
        return false;
    }
    d.axes = axes;
    true
}

pub(in crate::launcher) fn handle(l: &mut Launcher, msg: Msg) {
    let reset_picker = |l: &mut Launcher| {
        l.pages.kb_picker = None;
        l.pages.kb_picker_filter.clear();
        l.pages.kb_picker_source_filter.clear();
        l.pages.kb_source_action = None;
    };
    match msg {
        Msg::Tab(k) => l.pages.controls_tab = k,
        Msg::UseCustom => {
            pages::use_custom_keys(l);
        }
        Msg::Filter(sec, s) => l.pages.kb_filter[sec] = s,
        Msg::OpenPicker(sec) => {
            l.pages.kb_picker = Some(sec);
            l.pages.kb_picker_filter.clear();
            l.pages.kb_picker_source_filter.clear();
            l.pages.kb_action_options = None;
            l.pages.kb_filtered_options = None;
            l.pages.capturing = None;
        }
        Msg::AddCustom(sec) => {
            let name = l.pages.kb_filter[sec].trim().to_string();
            if let Some(a) = l.state.keybindings.get_mut(section_key(sec)).and_then(|a| a.as_array_mut()) {
                a.push(json!({ "action": name, "scan_code": 0, "modifier": 0 }));
                l.pages.capturing = Some((sec, a.len() - 1));
            }
        }
        Msg::Capture(sec, i) => l.pages.capturing = if l.pages.capturing == Some((sec, i)) { None } else { Some((sec, i)) },
        Msg::More(sec, i) => {
            if let Some(a) = l.state.keybindings.get_mut(section_key(sec)).and_then(|a| a.as_array_mut()) {
                if let Some(b) = a.get(i).cloned() {
                    // (the held bit is the action's: it goes with it)
                    let hold = b.get("modifier").and_then(|x| x.as_i64()).unwrap_or(0) & omsi_content::input::KEY_HOLD as i64;
                    a.insert(i + 1, json!({ "action": b.get("action").cloned().unwrap_or(json!("")), "scan_code": 0, "modifier": hold }));
                    l.pages.capturing = Some((sec, i + 1));
                }
            }
        }
        Msg::Clear(sec, i) | Msg::Remove(sec, i) => {
            let remove = matches!(msg, Msg::Remove(..));
            let key = section_key(sec);
            let vr = vr_binding(l, key, i);
            if let Some(a) = l.state.keybindings.get_mut(key).and_then(|a| a.as_array_mut()) {
                if !remove {
                    if let Some(b) = a.get_mut(i) {
                        b["scan_code"] = json!(0);
                        b["modifier"] = json!(0);
                    }
                } else if i < a.len() {
                    a.remove(i);
                    l.pages.capturing = match l.pages.capturing {
                        Some((s, k)) if s == sec && k == i => None,
                        Some((s, k)) if s == sec && k > i => Some((s, k - 1)),
                        other => other,
                    };
                }
            }
            pages::save_keys(l, vr);
        }
        Msg::PickerQuery(s) => l.pages.kb_picker_filter = s,
        Msg::PickerSource(s) | Msg::PickerSuggestion(s) => l.pages.kb_picker_source_filter = s,
        Msg::Sources(action) => {
            let paths = l.pages.kb_action_options.as_ref().and_then(|o| o.iter().find(|x| x.action == action)).map(|o| o.bus_paths.clone()).unwrap_or_default();
            l.pages.kb_source_action = Some((action, paths));
        }
        Msg::SourcesBack => l.pages.kb_source_action = None,
        Msg::PickerCancel => reset_picker(l),
        Msg::Pick(action) => {
            let section = l.pages.kb_picker.unwrap_or(0);
            if let Some(b) = l.state.keybindings.get_mut(section_key(section)).and_then(Value::as_array_mut) {
                b.push(json!({ "action": action, "scan_code": 0, "modifier": 0 }));
                l.pages.capturing = Some((section, b.len() - 1));
            }
            l.pages.kb_filter[section] = action;
            l.state.set_status("Binding added. Press the key you want to use (Escape cancels).", false);
            l.pages.kb_action_options = None;
            l.pages.kb_filtered_options = None;
            l.pages.controller_action_choices = None;
            reset_picker(l);
        }
        Msg::PadSelect(i) => {
            let pv = &mut l.pages.pads;
            if i != pv.selected {
                pages::release_feedback(&mut pv.io, &mut pv.feedback_test);
                pv.selected = i;
                pv.capturing = false;
                pv.revealed_button = None;
                pv.wizard = None;
                pv.confirm_remove = None;
            }
        }
        Msg::PadAdd(name) => {
            let pv = &mut l.pages.pads;
            // (a device of buttons only - a gear shifter, a button box - has no axes for the
            // assistant: its buttons are given their keys on its page)
            let axes = !pv.io.as_ref().is_some_and(|io| io.buttons_only(&name));
            let devices = pv.devices.get_or_insert_with(Vec::new);
            devices.push(DeviceCfg { name, second: "0".into(), ..Default::default() });
            pv.selected = devices.len() - 1;
            pv.revealed_button = None;
            pv.dirty = true;
            if axes {
                pv.wizard = Some(new_wizard());
            }
        }
        Msg::Deadzone(v) => {
            l.state.settings["ctrl_deadzone"] = json!((v * 100.0).round() / 100.0);
            l.state.settings_dirty = 0.3;
        }
        Msg::PadSave => {
            let pv = &mut l.pages.pads;
            if pv.dirty {
                match pages::save_gamectrler(pv.devices.as_deref().unwrap_or_default()) {
                    Ok(p) => {
                        pv.dirty = false;
                        l.state.set_status(format!("Game controllers saved to {}", p.display()), false);
                    }
                    Err(e) => l.state.set_status(format!("Not saved: {e}"), true),
                }
            }
        }
        Msg::PadOn(on) => {
            let Some(name) = selected(l).map(|d| d.name.clone()) else { return };
            let mut o: Vec<String> = offs(l).into_iter().filter(|x| !x.eq_ignore_ascii_case(&name)).collect();
            if !on {
                o.push(name);
            }
            l.state.settings["ctrl_off"] = json!(o.join("|"));
            l.state.settings_dirty = 0.3;
        }
        Msg::PadWizard => l.pages.pads.wizard = Some(new_wizard()),
        Msg::Axis(a, sel) => edit(l, |d| {
            let inv = d.axes[a].map(|x| x.1).unwrap_or(false);
            d.axes[a] = Func::from_code(sel as i32 - 1).map(|f| (f, inv));
        }),
        Msg::AxisReversed(a, v) => edit(l, |d| {
            if let Some(x) = d.axes[a].as_mut() {
                x.1 = v;
            }
        }),
        Msg::AxisShape(a, i) => edit(l, |d| {
            d.axis_flags[a] = (d.axis_flags[a] & !(4 | 8 | 0x10)) | controllers::AXIS_SHAPES[i.min(controllers::AXIS_SHAPES.len() - 1)].1;
        }),
        Msg::ButtonAction(b, i) => {
            let action = l.pages.controller_action_choices.as_ref().and_then(|c| c.0.get(i).cloned()).unwrap_or_default();
            edit(l, |d| {
                if let Some(x) = d.buttons.get_mut(b) {
                    x.0 = if i == 0 { String::new() } else { action };
                }
            });
        }
        Msg::Latching(b, v) => edit(l, |d| {
            d.latching.retain(|x| *x != b);
            if v {
                d.latching.push(b);
                d.latching.sort_unstable();
            }
        }),
        Msg::FfSteering(v) => edit(l, |d| d.ff_scale = Some((v, d.ff_scale.map(|s| s.1).unwrap_or(1.0)))),
        Msg::FfVibration(v) => edit(l, |d| d.ff_scale = Some((d.ff_scale.map(|s| s.0).unwrap_or(1.0), v))),
        Msg::FfInvert(v) => edit(l, |d| d.ff_invert = Some(v)),
        Msg::AddButton => l.pages.pads.capturing = !l.pages.pads.capturing,
        Msg::RemoveDevice => {
            let pv = &mut l.pages.pads;
            if pv.confirm_remove.is_some_and(|t| t.elapsed().as_secs() < 4) {
                pages::release_feedback(&mut pv.io, &mut pv.feedback_test);
                let name = pages::remove_device(pv.devices.get_or_insert_with(Vec::new), &mut pv.selected);
                pv.capturing = false;
                pv.revealed_button = None;
                pv.last_pressed = None;
                pv.confirm_remove = None;
                pv.dirty = true;
                l.state.set_status(format!("{name} removed: press Save to keep it so."), false);
            } else {
                pv.confirm_remove = Some(std::time::Instant::now());
            }
        }
        Msg::WizCancel => {
            let pv = &mut l.pages.pads;
            pages::release_feedback(&mut pv.io, &mut pv.feedback_test);
            pv.wizard = None;
        }
        Msg::WizNext | Msg::WizSkip => {
            let connected = l.gui.as_ref().and_then(|g| g.controls.as_ref()).map(|p| p.connected.clone()).unwrap_or_default();
            let pv = &mut l.pages.pads;
            let Some(d) = pv.devices.as_mut().and_then(|v| v.get_mut(pv.selected)) else { return };
            let Some(w) = pv.wizard.as_mut() else { return };
            let dev = controllers::find_connected(&connected, &d.name);
            let live = dev.map(|c| c.axes.clone()).unwrap_or_default();
            let feedback = dev.is_some_and(|c| c.ff_capable && !c.gamepad);
            if wizard_next(w, d, &live, matches!(msg, Msg::WizSkip), feedback) {
                pv.wizard = None;
                pv.dirty = true;
                l.state.set_status("Set up: press Save to keep it (the buttons can be given their keys below).", false);
            }
        }
        Msg::FfStrength(v) => {
            if let Some(w) = l.pages.pads.wizard.as_mut() {
                w.test_strength = v;
            }
        }
        Msg::FfTest => {
            let connected = l.gui.as_ref().and_then(|g| g.controls.as_ref()).map(|p| p.connected.clone()).unwrap_or_default();
            let hwnd = l.window.as_deref().and_then(controllers::window_handle);
            let pv = &mut l.pages.pads;
            let Some(d) = pv.devices.as_ref().and_then(|v| v.get(pv.selected)) else { return };
            let Some(w) = pv.wizard.as_mut() else { return };
            let axes = pages::wizard_result(&w.rest, &w.at);
            let axis = axes.iter().position(|a| matches!(a, Some((Func::Steering, _))));
            let dev = controllers::find_connected(&connected, &d.name);
            if dev.is_none() || axis.is_none() {
                w.error = Some("The wheel is unavailable. Reconnect it and try again.".into());
            } else {
                pv.io = None;
                pv.io = Some(controllers::Devices::new(hwnd, true));
                pv.feedback_test = true;
                w.error = None;
                log::info!("FFB calibration: device {}, raw steering axis {:?}, test strength {:.0}%", d.name, axis, w.test_strength * 100.0);
                w.calibration = Some((std::time::Instant::now(), crate::ffb_calibration::Calibration::new(w.test_strength)));
            }
        }
        Msg::FfManual(v) => {
            if let Some(w) = l.pages.pads.wizard.as_mut() {
                w.ff_choice = Some(v);
            }
        }
        Msg::FfFinish => {
            let global = l.state.settings.get("ff_invert").and_then(|v| v.as_bool()).unwrap_or(false);
            let pv = &mut l.pages.pads;
            let Some(d) = pv.devices.as_mut().and_then(|v| v.get_mut(pv.selected)) else { return };
            let Some(w) = pv.wizard.as_ref() else { return };
            d.axes = pages::wizard_result(&w.rest, &w.at);
            d.ff_invert = Some(w.ff_choice.or(d.ff_invert).unwrap_or(global));
            pages::release_feedback(&mut pv.io, &mut pv.feedback_test);
            pv.wizard = None;
            pv.dirty = true;
            l.state.set_status("Set up: press Save to keep it (the buttons can be given their keys below).", false);
        }
    }
}

fn selected(l: &Launcher) -> Option<&DeviceCfg> {
    l.pages.pads.devices.as_ref().and_then(|v| v.get(l.pages.pads.selected))
}

/// The device shown, changed (the page's Save keeps it).
fn edit(l: &mut Launcher, f: impl FnOnce(&mut DeviceCfg)) {
    let pv = &mut l.pages.pads;
    if let Some(d) = pv.devices.as_mut().and_then(|v| v.get_mut(pv.selected)) {
        f(d);
        pv.dirty = true;
    }
}
