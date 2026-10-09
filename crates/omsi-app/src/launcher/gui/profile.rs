//! The Profile page: the driver whose personnel file the game writes - chosen, created or
//! deleted -, the level with its progress, the figures, and the recent runs.

use super::kit::{self, len, lp, tr};
use super::theme::ACCENT;
use super::Msg as Top;
use crate::launcher::state::short_map;
use crate::launcher::Launcher;
use egui_retained::widgets::{Button, Icon, Progress, Select, Text, TextInput};
use egui_retained::{Color32, Element, MeasureCx, NodeId, PaintCx, ScrollAxes, Ui, Vec2, taffy};
use omsi_launcher_lib as core;

#[derive(Clone, Debug)]
pub(in crate::launcher) enum Msg {
    Pick(usize),
    Delete,
    NewName(String),
    Create,
}

fn p(x: Msg) -> Top {
    Top::Profile(x)
}

pub(in crate::launcher) struct ProfilePage {
    pub root: NodeId,
    new_name: String,
    confirm_delete: Option<std::time::Instant>,
    driver: NodeId,
    delete: NodeId,
    name_field: NodeId,
    empty: NodeId,
    stats_card: NodeId,
    ring: NodeId,
    title: NodeId,
    bar: NodeId,
    xp: NodeId,
    stats: Vec<(NodeId, NodeId)>,
    runs: NodeId,
    runs_key: String,
}

/// The level on a ring filled as far as the next level is.
pub struct Ring {
    pub level: u32,
    pub frac: f32,
}

impl Element for Ring {
    fn measure(&mut self, _cx: &mut MeasureCx<'_>, _k: [Option<f32>; 2], _a: [Option<f32>; 2]) -> Vec2 {
        Vec2::splat(104.0)
    }
    fn paint(&mut self, cx: &mut PaintCx<'_>) {
        let c = cx.content.center();
        let stroke = egui_retained::epaint::Stroke::new(8.0, Color32::from_white_alpha(20));
        cx.painter.arc(c, 46.0, 0.0, std::f32::consts::TAU, stroke);
        let a0 = -std::f32::consts::FRAC_PI_2;
        cx.painter.arc(c, 46.0, a0, a0 + std::f32::consts::TAU * self.frac.max(0.01), egui_retained::epaint::Stroke::new(8.0, ACCENT));
        let mut look = cx.look.clone();
        look.font_size = 32.0;
        let g = egui_retained::layout_text(cx.fonts, cx.ppp, &self.level.to_string(), look.font_id(cx.theme), cx.look.color, None);
        cx.painter.galley(c - g.size() * 0.5 - Vec2::new(0.0, 6.0), g, cx.look.color);
        look.font_size = 10.0;
        let g = egui_retained::layout_text(cx.fonts, cx.ppp, &tr("LEVEL"), look.font_id(cx.theme), cx.look.color.gamma_multiply(0.6), None);
        cx.painter.galley(c + Vec2::new(-g.size().x * 0.5, 14.0), g, cx.look.color);
    }
    fn hit_test(&self) -> bool {
        false
    }
}

impl ProfilePage {
    pub fn build(ui: &mut Ui, host: NodeId) -> ProfilePage {
        let root = ui.column(host);
        ui.add_class(root, "content");
        kit::grow(ui, root);
        ui.set_scroll(root, ScrollAxes { x: false, y: true });
        kit::page_title(ui, root, "Profile", "The driver whose personnel file the game writes: hours, kilometres, punctuality, tickets.");
        let cols = ui.row(root);
        kit::gap(ui, cols, 14.0);
        ui.style(cols, |s| s.align_items = Some(taffy::AlignItems::Stretch));
        let left = ui.column(cols);
        kit::gap(ui, left, 14.0);
        ui.style(left, |s| {
            s.flex_grow = 1.4;
            s.flex_basis = len(0.0);
        });
        // the driver
        let c = kit::titled_card(ui, left, "Driver");
        let r = ui.row(c);
        kit::gap(ui, r, 8.0);
        let driver = ui.add(r, Select::new(Vec::new(), None).on_change(|i| p(Msg::Pick(i))));
        kit::grow(ui, driver);
        let delete = ui.add(r, Button::new(tr("Delete this driver")).icon("delete").class("danger"));
        ui.on_click(delete, p(Msg::Delete));
        let r = ui.row(c);
        kit::gap(ui, r, 8.0);
        let name_field = ui.add(r, TextInput::new("").hint(tr("New driver's name")).on_change(|s| p(Msg::NewName(s))).on_submit(|_| p(Msg::Create)));
        kit::grow(ui, name_field);
        let create = ui.add(r, Button::new(tr("Create")).icon("add"));
        ui.on_click(create, p(Msg::Create));
        // level and figures
        let empty = kit::card(ui, left);
        kit::para(ui, empty, &tr("Create a driver to start a personnel file."), "dim");
        let stats_card = kit::card(ui, left);
        let head = ui.row(stats_card);
        kit::gap(ui, head, 18.0);
        let ring = ui.add(head, Ring { level: 1, frac: 0.0 });
        ui.add_class(ring, "strong");
        let who = ui.column(head);
        kit::grow(ui, who);
        kit::gap(ui, who, 8.0);
        let title = kit::text(ui, who, "", "title");
        let bar = ui.add(who, Progress { value: Some(0.0) });
        ui.style(bar, |s| s.size.height = len(8.0));
        let xp = kit::text(ui, who, "", "dim");
        let grid = ui.row(stats_card);
        ui.style(grid, |s| {
            s.flex_wrap = taffy::FlexWrap::Wrap;
            s.gap = taffy::Size { width: lp(10.0), height: lp(10.0) };
            s.margin.top = taffy::LengthPercentageAuto::length(8.0);
        });
        let icons = ["schedule", "route", "location_on", "timer", "confirmation_number", "payments", "warning", "person", "speed", "airport_shuttle", "receipt_long", "history"];
        let stats = icons
            .iter()
            .map(|icon| {
                let tile = ui.column(grid);
                ui.style(tile, |s| {
                    s.size.width = taffy::Dimension::percent(0.31);
                    s.flex_grow = 1.0;
                    s.padding = taffy::Rect::length(12.0_f32);
                    s.gap = taffy::Size { width: lp(4.0), height: lp(4.0) };
                });
                ui.visual(tile, egui_retained::Visual::new().background(Color32::from_white_alpha(6)).radius(10.0_f32));
                let top = ui.row(tile);
                kit::gap(ui, top, 8.0);
                let i = ui.add(top, Icon::new(*icon));
                ui.visual(i, egui_retained::Visual::new().color(ACCENT));
                let v = kit::text(ui, top, "", "strong");
                ui.visual(v, egui_retained::Visual::new().font_size(18.0_f32));
                let l = kit::text(ui, tile, "", "faint");
                (v, l)
            })
            .collect();
        // recent runs
        let right = kit::titled_card(ui, cols, "Recent runs");
        ui.style(right, |s| {
            s.flex_grow = 1.0;
            s.flex_basis = len(0.0);
        });
        let runs = ui.column(right);
        kit::gap(ui, runs, 6.0);
        ProfilePage { root, new_name: String::new(), confirm_delete: None, driver, delete, name_field, empty, stats_card, ring, title, bar, xp, stats, runs, runs_key: String::new() }
    }

    pub fn sync(&mut self, ui: &mut Ui, l: &Launcher) {
        let names = l.state.profiles.clone();
        let sel = names.iter().position(|n| *n == l.state.config.profile);
        ui.set_select(self.driver, Some(names), sel);
        let armed = self.confirm_delete.is_some_and(|t| t.elapsed().as_secs() < 4);
        ui.set_text(self.delete, &tr(if armed { "Click again to delete" } else { "Delete this driver" }));
        let profile = l.state.profile.clone();
        ui.set_visible(self.empty, profile.is_none());
        ui.set_visible(self.stats_card, profile.is_some());
        let Some(pr) = profile else { return };
        let prev = ((pr.level - 1) * (pr.level - 1) * 250) as f64;
        let frac = ((pr.xp as f64 - prev) / (pr.next_level_xp as f64 - prev).max(1.0)).clamp(0.0, 1.0) as f32;
        ui.update::<Ring>(self.ring, |r| {
            let ch = r.level != pr.level as u32 || r.frac != frac;
            r.level = pr.level as u32;
            r.frac = frac;
            ch
        });
        ui.set_text(self.title, &format!("{}{}", pr.name, if pr.exists { "" } else { " (no personnel file yet)" }));
        ui.set_progress(self.bar, Some(frac));
        ui.set_text(self.xp, &format!("{} XP · {} to level {}", pr.xp, (pr.next_level_xp - pr.xp).max(0), pr.level + 1));
        let hours = |h: f64| format!("{} h {:02} min", h.floor() as i64, ((h - h.floor()) * 60.0).round() as i64);
        let values = [
            (hours(pr.hours), "hours driven"),
            (format!("{:.1} km", pr.km), "distance"),
            (pr.stops.to_string(), "stops served"),
            (format!("{} / {}", pr.early, pr.late), "early / late"),
            (format!("{:.0}", pr.tickets), "tickets sold"),
            (format!("{:.2}", pr.cash), "takings"),
            (pr.crashes.to_string(), "crashes"),
            (pr.hurt.to_string(), "pedestrians hurt"),
            (format!("{:.0} %", pr.rating_driving), "driving"),
            (format!("{:.0} %", pr.rating_comfort), "comfort"),
            (format!("{:.0} %", pr.rating_tickets), "ticket selling"),
            (pr.sessions.len().to_string(), "runs"),
        ];
        for ((v, l), (value, label)) in self.stats.iter().zip(values) {
            ui.set_text(*v, &value);
            ui.set_text(*l, &tr(label));
        }
        let key = format!("{}|{}", pr.name, pr.sessions.len());
        if key != self.runs_key {
            self.runs_key = key;
            ui.clear(self.runs);
            if pr.sessions.is_empty() {
                kit::para(ui, self.runs, &tr("No runs yet. Drive a duty and it shows up here."), "dim");
            }
            for s in &pr.sessions {
                let row = ui.column(self.runs);
                ui.style(row, |st| {
                    st.padding = taffy::Rect::length(10.0_f32);
                    st.gap = taffy::Size { width: lp(3.0), height: lp(3.0) };
                    st.flex_shrink = 0.0;
                });
                ui.visual(row, egui_retained::Visual::new().background(Color32::from_white_alpha(6)).radius(9.0_f32));
                let top = ui.row(row);
                let title = match &s.line {
                    Some(line) => format!("Line {line}{} · {}", s.tour.as_ref().map(|t| format!(" / {t}")).unwrap_or_default(), short_map(&s.map)),
                    None => format!("Free drive · {}", short_map(&s.map)),
                };
                let t = ui.add(top, Text::new(title));
                ui.add_class(t, "strong");
                kit::grow(ui, t);
                kit::text(ui, top, &crate::launcher::pages::chrono_like(s.time), "dim");
                let bottom = ui.row(row);
                let d = ui.add(bottom, Text::new(format!("{} · {:.1} km · {} stops · {} tickets · {} crashes", s.bus.rsplit('/').next().unwrap_or(""), s.metres / 1000.0, s.stops, s.tickets, s.crashes)));
                ui.add_class(d, "faint");
                kit::grow(ui, d);
                let h = s.seconds / 3600.0;
                kit::text(ui, bottom, &format!("{}:{:02} h", h.floor() as i64, ((h - h.floor()) * 60.0).round() as i64), "accent-text");
            }
        }
        let _ = (self.name_field, &self.new_name);
    }
}

pub(in crate::launcher) fn handle(l: &mut Launcher, m: Msg) {
    fn page(l: &mut Launcher) -> Option<&mut ProfilePage> {
        l.gui.as_mut().and_then(|g| g.profile.as_mut())
    }
    match m {
        Msg::Pick(i) => {
            if let Some(n) = l.state.profiles.get(i).cloned() {
                l.state.config.profile = n;
                let _ = core::save_config(&l.state.config);
                l.state.load_profile();
                l.state.touched();
            }
        }
        Msg::Delete => {
            let armed = page(l).and_then(|p| p.confirm_delete).is_some_and(|t| t.elapsed().as_secs() < 4);
            if armed {
                let name = l.state.config.profile.clone();
                match core::delete_profile(&name) {
                    Ok(()) => {
                        l.state.profile = None;
                        l.state.config.profile.clear();
                        let _ = core::save_config(&l.state.config);
                        l.state.set_status(format!("Personnel file of {name} deleted."), false);
                        l.state.load_profiles();
                    }
                    Err(e) => l.state.set_status(format!("{e:#}"), true),
                }
                if let Some(p) = page(l) {
                    p.confirm_delete = None;
                }
            } else if let Some(p) = page(l) {
                p.confirm_delete = Some(std::time::Instant::now());
            }
        }
        Msg::NewName(s) => {
            if let Some(p) = page(l) {
                p.new_name = s;
            }
        }
        Msg::Create => {
            let name = page(l).map(|p| p.new_name.trim().to_string()).unwrap_or_default();
            if name.is_empty() {
                return;
            }
            match core::create_profile(&name, "M") {
                Ok(_) => {
                    l.state.config.profile = name.clone();
                    let _ = core::save_config(&l.state.config);
                    l.state.load_profiles();
                    l.state.load_profile();
                    l.state.set_status(format!("Driver {name} created."), false);
                    let field = page(l).map(|p| {
                        p.new_name.clear();
                        p.name_field
                    });
                    if let (Some(f), Some(g)) = (field, l.gui.as_mut()) {
                        g.ui.with::<TextInput, _>(f, |t| t.set(""));
                    }
                }
                Err(e) => l.state.set_status(format!("{e:#}"), true),
            }
        }
    }
}
