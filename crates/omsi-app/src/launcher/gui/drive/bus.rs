//! The bus step: a search, the manufacturers (OMSI's `[friendlyname]` groups) with their
//! types, favourites starred, the livery, and the vehicle's own settings.

use super::super::kit::{self, len, lp, tr};
use super::{Msg, m};
use crate::launcher::drive::{self as logic, BusManufacturer};
use crate::launcher::Launcher;
use egui_retained::widgets::{Button, Checkbox, Icon, Select, TextInput};
use egui_retained::{NodeId, ScrollAxes, Ui, Visual, taffy};
use std::collections::{BTreeSet, HashSet};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub(in crate::launcher) enum BusMsg {
    Filter(String),
    OnlyFavourites(bool),
    Family(String),
    Pick(String),
    Star(String),
    Type(String, usize),
    Paint(usize),
    PaintStep(i32),
    Settings,
    Hof(usize),
    Number(usize),
    Plate(String),
}

fn b(x: BusMsg) -> super::super::Msg {
    m(Msg::Bus(x))
}

pub(super) struct BusStep {
    filter: String,
    only_favourites: bool,
    expanded: Option<String>,
    settings_open: bool,
    favourites: Option<BTreeSet<String>>,
    makers: Arc<Vec<BusManufacturer>>,
    makers_key: (usize, u64, usize),
    initialized: bool,
    // nodes
    count: NodeId,
    only: NodeId,
    list: NodeId,
    list_key: u64,
    paint_row: NodeId,
    paint: NodeId,
    paint_count: NodeId,
    paint_prev: NodeId,
    paint_next: NodeId,
    settings_toggle: NodeId,
    settings: NodeId,
    hof: NodeId,
    number_row: NodeId,
    number: NodeId,
    plate: NodeId,
    warn: NodeId,
    description: NodeId,
    file: NodeId,
    plate_shown: String,
    /// The open family's types as its dropdown lists them: (family, files, options before them).
    type_files: Option<(String, Vec<String>, usize)>,
}

impl BusStep {
    pub fn build(ui: &mut Ui, p: NodeId) -> BusStep {
        let h = kit::text(ui, p, &tr("Choose a bus"), "heading");
        let _ = h;
        let search = ui.add(p, TextInput::new("").hint(tr("Search buses…")).on_change(|s| b(BusMsg::Filter(s))));
        ui.set_name(search, "bus-filter");
        let row = ui.row(p);
        let count = kit::text(ui, row, "", "faint");
        ui.spacer(row);
        let only = ui.add(row, Checkbox::switch(false, tr("Favourites only")).on_change(|v| b(BusMsg::OnlyFavourites(v))));
        let list = ui.column(p);
        ui.add_class(list, "list");
        kit::grow(ui, list);
        ui.set_scroll(list, ScrollAxes { x: false, y: true });
        // the livery
        let paint_row = ui.column(p);
        kit::gap(ui, paint_row, 6.0);
        let lr = ui.row(paint_row);
        kit::text(ui, lr, &tr("Livery"), "dim");
        let paint_count = kit::right_text(ui, lr, "", "faint");
        let pr = ui.row(paint_row);
        kit::gap(ui, pr, 8.0);
        let paint = ui.add(pr, Select::new(Vec::new(), None).on_change(|i| b(BusMsg::Paint(i))));
        kit::grow(ui, paint);
        let paint_prev = ui.add(pr, Button::new("").icon("chevron_left"));
        ui.on_click(paint_prev, b(BusMsg::PaintStep(-1)));
        ui.set_tooltip(paint_prev, Some(tr("Preview previous livery")));
        let paint_next = ui.add(pr, Button::new("").icon("chevron_right"));
        ui.on_click(paint_next, b(BusMsg::PaintStep(1)));
        ui.set_tooltip(paint_next, Some(tr("Preview next livery")));
        // the vehicle's own settings
        let settings_toggle = ui.add(p, Button::new(tr("Vehicle settings & details")).icon("expand_more").class("ghost").align(egui_retained::widgets::TextAlign::Left));
        ui.style(settings_toggle, |s| s.justify_content = Some(taffy::JustifyContent::FlexStart));
        ui.on_click(settings_toggle, b(BusMsg::Settings));
        let settings = ui.column(p);
        kit::gap(ui, settings, 8.0);
        ui.set_scroll(settings, ScrollAxes { x: false, y: true });
        ui.style(settings, |s| s.max_size.height = len(220.0));
        let r = kit::labelled(ui, settings, "Depot file", 110.0);
        let hof = ui.add(r, Select::new(Vec::new(), None).on_change(|i| b(BusMsg::Hof(i))));
        kit::grow(ui, hof);
        let number_row = kit::labelled(ui, settings, "Fleet number", 110.0);
        let number = ui.add(number_row, Select::new(Vec::new(), None).on_change(|i| b(BusMsg::Number(i))));
        kit::grow(ui, number);
        let r = kit::labelled(ui, settings, "Number plate", 110.0);
        let plate = ui.add(r, TextInput::new("").hint(tr("Automatic")).on_change(|s| b(BusMsg::Plate(s))));
        kit::grow(ui, plate);
        let warn = kit::para(ui, settings, "", "warn-text");
        let description = kit::para(ui, settings, "", "dim");
        let file = kit::para(ui, settings, "", "faint");
        BusStep {
            filter: String::new(),
            only_favourites: false,
            expanded: None,
            settings_open: false,
            favourites: None,
            makers: Arc::new(Vec::new()),
            makers_key: (usize::MAX, 0, 0),
            initialized: false,
            count,
            only,
            list,
            list_key: 0,
            paint_row,
            paint,
            paint_count,
            paint_prev,
            paint_next,
            settings_toggle,
            settings,
            hof,
            number_row,
            number,
            plate,
            warn,
            description,
            file,
            plate_shown: String::new(),
            type_files: None,
        }
    }

    fn favs(&mut self) -> &BTreeSet<String> {
        self.favourites.get_or_insert_with(logic::read_favourites)
    }

    pub fn sync(&mut self, ui: &mut Ui, l: &Launcher) {
        let norm = |f: &str| f.replace('\\', "/").to_lowercase();
        let allowed: Option<HashSet<String>> = l.state.host_vehicles().map(|v| v.iter().map(|f| norm(f)).collect());
        let allowed_key = allowed.as_ref().map(|a| a.iter().fold(0u64, |h, f| h ^ hash(f))).unwrap_or(u64::MAX);
        let key = (l.state.vehicles.len(), allowed_key, l.state.fresh.len());
        if key != self.makers_key {
            let fresh = l.state.fresh.keys().cloned().collect();
            self.makers = Arc::new(logic::build_bus_manufacturers(&l.state.vehicles, allowed.as_ref(), &fresh));
            self.makers_key = key;
        }
        let chosen = l.state.choice.bus.clone();
        if !self.initialized && !self.makers.is_empty() {
            self.initialized = true;
            self.expanded = self.makers.iter().find(|mk| mk.variants.iter().any(|v| v.file == chosen)).map(|mk| mk.key.clone());
        }
        let favs = self.favs().clone();
        let is_fav = |f: &str| favs.contains(&logic::fav_key(f));
        let only = self.only_favourites && !favs.is_empty();
        let q = omsi_launcher_lib::display_bus_name(self.filter.trim()).to_lowercase();
        let makers = self.makers.clone();
        let visible: Vec<&BusManufacturer> = makers.iter().filter(|mk| logic::manufacturer_matches(mk, &q) && (!only || mk.variants.iter().any(|v| is_fav(&v.file)))).collect();
        ui.set_text(self.count, &format!("{} {}", visible.len(), tr(if visible.len() == 1 { "manufacturer" } else { "manufacturers" })));
        ui.set_checked(self.only, self.only_favourites);
        // the list, built again when what it shows changed
        let lk = hash(&format!("{q}|{only}|{:?}|{chosen}|{}|{}|{:?}", self.expanded, visible.len(), favs.len(), self.makers_key));
        if lk != self.list_key {
            self.list_key = lk;
            ui.clear(self.list);
            if visible.is_empty() {
                kit::para(ui, self.list, &tr(if l.state.loading_content { "Reading the buses…" } else { "No buses found. Try another search." }), "dim");
            }
            let mut scroll_to = None;
            for mk in &visible {
                let selected = mk.variants.iter().find(|v| v.file == chosen);
                let single = mk.variants.len() == 1;
                let open = !single && self.expanded.as_deref() == Some(mk.key.as_str());
                let row = ui.row(self.list);
                ui.add_class(row, "list-row");
                kit::gap(ui, row, 10.0);
                ui.set_selected(row, selected.is_some());
                ui.on_click(row, if single { b(BusMsg::Pick(mk.variants[0].file.clone())) } else { b(BusMsg::Family(mk.key.clone())) });
                let icon = ui.add(row, Icon::new("directions_bus"));
                if selected.is_some() {
                    ui.visual(icon, Visual::new().color(super::super::theme::ACCENT));
                    scroll_to = Some(row);
                }
                let texts = ui.column(row);
                kit::grow(ui, texts);
                kit::gap(ui, texts, 2.0);
                kit::text(ui, texts, &mk.name, "strong");
                let mut sub = if single {
                    format!("{} · {}", tr(&mk.variants[0].variant), logic::liveries_text(mk.variants[0].paints))
                } else if let Some(v) = selected {
                    format!("{} · {} {}", tr(&v.variant), mk.variants.len(), tr("models"))
                } else {
                    format!("{} {}", mk.variants.len(), tr("models"))
                };
                if let Some(v) = selected {
                    if v.incomplete {
                        sub = format!("{sub} · {}", tr("PARTS MISSING"));
                    } else if v.fresh {
                        sub = format!("{sub} · {}", tr("NEW"));
                    } else if v.installed {
                        sub = format!("{sub} · {}", tr("MOD"));
                    }
                }
                kit::text(ui, texts, &sub, "faint");
                let starred = mk.variants.iter().any(|v| is_fav(&v.file));
                if single {
                    let star = ui.add(row, Button::new("").icon("star").class("ghost"));
                    ui.on_click(star, b(BusMsg::Star(mk.variants[0].file.clone())));
                    ui.set_tooltip(star, Some(tr(if starred { "Remove from the favourites" } else { "Add to the favourites" })));
                    if starred {
                        ui.visual(star, Visual::new().color(super::super::theme::ACCENT));
                    }
                } else if starred {
                    let s = ui.add(row, Icon::new("star"));
                    ui.visual(s, Visual::new().color(super::super::theme::ACCENT));
                }
                let chev = ui.add(row, Icon::new(if single { if selected.is_some() { "check" } else { "chevron_right" } } else if open { "expand_less" } else { "expand_more" }));
                if selected.is_some() {
                    ui.visual(chev, Visual::new().color(super::super::theme::ACCENT));
                }
                if open {
                    let variants: Vec<_> = mk.variants.iter().filter(|v| (q.is_empty() || mk.name.to_lowercase().contains(&q) || v.file == chosen || logic::variant_matches(v, &q)) && (!only || v.file == chosen || is_fav(&v.file))).collect();
                    let sel = variants.iter().position(|v| v.file == chosen);
                    let mut options: Vec<String> = variants.iter().map(|v| logic::variant_option(v)).collect();
                    let offset = if sel.is_none() {
                        options.insert(0, tr("Choose a bus"));
                        1
                    } else {
                        0
                    };
                    let files: Vec<String> = variants.iter().map(|v| v.file.clone()).collect();
                    let types = ui.column(self.list);
                    ui.style(types, |s| s.padding = taffy::Rect { left: lp(40.0), right: lp(8.0), top: lp(2.0), bottom: lp(8.0) });
                    kit::gap(ui, types, 6.0);
                    kit::text(ui, types, &tr("Type / variant"), "faint");
                    let tr_ = ui.row(types);
                    kit::gap(ui, tr_, 6.0);
                    let key = mk.key.clone();
                    let s = ui.add(tr_, Select::new(options, Some(sel.unwrap_or(0))).on_change(move |i| b(BusMsg::Type(key.clone(), i))));
                    kit::grow(ui, s);
                    if let Some(v) = selected {
                        ui.set_tooltip(s, Some(format!("{}\n{}\n{}", v.name, v.file, logic::liveries_text(v.paints))));
                        let on = is_fav(&v.file);
                        let star = ui.add(tr_, Button::new("").icon("star").class("ghost"));
                        ui.on_click(star, b(BusMsg::Star(v.file.clone())));
                        ui.set_tooltip(star, Some(tr(if on { "Remove from the favourites" } else { "Add to the favourites" })));
                        if on {
                            ui.visual(star, Visual::new().color(super::super::theme::ACCENT));
                        }
                    }
                    self.type_files = Some((mk.key.clone(), files, offset));
                }
            }
            if let Some(r) = scroll_to {
                ui.defer(move |ui| ui.scroll_into_view(r));
            }
        }
        // the livery
        let vehicle = l.state.bus().cloned();
        ui.set_visible(self.paint_row, vehicle.is_some());
        ui.set_visible(self.settings_toggle, vehicle.is_some());
        ui.set_visible(self.settings, vehicle.is_some() && self.settings_open);
        let Some(vehicle) = vehicle else { return };
        let paints: Vec<String> = std::iter::once(logic::default_livery_label(&vehicle).to_string()).chain(vehicle.paints.iter().cloned()).collect();
        let sel = vehicle.paints.iter().position(|p| *p == l.state.choice.paint).map(|i| i + 1).unwrap_or(0);
        ui.set_select(self.paint, Some(paints.clone()), Some(sel));
        ui.set_text(self.paint_count, &if paints.len() > 1 { format!("{} / {}", sel + 1, paints.len()) } else { String::new() });
        ui.set_visible(self.paint_prev, paints.len() > 1);
        ui.set_visible(self.paint_next, paints.len() > 1);
        let chev = if self.settings_open { "expand_less" } else { "expand_more" };
        ui.update::<Button>(self.settings_toggle, |bt| bt.icon.as_deref() != Some(chev) && { bt.icon = Some(chev.into()); true });
        if self.settings_open {
            let auto = l.state.default_hof();
            let mut hofs = vec![format!("{} ({auto})", tr("Automatic"))];
            hofs.extend(vehicle.hofs.iter().cloned());
            let hs = if l.state.choice.hof_manual { vehicle.hofs.iter().position(|h| h.eq_ignore_ascii_case(&l.state.choice.hof)).map(|i| i + 1).unwrap_or(0) } else { 0 };
            ui.set_select(self.hof, Some(hofs), Some(hs));
            let numbers: Vec<String> = vehicle.numbers.iter().map(|(n, p)| if p.trim().is_empty() { n.clone() } else { format!("{n}  ({})", p.trim()) }).collect();
            ui.set_visible(self.number_row, !numbers.is_empty());
            let ns = vehicle.numbers.iter().position(|(n, _)| *n == l.state.choice.number).unwrap_or(0);
            ui.set_select(self.number, Some(numbers), Some(ns));
            if self.plate_shown != l.state.choice.plate && ui.focused() != Some(self.plate) {
                self.plate_shown = l.state.choice.plate.clone();
                let v = self.plate_shown.clone();
                ui.with::<TextInput, _>(self.plate, |t| t.set(&v));
            }
            let warn = if vehicle.missing_packs.is_empty() { String::new() } else { omsi_ui::tr("Parts missing: needs %{packs}").replace("%{packs}", &vehicle.missing_packs.join(", ")) };
            ui.set_text(self.warn, &warn);
            ui.set_visible(self.warn, !warn.is_empty());
            let description = vehicle.description.replace('\t', " ").lines().map(str::trim).collect::<Vec<_>>().join("\n").trim().to_string();
            ui.set_text(self.description, &description);
            ui.set_visible(self.description, !description.is_empty());
            ui.set_text(self.file, &vehicle.file);
        }
    }
}

fn hash(s: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

pub(super) fn handle(l: &mut Launcher, msg: BusMsg) {
    let ui_step = |l: &mut Launcher, f: &mut dyn FnMut(&mut BusStep)| {
        if let Some(d) = l.gui.as_mut().and_then(|g| g.drive_mut()) {
            f(d.bus_mut());
        }
    };
    match msg {
        BusMsg::Filter(s) => ui_step(l, &mut |st| {
            st.filter = s.clone();
            let q = omsi_launcher_lib::display_bus_name(s.trim()).to_lowercase();
            if !q.is_empty() {
                st.expanded = st.makers.iter().find(|mk| logic::manufacturer_matches(mk, &q)).map(|mk| mk.key.clone());
            }
        }),
        BusMsg::OnlyFavourites(v) => ui_step(l, &mut |st| st.only_favourites = v),
        BusMsg::Family(k) => ui_step(l, &mut |st| st.expanded = if st.expanded.as_deref() == Some(k.as_str()) { None } else { Some(k.clone()) }),
        BusMsg::Pick(f) => l.state.select_bus(&f),
        BusMsg::Type(key, i) => {
            let file = l.gui.as_mut().and_then(|g| g.drive_mut()).and_then(|d| d.bus_mut().type_files.clone()).filter(|t| t.0 == key).and_then(|(_, files, offset)| i.checked_sub(offset).and_then(|k| files.get(k).cloned()));
            if let Some(f) = file {
                l.state.select_bus(&f);
            }
        }
        BusMsg::Star(file) => ui_step(l, &mut |st| {
            let f = st.favourites.get_or_insert_with(logic::read_favourites);
            let k = logic::fav_key(&file);
            if !f.remove(&k) {
                f.insert(k);
            }
            logic::write_favourites(f);
            st.list_key = 0;
        }),
        BusMsg::Paint(i) => {
            if let Some(v) = l.state.bus().cloned() {
                l.state.choice.paint = if i == 0 { String::new() } else { v.paints.get(i - 1).cloned().unwrap_or_default() };
                l.state.touched();
            }
        }
        BusMsg::PaintStep(d) => {
            if let Some(v) = l.state.bus().cloned() {
                let n = v.paints.len() + 1;
                let cur = v.paints.iter().position(|p| *p == l.state.choice.paint).map(|i| i + 1).unwrap_or(0);
                let next = (cur as i32 + d).rem_euclid(n as i32) as usize;
                l.state.choice.paint = if next == 0 { String::new() } else { v.paints[next - 1].clone() };
                l.state.touched();
            }
        }
        BusMsg::Settings => ui_step(l, &mut |st| st.settings_open = !st.settings_open),
        BusMsg::Hof(i) => {
            if let Some(v) = l.state.bus().cloned() {
                l.state.choice.hof_manual = i != 0;
                l.state.choice.hof = if i == 0 { l.state.default_hof() } else { v.hofs.get(i - 1).cloned().unwrap_or_default() };
                l.state.touched();
            }
        }
        BusMsg::Number(i) => {
            if let Some(v) = l.state.bus().cloned() {
                if let Some((n, _)) = v.numbers.get(i) {
                    l.state.choice.number = n.clone();
                    l.state.touched();
                }
            }
        }
        BusMsg::Plate(s) => {
            l.state.choice.plate = s;
            l.state.touched();
        }
    }
}
