//! The launcher's side of the updates (see `crate::updater`): the check when it starts,
//! the question, the progress, and the restart.

use super::Launcher;
use crate::updater::{self, Status};
use winit::event_loop::ActiveEventLoop;

impl Launcher {
    fn setting(&self, key: &str, default: bool) -> bool {
        self.state.settings.get(key).and_then(|v| v.as_bool()).unwrap_or(default)
    }

    /// Once a frame (drawn or not): look for an update when the launcher has started, install
    /// one when that is what the player chose, and on a computer hand over to the new
    /// launcher once it is in place.
    pub(super) fn update_tick(&mut self, event_loop: &ActiveEventLoop) {
        let looking = self.setting("update_check", true) && !omsi_cfg::flags::OMSI_NO_UPDATE.is_set();
        if !self.update.checked_once && self.started.elapsed().as_secs_f32() > 1.0 {
            if looking {
                self.update.check();
            } else {
                self.update.checked_once = true;
            }
        }
        // (a game started from here is a program of its own on a computer: nothing is put in
        // its place while it runs - once it ends, the update the game downloaded in the
        // background goes in, without a wait)
        let game_running = !omsi_launcher_lib::IN_PROCESS_GAMES && self.state.instances.iter().any(|i| i.running);
        if self.update.game_was_running && !game_running && looking {
            log::info!("update: the game has ended - looking for an update");
            self.update.dismissed_version = None;
            self.update.auto_started = false;
            if let Status::Failed(_) | Status::UpToDate = self.update.status() {
                self.update.dismiss();
            }
            self.update.check();
        }
        self.update.game_was_running = game_running;
        // and every half hour while the launcher stays open, not only when it starts
        let idle = matches!(self.update.status(), Status::Idle | Status::UpToDate) || (matches!(self.update.status(), Status::Failed(_)) && self.update.dismissed);
        if looking && idle && self.update.last_check.is_some_and(|t| t.elapsed() > std::time::Duration::from_secs(30 * 60)) {
            if let Status::Failed(_) | Status::UpToDate = self.update.status() {
                self.update.dismiss();
            }
            self.update.check();
        }
        self.update.poll();
        // (an offer put aside comes back for a newer version only)
        if let Status::Available(r) = self.update.status() {
            if self.update.dismissed && self.update.dismissed_version.as_deref() != Some(r.version.as_str()) {
                self.update.dismissed = false;
            } else if !self.update.dismissed && self.update.dismissed_version.as_deref() == Some(r.version.as_str()) {
                self.update.dismissed = true;
            }
        }
        match self.update.status() {
            // "install updates without asking" (not while a game runs)
            Status::Available(r) if self.setting("update_auto", false) && !self.update.dismissed && !self.update.auto_started && !game_running => {
                self.update.auto_started = true;
                log::info!("update: installing {} by itself (update_auto)", r.version);
                self.update.install(r);
            }
            Status::Restarting(r) => {
                if !self.update.relaunched {
                    self.update.relaunched = true;
                    match updater::install_place().and_then(|p| updater::relaunch(&p)) {
                        Ok(()) => {
                            log::info!("update: {} installed, the new launcher starts", r.version);
                            event_loop.exit();
                        }
                        Err(e) => self.state.set_status(format!("openOMSI {} is installed; start it again yourself ({e}).", r.version), true),
                    }
                }
            }
            _ => {}
        }
    }

    /// Whether the update dialog lies over the page this frame.
    pub(super) fn update_dialog_open(&self) -> bool {
        // (the launcher rests while a game runs: the offer waits for the session's end)
        if self.update.game_was_running {
            return false;
        }
        match self.update.status() {
            Status::Available(_) | Status::Failed(_) => !self.update.dismissed,
            Status::Downloading { .. } | Status::Installing(_) | Status::WaitingForInstaller(_) | Status::Restarting(_) => true,
            _ => false,
        }
    }

}

impl Launcher {
}

/// The run went down while a Vulkan driver compiled the shaders: the LAST it said was a stage
/// of that, and it drew with Vulkan (as the phone's shell decides it, `android.rs`). Any
/// compile stage anywhere in the log said so of every silent end - a phone run out of memory
/// 75 % into loading a map on OpenGL was told its Vulkan driver had failed (#848).
pub(crate) fn died_compiling_on_vulkan(log: &str) -> bool {
    let last = log.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("");
    let compiling = last.contains("renderer: compiling") || last.contains("cloud noise made") || last.contains("opening graphics device") || last.contains("compiling renderer pipelines");
    let vulkan = log.lines().any(|l| (l.contains("renderer: ") || l.contains("opening graphics device")) && l.contains("(Vulkan")) || log.lines().any(|l| l.contains("graphics: ") && l.to_ascii_uppercase().contains("VULKAN"));
    compiling && vulkan
}

#[cfg(test)]
mod hint_tests {
    #[test]
    fn only_a_run_that_died_compiling_on_vulkan_is_told_so() {
        let compiled = "[t INFO r] opening graphics device: Mali (Vulkan, vendor 0x13b5)\n[t INFO r] renderer: compiling the scene shaders\n";
        assert!(super::died_compiling_on_vulkan(compiled));
        // compiled long ago, then ran out of memory loading
        assert!(!super::died_compiling_on_vulkan(&format!("{compiled}[t INFO g] status: 63 fps, view driver\n[t INFO m] loading tiles 75 %\n")));
        // on OpenGL it is never the Vulkan driver
        assert!(!super::died_compiling_on_vulkan("[t INFO r] opening graphics device: Mali (Gl, vendor 0x13b5)\n[t INFO r] renderer: compiling the scene shaders\n"));
    }
}

impl Launcher {

}

/// A crash's report for the clipboard: the version, the system, what went wrong and the end
/// of the log.
pub(super) fn crash_report(what: &str, tail: &str) -> String {
    format!("openOMSI {} ({})\n{what}\n\n{tail}", updater::current_version(), std::env::consts::OS)
}

/// A new GitHub issue about a crash, filled in: the end of the log goes with it, as much as a
/// link holds (a report of the last line alone said where the game stopped, never what led
/// there); the computer and the map always (see `crash_of`).
pub(super) fn crash_issue_url(what: &str, tail: &str) -> String {
    let title = format!("Crash: {}", what.chars().take(80).collect::<String>());
    let enc = |t: &str| t.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect::<String>();
    let (machine, end) = tail.split_once(&format!("\n{}\n", super::state::CRASH_TAIL_GAP)).unwrap_or(("", tail));
    let machine = if machine.is_empty() { String::new() } else { format!("The computer:\n```\n{machine}\n```\n\n") };
    let body_with = |end: &str| format!("openOMSI {} on {}\n\n```\n{what}\n```\n\n{machine}The end of the log:\n```\n{end}\n```\n", updater::current_version(), std::env::consts::OS);
    let lines: Vec<&str> = end.lines().collect();
    let mut shown = 0;
    let body = loop {
        let body = body_with(&lines[lines.len() - shown..].join("\n"));
        if shown >= lines.len() || enc(&body).len() > 6500 {
            break if shown == 0 { body } else { body_with(&lines[lines.len() - shown.saturating_sub(1)..].join("\n")) };
        }
        shown += 1;
    };
    format!("{}/issues/new?title={}&body={}", updater::REPO_URL, enc(&title), enc(&body))
}

pub(super) fn mb(bytes: u64) -> String {
    format!("{:.1} MB", bytes as f64 / 1_000_000.0)
}
