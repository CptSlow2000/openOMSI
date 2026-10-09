//! The Drive page: three steps as tabs - the bus, the day and the weather, the map and the
//! duty - the step's card on the left, the stage on the right (the bus on its turntable, or
//! the map with the roadbook beside it), and a foot under the stage that sums the choice up
//! and holds the buttons that go on.

mod bus;
mod duty;
mod time;

use super::kit::{self, lp, tr};
use super::stage::{Stage, StagePointer};
use super::Msg as Top;
use crate::launcher::drive as logic;
use crate::launcher::{Launcher, Page};
use egui_retained::widgets::{Button, Icon, Select};
use egui_retained::{EventKind, NodeId, Ui, Vec2, taffy};
use std::cell::RefCell;
use std::rc::Rc;

pub(super) use bus::BusMsg;
pub(super) use duty::DutyMsg;
pub(super) use time::TimeMsg;

const STEPS: [&str; 3] = ["Bus", "Day & weather", "Map & duty"];

#[derive(Clone, Debug)]
pub(in crate::launcher) enum Msg {
    Step(usize),
    Next,
    Launch,
    Continue,
    ContinuePick(usize),
    GoSessions,
    Bus(BusMsg),
    Time(TimeMsg),
    Duty(DutyMsg),
}

fn m(x: Msg) -> Top {
    Top::Drive(x)
}

pub(in crate::launcher) struct DrivePage {
    pub root: NodeId,
    steps: [NodeId; 3],
    panels: [NodeId; 3],
    bus: bus::BusStep,
    time: time::TimeStep,
    duty: duty::DutyStep,
    preview: NodeId,
    preview_ptr: Rc<RefCell<StagePointer>>,
    map: NodeId,
    map_ptr: Rc<RefCell<StagePointer>>,
    book: NodeId,
    foot_icon: NodeId,
    foot_title: NodeId,
    foot_sub: NodeId,
    foot_note: NodeId,
    cont_pick: NodeId,
    cont: NodeId,
    go: NodeId,
}

impl DrivePage {
    pub fn build(ui: &mut Ui, host: NodeId) -> DrivePage {
        let root = ui.column(host);
        ui.add_class(root, "content");
        kit::grow(ui, root);
        // the steps
        let tabs = ui.row(root);
        kit::gap(ui, tabs, 6.0);
        ui.style(tabs, |s| s.flex_shrink = 0.0);
        let steps = std::array::from_fn(|k| {
            let b = ui.add(tabs, Button::new(format!("{}  {}", k + 1, tr(STEPS[k]))).class("tab"));
            ui.style(b, |s| s.padding = taffy::Rect { left: lp(14.0), right: lp(14.0), top: lp(8.0), bottom: lp(8.0) });
            ui.on_click(b, m(Msg::Step(k)));
            ui.set_name(b, &format!("drive-tab-{k}"));
            b
        });
        let body = ui.row(root);
        kit::grow(ui, body);
        kit::gap(ui, body, 14.0);
        ui.style(body, |s| s.align_items = Some(taffy::AlignItems::Stretch));
        // the step's card
        let left = ui.column(body);
        ui.add_class(left, "card");
        ui.style(left, |s| {
            s.size.width = taffy::Dimension::percent(0.33);
            s.min_size.width = kit::len(340.0);
            s.max_size.width = kit::len(460.0);
            s.flex_shrink = 0.0;
            s.min_size.height = kit::len(0.0);
        });
        let panels: [NodeId; 3] = std::array::from_fn(|_| {
            let p = ui.column(left);
            kit::grow(ui, p);
            kit::gap(ui, p, 10.0);
            p
        });
        let bus = bus::BusStep::build(ui, panels[0]);
        let time = time::TimeStep::build(ui, panels[1]);
        let mut duty = duty::DutyStep::build(ui, panels[2]);
        // the stage and its foot
        let right = ui.column(body);
        kit::grow(ui, right);
        kit::gap(ui, right, 12.0);
        let stage_row = ui.row(right);
        kit::grow(ui, stage_row);
        kit::gap(ui, stage_row, 12.0);
        ui.style(stage_row, |s| s.align_items = Some(taffy::AlignItems::Stretch));
        let (st, preview_ptr) = Stage::new(&tr("Loading…"), true);
        let preview = ui.add(stage_row, st);
        kit::grow(ui, preview);
        ui.visual(preview, egui_retained::Visual::new().cursor(egui_retained::Cursor::Grab));
        let (st, map_ptr) = Stage::new(&tr("Loading…"), false);
        let map = ui.add(stage_row, st);
        kit::grow(ui, map);
        let book = duty.build_book(ui, stage_row);
        let foot = ui.row(right);
        ui.add_class(foot, "card");
        ui.style(foot, |s| {
            s.flex_direction = taffy::FlexDirection::Row;
            s.align_items = Some(taffy::AlignItems::Center);
            s.gap = taffy::Size { width: lp(12.0), height: lp(0.0) };
            s.min_size.height = kit::len(72.0);
        });
        let foot_icon = ui.add(foot, Icon::new("directions_bus"));
        ui.visual(foot_icon, egui_retained::Visual::new().font_size(18.0_f32));
        let texts = ui.column(foot);
        kit::grow(ui, texts);
        kit::gap(ui, texts, 2.0);
        let foot_title = kit::text(ui, texts, "", "strong");
        let foot_sub = kit::text(ui, texts, "", "dim");
        let foot_note = kit::text(ui, texts, "", "ok-text");
        ui.on_click(foot_note, m(Msg::GoSessions));
        let cont_pick = ui.add(foot, Select::new(Vec::new(), None).on_change(|i| m(Msg::ContinuePick(i))));
        let cont = ui.add(foot, Button::new(tr("Continue last game")).icon("history"));
        ui.on_click(cont, m(Msg::Continue));
        ui.set_tooltip(cont, Some(tr("Continue where you left off on this map")));
        let go = ui.add(foot, Button::new("").icon("arrow_forward").class("primary"));
        ui.set_name(go, "drive-go");
        ui.on(go, EventKind::Click, |cx, _| {
            cx.emit(Top::Drive(Msg::Next));
            true
        });
        DrivePage { root, steps, panels, bus, time, duty, preview, preview_ptr, map, map_ptr, book, foot_icon, foot_title, foot_sub, foot_note, cont_pick, cont, go }
    }

    pub fn preview_node(&self) -> Option<NodeId> {
        Some(self.preview)
    }

    pub fn map_node(&self) -> Option<NodeId> {
        Some(self.map)
    }

    pub fn sync(&mut self, ui: &mut Ui, l: &Launcher) {
        let tab = l.drive.tab.min(2);
        for k in 0..3 {
            ui.set_selected(self.steps[k], k == tab);
            ui.set_visible(self.panels[k], k == tab);
        }
        match tab {
            0 => self.bus.sync(ui, l),
            1 => self.time.sync(ui, l),
            _ => self.duty.sync(ui, l),
        }
        ui.set_visible(self.preview, tab < 2);
        ui.set_visible(self.map, tab == 2);
        let book = tab == 2 && self.duty.book_open(l);
        ui.set_visible(self.book, book);
        // the foot
        let running = l.state.instances.iter().filter(|i| i.running).count();
        let icon = ["directions_bus", "partly_cloudy_day", "map"][tab];
        ui.update::<Icon>(self.foot_icon, |i| i.name != icon && { i.name = icon.into(); true });
        let (title, sub) = if tab < 2 {
            let bus = l.state.bus().map(|b| omsi_launcher_lib::display_bus_name(&b.name)).unwrap_or_else(|| tr("No bus chosen"));
            let paint = logic::paint_line(l);
            (if paint.is_empty() { bus } else { format!("{bus} · {paint}") }, logic::start_line(l))
        } else {
            let (duty, when) = logic::duty_of(l);
            let place = logic::duty_place(l);
            (duty, if when.is_empty() { place } else { format!("{when} · {place}") })
        };
        ui.set_text(self.foot_title, &title);
        ui.set_text(self.foot_sub, &sub);
        let warn = l.state.bus().filter(|b| !b.missing_packs.is_empty()).map(|b| omsi_ui::tr("Parts missing: needs %{packs}").replace("%{packs}", &b.missing_packs.join(", ")));
        let note = match (&warn, running) {
            (Some(w), _) => w.clone(),
            (None, n) if n > 0 => format!("{n} game{} running - see Sessions", if n > 1 { "s" } else { "" }),
            _ => String::new(),
        };
        ui.set_text(self.foot_note, &note);
        ui.set_visible(self.foot_note, !note.is_empty());
        ui.set_class(self.foot_note, "warn-text", warn.is_some());
        ui.set_class(self.foot_note, "link", warn.is_none());
        // the way back into the last game on this map, and which save when it keeps several
        let has_last = l.state.joined_server.is_none() && l.last_situation_names().is_some();
        let saves = l.last_situation_names().unwrap_or_default();
        ui.set_visible(self.cont, has_last);
        ui.set_visible(self.cont_pick, has_last && saves.len() > 1);
        if saves.len() > 1 {
            ui.set_select(self.cont_pick, Some(saves.clone()), Some(l.state.save_pick.min(saves.len() - 1)));
            ui.set_text(self.cont, &tr("Continue"));
        } else {
            ui.set_text(self.cont, &tr("Continue last game"));
        }
        // the button that goes on: the next step, or into the game
        let label = if tab < 2 {
            format!("{} {}", tr("Next:"), tr(STEPS[tab + 1]))
        } else if running > 0 && l.state.second_armed.map(|t| t.elapsed().as_secs() < 6).unwrap_or(false) {
            tr("Start another game")
        } else if (l.state.choice.free || l.state.choice.line.is_none()) && l.state.joined_server.is_none() {
            tr("Drive")
        } else {
            tr("Start the duty")
        };
        ui.set_text(self.go, &label);
        let go_icon = if tab < 2 { "arrow_forward" } else { "play_arrow" };
        ui.update::<Button>(self.go, |b| b.icon.as_deref() != Some(go_icon) && { b.icon = Some(go_icon.into()); true });
    }

    /// The pointers over the bus and the map since the last frame.
    pub fn pointers(&self) -> (StagePointer, StagePointer) {
        (self.preview_ptr.borrow_mut().take(), self.map_ptr.borrow_mut().take())
    }

    pub(super) fn bus_mut(&mut self) -> &mut bus::BusStep {
        &mut self.bus
    }
    pub(super) fn duty_mut(&mut self) -> &mut duty::DutyStep {
        &mut self.duty
    }
    pub(super) fn time_mut(&mut self) -> &mut time::TimeStep {
        &mut self.time
    }
}

pub(in crate::launcher) fn handle(l: &mut Launcher, msg: Msg) {
    match msg {
        Msg::Step(k) => l.drive.tab = k.min(2),
        Msg::Next => {
            if l.drive.tab < 2 {
                l.drive.tab += 1;
            } else {
                logic::start(l);
            }
        }
        Msg::Launch => logic::start(l),
        Msg::Continue => l.state.launch_last_situation(),
        Msg::ContinuePick(i) => l.state.save_pick = i,
        Msg::GoSessions => l.go(Page::Sessions),
        Msg::Bus(b) => bus::handle(l, b),
        Msg::Time(t) => time::handle(l, t),
        Msg::Duty(d) => duty::handle(l, d),
    }
}

impl Launcher {
    /// The names of the saved games on the chosen map (None: there is none).
    pub(in crate::launcher) fn last_situation_names(&self) -> Option<Vec<String>> {
        let cached = self.state.last_sit.as_ref().filter(|(m, _, _)| *m == self.state.choice.map)?;
        (!cached.1.is_empty()).then(|| cached.1.iter().map(|s| s.name.clone()).collect())
    }

    /// The bus and the map pictures worked by the pointer: the bus turned and zoomed, the map
    /// dragged, zoomed and its entry points clicked.
    pub(in crate::launcher) fn gui_stage(&mut self, bus: StagePointer, map: StagePointer, ppp: f32) {
        if bus.drag != Vec2::ZERO {
            self.showroom.orbit(bus.drag.x, bus.drag.y);
        }
        if bus.wheel != 0.0 {
            self.showroom.zoom_by((1.0 - bus.wheel * 0.08).clamp(0.8, 1.25));
        }
        if self.drive.tab == 2 && map.rect.is_positive() {
            let r = omsi_ui::Rect::new(map.rect.min.x, map.rect.min.y, map.rect.width(), map.rect.height());
            let p = crate::launcher::mapview::Pointer {
                at: map.at.map(|a| glam::Vec2::new(a.x, a.y)).unwrap_or(glam::Vec2::new(-1e4, -1e4)),
                pressed: map.pressed,
                released: map.released,
                down: map.down,
                wheel: map.wheel,
                blocked: map.at.is_none(),
            };
            self.mapview.want(logic::map_look(self));
            self.mapview.think(r, r, ppp, p);
            if let Some(i) = self.mapview.take_clicked() {
                if self.state.choice.entry != i as i32 {
                    log::info!("launcher map: entry point {} of the map's list taken from the map", i + 1);
                    self.state.choice.entry = i as i32;
                    self.state.touched();
                }
            }
        }
    }
}
