//! The Sessions page: the games started from the launcher - running or ended, their log, and
//! of a LAN game the code to give friends, who drives along and what they say.

use super::kit::{self, len, lp, tr};
use super::theme::{ACCENT, DANGER, OK, WARN};
use super::Msg as Top;
use crate::launcher::state::short_map;
use crate::launcher::Launcher;
use egui_retained::widgets::{Button, Icon, Text};
use egui_retained::{Color32, NodeId, ScrollAxes, Ui, Visual, taffy};
use serde_json::Value;

#[derive(Clone, Debug)]
pub(in crate::launcher) enum Msg {
    Stop(u32),
    Log(u32),
}

fn s(x: Msg) -> Top {
    Top::Sessions(x)
}

pub(in crate::launcher) struct SessionsPage {
    pub root: NodeId,
    list: NodeId,
    key: u64,
}

fn ago(t: u64) -> String {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(t);
    let s = now.saturating_sub(t);
    if s < 60 {
        format!("{s} s")
    } else if s < 3600 {
        format!("{} min", s / 60)
    } else {
        format!("{} h {} min", s / 3600, (s / 60) % 60)
    }
}

fn section(ui: &mut Ui, parent: NodeId, t: &str) {
    let n = kit::text(ui, parent, &tr(t), "faint");
    ui.visual(n, Visual::new().font_size(11.0_f32));
    ui.style(n, |st| st.margin.top = taffy::LengthPercentageAuto::length(6.0));
}

impl SessionsPage {
    pub fn build(ui: &mut Ui, host: NodeId) -> SessionsPage {
        let root = ui.column(host);
        ui.add_class(root, "content");
        kit::grow(ui, root);
        ui.set_scroll(root, ScrollAxes { x: false, y: true });
        kit::page_title(ui, root, "Sessions", "The games you started, and who drives with you.");
        let list = ui.column(root);
        kit::gap(ui, list, 12.0);
        SessionsPage { root, list, key: 0 }
    }

    pub fn sync(&mut self, ui: &mut Ui, l: &Launcher) {
        // (drawn again when anything shown changed: the minutes running, a player, a line)
        let names: std::collections::HashMap<String, String> = l.state.vehicles.iter().map(|v| (v.file.clone(), v.name.clone())).collect();
        let short_bus = |b: &str| names.get(b).cloned().unwrap_or_else(|| b.rsplit('/').next().unwrap_or("").trim_end_matches(".bus").to_string());
        let key = {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            for i in &l.state.instances {
                (i.pid, i.running, &i.last_line, i.exit_code, i.killed, i.stopping.is_some(), l.state.stopping.contains(&i.pid), ago(i.started)).hash(&mut h);
                i.lan_status.as_ref().map(|v| v.to_string()).hash(&mut h);
                l.state.open_logs.contains(&i.pid).hash(&mut h);
                l.state.logs.get(&i.pid).map(|v| v.len()).hash(&mut h);
                l.state.logs.get(&i.pid).and_then(|v| v.last().cloned()).hash(&mut h);
            }
            h.finish()
        };
        if key == self.key {
            return;
        }
        self.key = key;
        ui.clear(self.list);
        if l.state.instances.is_empty() {
            let c = kit::card(ui, self.list);
            ui.style(c, |st| {
                st.flex_direction = taffy::FlexDirection::Row;
                st.align_items = Some(taffy::AlignItems::Center);
                st.gap = taffy::Size { width: lp(18.0), height: lp(0.0) };
                st.max_size.width = len(760.0);
            });
            let i = ui.add(c, Icon::new("sports_esports"));
            ui.visual(i, Visual::new().font_size(28.0_f32));
            let t = kit::para(ui, c, &tr("No game is running. Start a duty on the Drive page; to drive with friends, turn on hosting on the Multiplayer page and give them the code shown here."), "dim");
            kit::grow(ui, t);
            return;
        }
        for i in &l.state.instances {
            let lan = i.lan_status.clone().unwrap_or(Value::Null);
            let role = lan.get("role").and_then(|x| x.as_str()).unwrap_or("").to_string();
            let players: Vec<Value> = lan.get("players").and_then(|x| x.as_array()).cloned().unwrap_or_default();
            let chat: Vec<String> = lan.get("chat").and_then(|x| x.as_array()).map(|a| a.iter().filter_map(|c| c.as_str().map(String::from)).collect()).unwrap_or_default();
            let warnings: Vec<String> = lan.get("warnings").and_then(|x| x.as_array()).map(|a| a.iter().filter_map(|c| c.as_str().map(String::from)).collect()).unwrap_or_default();
            let card = kit::card(ui, self.list);
            let head = ui.row(card);
            kit::gap(ui, head, 12.0);
            let dot = ui.add(head, kit::Dot::new(if i.running { OK } else { Color32::from_gray(100) }));
            let _ = dot;
            let texts = ui.column(head);
            kit::grow(ui, texts);
            kit::gap(ui, texts, 2.0);
            let duty = i.line.as_ref().map(|ln| format!(" · line {ln}{}", i.tour.as_ref().map(|t| format!(" / {t}")).unwrap_or_default())).unwrap_or_default();
            let t = kit::text(ui, texts, &format!("{} · {}{duty}", short_map(&i.map), short_bus(&i.bus)), "strong");
            ui.visual(t, Visual::new().font_size(16.0_f32));
            let status = if i.running {
                if l.state.stopping.contains(&i.pid) || i.stopping.is_some() { "stopping - saving the run…".to_string() } else { format!("running for {}", ago(i.started)) }
            } else {
                let how = if i.exit_code == Some(0) {
                    String::new()
                } else if i.killed {
                    " (killed - it did not end by itself, the run is not saved)".into()
                } else {
                    i.exit_code.map(|c| format!(" (exit code {c})")).unwrap_or_default()
                };
                format!("ended{how}")
            };
            kit::text(ui, texts, &format!("{status} · driver {}", i.profile), "dim");
            if !i.last_line.is_empty() {
                kit::text(ui, texts, &i.last_line, "faint");
            }
            let log_open = l.state.open_logs.contains(&i.pid);
            let lb = ui.add(head, Button::new(tr(if log_open { "Hide log" } else { "Show log" })).icon("receipt_long"));
            ui.on_click(lb, s(Msg::Log(i.pid)));
            if i.running {
                let stopping = l.state.stopping.contains(&i.pid);
                let b = ui.add(head, Button::new(tr(if stopping { "Stopping…" } else { "Stop" })).icon("close").class("danger"));
                ui.set_disabled(b, stopping);
                ui.on_click(b, s(Msg::Stop(i.pid)));
            }
            if role == "host" {
                let code = lan.get("code").and_then(|x| x.as_str()).unwrap_or("").to_string();
                let bx = ui.row(card);
                ui.style(bx, |st| {
                    st.padding = taffy::Rect::length(14.0_f32);
                    st.gap = taffy::Size { width: lp(12.0), height: lp(0.0) };
                    st.margin.top = taffy::LengthPercentageAuto::length(4.0);
                });
                ui.visual(bx, Visual::new().background(ACCENT.gamma_multiply(0.12)).border(egui_retained::epaint::Stroke::new(1.0, ACCENT.gamma_multiply(0.5))).radius(10.0_f32));
                let tc = ui.column(bx);
                kit::grow(ui, tc);
                kit::gap(ui, tc, 4.0);
                let h = kit::text(ui, tc, &tr("SESSION CODE"), "accent-text");
                ui.visual(h, Visual::new().font_size(11.0_f32));
                let c = kit::text(ui, tc, &code, "strong");
                ui.visual(c, Visual::new().font_size(20.0_f32).font(egui_retained::epaint::FontFamily::Monospace));
                kit::text(ui, tc, &tr("Your friends paste it into Multiplayer → Connect by Code."), "dim");
                let cb = ui.add(bx, Button::new(tr("Copy code")).icon("content_copy").class("primary"));
                ui.on_click(cb, Top::Copy(code, tr("Session code copied.")));
            } else if role == "client" {
                let connected = lan.get("connected").and_then(|x| x.as_bool()).unwrap_or(false);
                let text = if let Some(rej) = lan.get("rejected").and_then(|x| x.as_str()) {
                    format!("not connected: {rej}")
                } else if connected {
                    format!("connected to {}", lan.get("host_name").and_then(|x| x.as_str()).unwrap_or(""))
                } else {
                    "connecting…".to_string()
                };
                kit::text(ui, card, &format!("Multiplayer: {text}"), if connected { "ok-text" } else { "warn-text" });
            }
            if !players.is_empty() {
                section(ui, card, "PLAYERS");
                for p in &players {
                    let sv = |k: &str| p.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
                    let pax = p.get("passengers").and_then(|x| x.as_i64()).unwrap_or(0);
                    let dest = if sv("destination").is_empty() { String::new() } else { format!(" · {} → {}", sv("line"), sv("destination")) };
                    kit::text(ui, card, &format!("{} · {}{dest}{} · {}", sv("name"), short_bus(&sv("bus")), if pax > 0 { format!(" · {pax} passengers") } else { String::new() }, sv("where")), "");
                }
            }
            if !chat.is_empty() {
                section(ui, card, "CHAT  (V in the game to write)");
                for c in chat.iter().rev().take(6).rev() {
                    kit::text(ui, card, c, "dim");
                }
            }
            for w in &warnings {
                let r = ui.row(card);
                kit::gap(ui, r, 8.0);
                let ic = ui.add(r, Icon::new("warning"));
                ui.visual(ic, Visual::new().color(WARN));
                kit::text(ui, r, w, "warn-text");
            }
            if log_open {
                let lb = ui.column(card);
                ui.style(lb, |st| {
                    st.padding = taffy::Rect::length(10.0_f32);
                    st.gap = taffy::Size { width: lp(2.0), height: lp(2.0) };
                });
                ui.visual(lb, Visual::new().background(Color32::from_rgba_unmultiplied(6, 8, 10, 230)).radius(8.0_f32));
                for line in l.state.logs.get(&i.pid).cloned().unwrap_or_default().iter().rev().take(14).rev() {
                    let n = ui.add(lb, Text::new(line.clone()));
                    ui.visual(n, Visual::new().font_size(11.5_f32).font(egui_retained::epaint::FontFamily::Monospace).color(if line.contains("ERROR") { DANGER } else if line.contains("WARN") { WARN } else { Color32::from_gray(150) }));
                }
            }
        }
    }
}

pub(in crate::launcher) fn handle(l: &mut Launcher, m: Msg) {
    match m {
        Msg::Stop(pid) => l.state.stop(pid),
        Msg::Log(pid) => {
            if !l.state.open_logs.remove(&pid) {
                l.state.open_logs.insert(pid);
                l.state.log_tail(pid);
            }
        }
    }
}
