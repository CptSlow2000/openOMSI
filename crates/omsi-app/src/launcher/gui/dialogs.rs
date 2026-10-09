//! What lies over the page: the update offer and its progress, a game the server ended or
//! that crashed, the question before every setting is reset, the banner while a game runs,
//! and "Updated to …" in the corner after an update. One dialog at a time, on the overlay
//! layer behind a scrim that takes the pointer from the page; built anew when another one is
//! due, kept in step (a download's progress) while it stays.

use super::kit::{self, len, lp, tr};
use super::theme::{ACCENT, DANGER, WARN};
use super::Msg as Top;
use crate::launcher::update::{crash_issue_url, crash_report, mb};
use crate::launcher::{Launcher, Page};
use crate::updater::{self, Status};
use egui_retained::widgets::{Button, Checkbox, Icon, Progress};
use egui_retained::{Color32, Layer, NodeId, Ui, Visual, taffy};
use omsi_launcher_lib as core;

#[derive(Clone, Debug)]
pub(in crate::launcher) enum Msg {
    UpdateNow,
    UpdateLater,
    UpdateAuto(bool),
    WhatsNew,
    UpdateRetry,
    UpdateGithub,
    DisconnectClose,
    CrashClose,
    CrashCopy,
    CrashDx12,
    CrashSettings,
    CrashIssue,
    ResetNo,
    ResetYes,
}

fn m(x: Msg) -> Top {
    Top::Dialog(x)
}

/// Which dialog, by what makes it another one.
#[derive(Clone, PartialEq, Debug)]
enum Kind {
    Offer(String),
    Downloading(String),
    Installing(String),
    WaitingForInstaller(String),
    Failed(String),
    Disconnected(String),
    Crash(String, String),
    Reset,
    Banner,
}

pub(in crate::launcher) struct Dialogs {
    scrim: NodeId,
    kind: Option<Kind>,
    /// The parts a dialog changes while it stays: a download's words and bar, the switch.
    detail: NodeId,
    progress: NodeId,
    auto: NodeId,
    notice: NodeId,
    notice_text: NodeId,
}

fn kind(l: &Launcher) -> Option<Kind> {
    if l.update_dialog_open() {
        return match l.update.status() {
            Status::Available(r) => Some(Kind::Offer(r.version)),
            Status::Downloading { release, .. } => Some(Kind::Downloading(release.version)),
            Status::Installing(r) | Status::Restarting(r) => Some(Kind::Installing(r.version)),
            Status::WaitingForInstaller(r) => Some(Kind::WaitingForInstaller(r.version)),
            Status::Failed(e) => Some(Kind::Failed(e)),
            _ => None,
        };
    }
    if let Some(why) = &l.state.disconnected {
        return Some(Kind::Disconnected(why.clone()));
    }
    if let Some((what, tail)) = &l.state.crash {
        return Some(Kind::Crash(what.clone(), tail.clone()));
    }
    if l.pages.confirm_reset {
        return Some(Kind::Reset);
    }
    if !crate::launcher::mobile::mobile() && l.state.in_game() && !l.awake() {
        return Some(Kind::Banner);
    }
    None
}

/// The dialog's card: its icon and title at the top.
fn card(ui: &mut Ui, scrim: NodeId, icon: &str, color: Color32, title: &str, width: f32) -> NodeId {
    let c = ui.column(scrim);
    ui.add_class(c, "dialog");
    ui.style(c, |s| {
        s.size.width = len(width);
        s.max_size.width = taffy::Dimension::percent(0.94);
        s.max_size.height = taffy::Dimension::percent(0.92);
        s.padding = taffy::Rect { left: lp(24.0), right: lp(24.0), top: lp(20.0), bottom: lp(20.0) };
        s.gap = taffy::Size { width: lp(12.0), height: lp(12.0) };
    });
    let head = ui.row(c);
    kit::gap(ui, head, 12.0);
    let i = ui.add(head, Icon::new(icon));
    ui.visual(i, Visual::new().font_size(26.0_f32).color(color));
    let t = kit::text(ui, head, &tr(title), "strong");
    ui.visual(t, Visual::new().font_size(18.0_f32));
    c
}

/// The row of buttons at the foot: `left` at the left, `right` at the right, each (text,
/// icon, class, message).
fn buttons(ui: &mut Ui, c: NodeId, left: &[(&str, &str, &'static str, Msg)], right: &[(&str, &str, &'static str, Msg)]) {
    let r = ui.row(c);
    kit::gap(ui, r, 10.0);
    ui.style(r, |s| s.margin.top = taffy::LengthPercentageAuto::length(6.0));
    for (t, i, class, msg) in left {
        let b = ui.add(r, Button::new(tr(t)).icon(*i).class(class));
        ui.on_click(b, m(msg.clone()));
    }
    ui.spacer(r);
    for (t, i, class, msg) in right {
        let b = ui.add(r, Button::new(tr(t)).icon(*i).class(class));
        ui.on_click(b, m(msg.clone()));
    }
}

impl Dialogs {
    pub fn build(ui: &mut Ui) -> Dialogs {
        let over = ui.root(Layer::Overlay);
        let scrim = ui.column(over);
        ui.add_class(scrim, "scrim");
        ui.style(scrim, |s| {
            s.position = taffy::Position::Absolute;
            s.inset = taffy::Rect { left: kit::lpa(0.0), right: kit::lpa(0.0), top: kit::lpa(0.0), bottom: kit::lpa(0.0) };
            s.align_items = Some(taffy::AlignItems::Center);
            s.justify_content = Some(taffy::JustifyContent::Center);
        });
        ui.visual(scrim, Visual::new().background(Color32::from_black_alpha(158)));
        ui.set_visible(scrim, false);
        // "Updated to …": a word in the top right corner, asking nothing
        let notice = ui.row(over);
        ui.add_class(notice, "card");
        ui.style(notice, |s| {
            s.position = taffy::Position::Absolute;
            s.inset = taffy::Rect { left: taffy::LengthPercentageAuto::auto(), right: kit::lpa(20.0), top: kit::lpa(58.0), bottom: taffy::LengthPercentageAuto::auto() };
            s.padding = taffy::Rect { left: lp(14.0), right: lp(18.0), top: lp(10.0), bottom: lp(10.0) };
            s.gap = taffy::Size { width: lp(10.0), height: lp(0.0) };
        });
        ui.visual(notice, Visual::new().border(egui_retained::epaint::Stroke::new(1.0, ACCENT.gamma_multiply(0.6))));
        let i = ui.add(notice, Icon::new("check_circle"));
        ui.visual(i, Visual::new().color(ACCENT).font_size(20.0_f32));
        let notice_text = kit::text(ui, notice, "", "strong");
        ui.set_visible(notice, false);
        let none = NodeId::dangling();
        Dialogs { scrim, kind: None, detail: none, progress: none, auto: none, notice, notice_text }
    }

    pub fn sync(&mut self, ui: &mut Ui, l: &Launcher) {
        // the notice after an update, for 9 seconds
        let updated = l.update.updated.as_ref().filter(|(_, at)| at.elapsed().as_secs_f32() < 9.0);
        ui.set_visible(self.notice, updated.is_some());
        if let Some((v, _)) = updated {
            ui.set_text(self.notice_text, &format!("{} {v}", tr("Updated to openOMSI")));
        }
        let k = kind(l);
        if k != self.kind {
            self.kind = k.clone();
            ui.clear(self.scrim);
            let none = NodeId::dangling();
            (self.detail, self.progress, self.auto) = (none, none, none);
            if let Some(k) = &k {
                self.build_kind(ui, l, k);
            }
            ui.set_visible(self.scrim, k.is_some());
        }
        // what changes while a dialog stays
        if let Status::Downloading { done, total, .. } = l.update.status() {
            if matches!(self.kind, Some(Kind::Downloading(_))) {
                ui.set_text(self.detail, &format!("{} of {} from github.com/{}", mb(done), mb(total), updater::REPO));
                ui.set_progress(self.progress, Some(if total > 0 { done as f32 / total as f32 } else { 0.0 }));
            }
        }
        if matches!(self.kind, Some(Kind::Offer(_))) {
            ui.set_checked(self.auto, l.state.settings.get("update_auto").and_then(|v| v.as_bool()).unwrap_or(false));
        }
    }

    fn build_kind(&mut self, ui: &mut Ui, l: &Launcher, k: &Kind) {
        let s = self.scrim;
        let current = updater::current_version();
        match k {
            Kind::Offer(v) => {
                let size = match l.update.status() {
                    Status::Available(r) => r.size,
                    _ => 0,
                };
                let c = card(ui, s, "system_update", ACCENT, &format!("openOMSI {v} is available"), 560.0);
                let text = if cfg!(target_os = "android") {
                    format!("You have {current}. Update now? The launcher downloads the new version ({}) from GitHub and Android installs it; openOMSI then starts again - your mods and settings stay as they are.", mb(size))
                } else {
                    format!("You have {current}. Update now? The launcher downloads the new version ({}) from GitHub, puts it in place of this one and starts again - your mods and settings stay as they are.", mb(size))
                };
                kit::para(ui, c, &text, "dim");
                self.auto = ui.add(c, Checkbox::switch(false, tr("Install updates without asking from now on")).on_change(|v| m(Msg::UpdateAuto(v))));
                buttons(ui, c, &[("What's new", "open_in_new", "ghost", Msg::WhatsNew)], &[("Not now", "", "button", Msg::UpdateLater), ("Update now", "download", "primary", Msg::UpdateNow)]);
            }
            Kind::Downloading(v) => {
                let c = card(ui, s, "download", ACCENT, &format!("Downloading openOMSI {v}"), 560.0);
                self.detail = kit::para(ui, c, "", "dim");
                self.progress = ui.add(c, Progress { value: Some(0.0) });
            }
            Kind::Installing(v) => {
                let c = card(ui, s, "install_desktop", ACCENT, &format!("Installing openOMSI {v}"), 560.0);
                kit::para(ui, c, &tr("The new version is put in place; the launcher starts again in a moment."), "dim");
                ui.add(c, Progress { value: Some(1.0) });
            }
            Kind::WaitingForInstaller(v) => {
                let c = card(ui, s, "install_mobile", ACCENT, &format!("Installing openOMSI {v}"), 560.0);
                kit::para(ui, c, &tr("Android asks whether to update openOMSI: press Update there. The app then starts again by itself."), "dim");
                ui.add(c, Progress { value: Some(1.0) });
            }
            Kind::Failed(e) => {
                let c = card(ui, s, "error", DANGER, "Not updated", 560.0);
                kit::para(ui, c, e, "dim");
                buttons(ui, c, &[("Open on GitHub", "open_in_new", "ghost", Msg::UpdateGithub)], &[("Try again", "refresh", "button", Msg::UpdateRetry), ("Close", "", "button", Msg::UpdateLater)]);
            }
            Kind::Disconnected(why) => {
                let c = card(ui, s, "error", DANGER, "Disconnected from the server", 560.0);
                kit::para(ui, c, &tr("The server ended your game. Its message:"), "dim");
                let w = kit::para(ui, c, why, "strong");
                ui.visual(w, Visual::new().font_size(14.0_f32));
                buttons(ui, c, &[], &[("Close", "", "primary", Msg::DisconnectClose)]);
            }
            Kind::Crash(what, tail) => {
                let (lost, silent, hint) = crash_hint(what, tail);
                let title = if silent { "The game was closed by the system" } else { "The game closed on an error" };
                let c = card(ui, s, if silent { "info" } else { "error" }, if silent { WARN } else { DANGER }, title, 640.0);
                let body = ui.column(c);
                ui.set_scroll(body, egui_retained::ScrollAxes { x: false, y: true });
                ui.style(body, |st| {
                    st.max_size.height = len(360.0);
                    st.flex_shrink = 1.0;
                    st.gap = taffy::Size { width: lp(8.0), height: lp(8.0) };
                });
                kit::para(ui, body, what, "strong");
                kit::para(ui, body, &tr(hint), "dim");
                let api = l.state.settings.get("graphics_api").and_then(|v| v.as_str()).unwrap_or("auto");
                let mut left: Vec<(&str, &str, &'static str, Msg)> = Vec::new();
                if silent {
                    left.push(("Settings", "tune", "ghost", Msg::CrashSettings));
                } else {
                    left.push(("Report on GitHub", "open_in_new", "ghost", Msg::CrashIssue));
                }
                if lost && cfg!(windows) && api != "dx12" {
                    left.push(("Use DirectX 12", "monitor", "button", Msg::CrashDx12));
                }
                buttons(ui, c, &left, &[("Copy report", "content_copy", "primary", Msg::CrashCopy), ("Close", "", "button", Msg::CrashClose)]);
            }
            Kind::Reset => {
                let c = card(ui, s, "restart_alt", DANGER, "Reset every setting?", 520.0);
                kit::para(ui, c, &tr("Graphics, sound, controllers and game settings go back to how they came. The language, the drivers, the key bindings and the game folder stay."), "dim");
                buttons(ui, c, &[], &[("Cancel", "", "button", Msg::ResetNo), ("Reset", "restart_alt", "danger", Msg::ResetYes)]);
            }
            Kind::Banner => {
                let c = card(ui, s, "directions_bus", ACCENT, "The game is running", 520.0);
                kit::para(ui, c, &tr("The launcher rests while you drive, so that the game has the graphics card to itself. It is back as soon as the game ends."), "dim");
            }
        }
    }
}

/// Of a crash: whether the graphics device was lost, whether the system closed the game, and
/// what to do about it.
fn crash_hint(what: &str, tail: &str) -> (bool, bool, &'static str) {
    let lost = what.contains("graphics device was lost");
    let silent = what.contains("closed without a word");
    let compiling = silent && crate::launcher::update::died_compiling_on_vulkan(tail);
    let hint = if lost {
        if cfg!(windows) {
            "The graphics driver stopped the game. Updating the graphics driver usually helps; you can also let the game draw with DirectX 12 instead of Vulkan (the button below, or Settings → Graphics API)."
        } else {
            "The graphics driver stopped the game. Updating the graphics driver usually helps; Settings → Graphics API can switch to OpenGL."
        }
    } else if compiling {
        "The Vulkan graphics driver stopped while compiling shaders. Starting the game again will switch to OpenGL (or change it in Settings → Graphics API)."
    } else if silent {
        "The system closed the game while it was running, typically because the device ran out of memory (RAM). Lowering texture resolution or reducing AI traffic in Settings helps prevent memory exhaustion."
    } else {
        "Copy the report (the end of the game's log), or open a GitHub issue with it: it tells what went wrong on this computer."
    };
    (lost, silent, hint)
}

pub(in crate::launcher) fn handle(l: &mut Launcher, msg: Msg) {
    let copy = |l: &mut Launcher, text: String| {
        if let Some(c) = l.clipboard.as_mut() {
            let _ = c.set_text(text);
        }
    };
    match msg {
        Msg::UpdateNow => {
            if let Status::Available(r) = l.update.status() {
                l.update.install(r);
            }
        }
        Msg::UpdateLater => l.update.dismiss(),
        Msg::UpdateAuto(v) => {
            l.state.settings["update_auto"] = serde_json::json!(v);
            l.state.settings_dirty = 0.3;
        }
        Msg::WhatsNew => {
            if let Status::Available(r) = l.update.status() {
                updater::open_url(&r.page);
            }
        }
        Msg::UpdateRetry => l.update.check(),
        Msg::UpdateGithub => updater::open_url(&format!("{}/releases/latest", updater::REPO_URL)),
        Msg::DisconnectClose => l.state.disconnected = None,
        Msg::CrashClose => l.state.crash = None,
        Msg::CrashCopy => {
            if let Some((what, tail)) = l.state.crash.clone() {
                copy(l, crash_report(&what, &tail));
                l.state.set_status("The report is copied: paste it into a GitHub issue or a message.", false);
            }
        }
        Msg::CrashIssue => {
            if let Some((what, tail)) = l.state.crash.clone() {
                copy(l, crash_report(&what, &tail));
                updater::open_url(&crash_issue_url(&what, &tail));
            }
        }
        Msg::CrashDx12 => {
            l.state.settings["graphics_api"] = serde_json::json!("dx12");
            l.state.settings_dirty = 0.3;
            l.state.crash = None;
            l.state.set_status("The game draws with DirectX 12 from the next start (Settings → Graphics API to change it back).", false);
        }
        Msg::CrashSettings => {
            l.state.crash = None;
            l.go(Page::Settings);
        }
        Msg::ResetNo => l.pages.confirm_reset = false,
        Msg::ResetYes => {
            let language = l.state.settings.get("language").cloned();
            l.state.settings = core::settings_from_text(None);
            if let Some(lang) = language {
                l.state.settings["language"] = lang;
            }
            l.state.settings_dirty = 0.3;
            l.pages.confirm_reset = false;
            l.state.set_status("Every setting is back to how it came.", false);
        }
    }
}
