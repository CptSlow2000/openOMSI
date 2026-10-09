//! The launcher's other pages: the driver's profile, the settings, the key bindings, the
//! running games, the mods and where things are.

use super::Launcher;
use omsi_launcher_lib as core;
use serde_json::{json, Value};

// the "Add binding" picker of the keyboard page: OMSI's events and the installed buses' own
// triggers, searchable
pub(crate) mod keybind_picker;
use keybind_picker::KeyActionOption;

#[derive(Default)]
pub struct PagesView {
    /// The "reset every setting" dialog is open.
    pub confirm_reset: bool,
    pub kb_filter: [String; 2],
    /// The keyboard action picker: which section it adds to, and its search query.
    pub kb_picker: Option<usize>,
    pub kb_picker_filter: String,
    pub kb_picker_source_filter: String,
    /// The complete bus-file provenance list shown from an action row.
    pub kb_source_action: Option<(String, Vec<String>)>,
    /// (section, index) of the binding waiting for a key.
    pub capturing: Option<(usize, usize)>,
    pub drop_hover: bool,
    pub setup_root: Option<String>,
    /// The Controls page's tab: 0 the keyboard, 1 the game controllers.
    pub controls_tab: usize,
    /// Sources indexed by action name, filled as installed vehicle scripts are scanned.
    pub kb_script_actions: Option<std::collections::HashMap<String, Vec<String>>>,
    pub kb_script_actions_rx: Option<std::sync::mpsc::Receiver<crate::describe::ScriptActionScanUpdate>>,
    kb_script_actions_root: Option<std::path::PathBuf>,
    kb_script_cache_loaded: bool,
    kb_script_scan_actions: Option<std::collections::HashMap<String, Vec<String>>>,
    kb_script_scan_paths: Vec<String>,
    pub(crate) kb_source_paths: Vec<String>,
    kb_source_path_set: std::collections::HashSet<String>,
    kb_source_suggestions: Option<(String, Vec<String>)>,
    pub kb_script_scan: (usize, usize, String),
    pub(crate) kb_script_total_buses: usize,
    pub kb_script_scan_complete: bool,
    pub(crate) kb_action_options: Option<Vec<KeyActionOption>>,
    pub(crate) kb_filtered_options: Option<(String, String, Vec<KeyActionOption>)>,
    pub(crate) controller_action_choices: Option<std::sync::Arc<(Vec<String>, Vec<String>)>>,
    /// The Settings page's tab (see `SETTINGS_TABS`).
    pub settings_tab: usize,
    pub pads: PadsView,
    pub tt: super::timetable::TimetableView,
}

/// The game controllers tab: the devices `gamectrler.cfg` sets up, the ones connected now,
/// the one shown, a button being waited for.
#[derive(Default)]
pub struct PadsView {
    pub io: Option<crate::controllers::Devices>,
    /// The set-up assistant, while it runs.
    pub wizard: Option<Wizard>,
    pub(crate) feedback_test: bool,
    pub devices: Option<Vec<crate::controllers::DeviceCfg>>,
    pub selected: usize,
    /// Waiting for a button of the shown device to be pressed (to add its binding).
    pub capturing: bool,
    /// A button found through "Add a button", kept visible even past the highlight.
    pub revealed_button: Option<usize>,
    pub dirty: bool,
    /// The button last pressed on the shown device and when: its line is lit, so that one
    /// sees which it is and what it does, and can give it an action there.
    pub last_pressed: Option<(usize, std::time::Instant)>,
    /// "Remove this device" clicked once, and when: a second click removes it.
    pub(crate) confirm_remove: Option<std::time::Instant>,
}

/// The set-up assistant of a device: the player lets go of everything, then turns the wheel
/// to the left and presses each pedal in turn; what moved most each time is that control
/// (and which way it runs), as OMSI's options dialog has the player choose by hand.
pub struct Wizard {
    pub step: usize,
    /// Where each axis rests, and where it stood at each step (left, throttle, brake,
    /// clutch).
    pub rest: [Option<f32>; 8],
    pub at: Vec<[Option<f32>; 8]>,
    pub error: Option<String>,
    pub(crate) calibration: Option<(std::time::Instant, crate::ffb_calibration::Calibration)>,
    pub(crate) ff_choice: Option<bool>,
    pub(crate) test_strength: f32,
}

impl PadsView {
    pub(super) fn cancel_feedback_test(&mut self) {
        release_feedback(&mut self.io, &mut self.feedback_test);
        if let Some((_, test)) = self.wizard.as_mut().and_then(|w| w.calibration.as_mut()) {
            if test.result.is_none() {
                test.fail("The test was interrupted. Please try again.");
            }
        }
    }

    /// Give every controller handle up before handing the hardware to the game. The normal
    /// controller list uses non-exclusive DirectInput too, not only the force-feedback test.
    pub(super) fn release_io(&mut self) {
        self.cancel_feedback_test();
        self.io = None;
    }
}

pub(crate) fn release_feedback(io: &mut Option<crate::controllers::Devices>, active: &mut bool) {
    if *active {
        *io = None;
        *active = false;
    }
}

// --- profile --------------------------------------------------------------------------------

/// A Unix time as "YYYY-MM-DD HH:MM" in the machine's time zone.
pub(crate) fn chrono_like(t: u64) -> String {
    #[cfg(unix)]
    {
        let tt = t as libc::time_t;
        let mut tm: libc::tm = unsafe { std::mem::zeroed() };
        if !unsafe { libc::localtime_r(&tt, &mut tm) }.is_null() {
            return format!("{:04}-{:02}-{:02} {:02}:{:02}", tm.tm_year + 1900, tm.tm_mon + 1, tm.tm_mday, tm.tm_hour, tm.tm_min);
        }
    }
    let days = (t / 86400) as i64;
    let secs = t % 86400;
    // civil from days (Howard Hinnant), UTC
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02} {:02}:{:02} UTC", secs / 3600, (secs % 3600) / 60)
}

// --- settings ---------------------------------------------------------------------------------

/// The window sizes the settings offer (`resolution`).
pub(crate) const RESOLUTIONS: &[(&str, &str)] = &[("auto", "Automatic"), ("1280x720", "1280 x 720"), ("1280x800", "1280 x 800 (Steam Deck)"), ("1366x768", "1366 x 768"), ("1600x900", "1600 x 900"), ("1920x1080", "1920 x 1080"), ("1920x1200", "1920 x 1200"), ("2560x1440", "2560 x 1440"), ("3840x2160", "3840 x 2160")];

/// The settings page's tabs: what one has come to change.
pub const SETTINGS_TABS: [&str; 6] = ["Graphics", "Driving", "Camera", "Sound", "Gameplay", "General"];

// --- controls ---------------------------------------------------------------------------------

/// The game's own actions a controller's button can be given, besides the bus's: the doors
/// and gears of any bus, looking round while held, the bus radio while held, the cameras
/// and the views - both of OMSI's view resets, the one view's (C) and every view's (Space),
/// which a controller could not bring back to the first camera (#1167) - and the main menu,
/// which Esc opens and a controller has no Esc for.
pub(crate) const PAD_GAME_ACTIONS: [&str; 27] = ["doors_all", "door_4", "door_3", "door_2", "door_1", "gear_up", "gear_down", "view_look_left", "view_look_right", "view_look_up", "view_look_down", "view_reset_direction", "view_reset_all_directions", "view_interiorcam_plus", "view_interiorcam_minus", "view_toggle_viewpoint", "view_toggle_interior", "view_set_driver", "view_set_passenger", "view_set_outside", "sim_pause", "open_menu", "screenshot", "quicksave", "toggel_mouse_ctrl", "toggel_ctrler", "voice_radio"];

pub(crate) fn action_text(names: &crate::describe::ControlNames, a: &str) -> String {
    known_action(a).unwrap_or_else(|| names.control(a))
}

pub(crate) fn control_names(l: &Launcher) -> &'static crate::describe::ControlNames {
    crate::describe::names(std::path::Path::new(&l.state.config.root), l.state.settings.get("language").and_then(|x| x.as_str()).unwrap_or("ENG"))
}

fn known_action(a: &str) -> Option<String> {
    if let Some(gear) = a.strip_prefix("kw_s_").and_then(|s| s.strip_suffix("_fest")) {
        return Some(format!("Gear {gear} (H-pattern)"));
    }
    let known: &[(&str, &str)] = &[
        ("throttle", "Throttle"),
        ("brake", "Brake"),
        ("throttle_amplify", "Throttle (full, kickdown)"),
        ("clutch", "Clutch"),
        ("steering_left", "Steer left"),
        ("steering_right", "Steer right"),
        ("steering_neutral", "Steering to centre"),
        ("parking_brake_toggle", "Parking brake"),
        ("blinker_left_set", "Indicator left"),
        ("blinker_right_set", "Indicator right"),
        ("blinker_left_toggle", "Indicator left (toggle)"),
        ("blinker_right_toggle", "Indicator right (toggle)"),
        ("blinker_off", "Indicators off"),
        ("blinker_warn_toggle", "Hazard lights"),
        ("gear_up", "Gear up (manual gearbox)"),
        ("gear_down", "Gear down (manual gearbox)"),
        ("horn", "Horn"),
        ("kw_scheinwerfer_toggle", "Headlights"),
        ("kw_standlicht_toggle", "Sidelights"),
        ("kw_fernlicht_toggle", "High beam"),
        ("kw_m_enginestart", "Starter"),
        ("kw_wipermode_up", "Wipers (next mode)"),
        ("cp_batterietrennschalter_toggle", "Battery / ignition"),
        ("automatic_D", "Gear D"),
        ("automatic_N", "Gear N"),
        ("automatic_R", "Gear R"),
        ("bus_doorfront0", "Front door (leaf 1)"),
        ("bus_doorfront1", "Front door (leaf 2)"),
        ("bus_dooraft", "Release rear doors"),
        ("door_1", "Door 1 (front), any bus"),
        ("door_2", "Door 2, any bus"),
        ("door_3", "Door 3, any bus"),
        ("door_4", "Door 4, any bus"),
        ("doors_all", "All doors, any bus"),
        ("ticket_give", "Sell the requested ticket"),
        ("view_set_driver", "Driver's view"),
        ("view_set_passenger", "Passenger view"),
        ("view_set_outside", "Outside view"),
        ("view_toggle_viewpoint", "Next view"),
        ("view_toggle_interior", "Cabin and outside, one key"),
        ("vr_recenter", "VR: Reset view"),
        ("vr_toggle_desktop_mirror", "VR: Monitor preview"),
        ("vr_toggle_mode", "VR: Switch VR / desktop"),
        ("vr_toggle_navigator", "VR: Toggle navigator"),
        ("vr_position_navigator", "VR: Position navigator"),
        ("exit", "Quit"),
        ("chat_open", "Multiplayer: write in the chat"),
        ("chat_toggle", "Multiplayer: show / hide the chat"),
        ("voice_radio", "Multiplayer: bus radio (hold)"),
        ("sim_pause", "Pause"),
        ("open_menu", "Open / close the main menu"),
        ("screenshot", "Screenshot"),
        ("quicksave", "Quicksave"),
        ("toggel_mouse_ctrl", "Toggle mouse steering"),
        ("toggel_ctrler", "Toggle game controllers"),
    ];
    known.iter().find(|k| k.0 == a).map(|k| k.1.to_string())
}

/// Hide empty slots beyond the physical buttons without changing the saved controller file.
pub(crate) fn shown_button_count(buttons: &[(String, String)], physical: usize, revealed: Option<usize>) -> usize {
    physical
        .max(buttons.iter().rposition(|(action, _)| !action.trim().is_empty()).map(|i| i + 1).unwrap_or(0))
        .max(revealed.map(|i| i + 1).unwrap_or(0))
}

/// Take the device shown (`selected`) out of the list; the one below it (or the last) is
/// shown next. Its name.
pub(crate) fn remove_device(devices: &mut Vec<crate::controllers::DeviceCfg>, selected: &mut usize) -> String {
    let name = devices.remove(*selected).name;
    *selected = (*selected).min(devices.len().saturating_sub(1));
    name
}

/// The steps of the set-up assistant (see `Wizard`): what the player is asked each time.
pub(crate) const WIZARD_STEPS: [(&str, &str); 5] = [
    ("Let go of everything", "Take your hands off the wheel and your feet off the pedals (the wheel in the middle), then press Next."),
    ("Steering", "Turn the wheel (or move the stick) all the way to the LEFT and hold it there, then press Next."),
    ("Throttle", "Press the throttle pedal all the way down and hold it, then press Next. No pedals: Skip."),
    ("Brake", "Press the brake pedal all the way down and hold it, then press Next. No brake pedal: Skip."),
    ("Clutch", "Press the clutch pedal all the way down and hold it, then press Next. No clutch: Skip."),
];

/// The axes the assistant found: `rest` where everything rested, `at` where the axes stood
/// with the wheel turned left, the throttle, the brake and the clutch pressed (all None: that
/// step skipped).
pub(crate) fn wizard_result(rest: &[Option<f32>; 8], at: &[[Option<f32>; 8]]) -> [Option<(crate::controllers::Func, bool)>; 8] {
    use crate::controllers::Func;
    let mut axes: [Option<(Func, bool)>; 8] = [None; 8];
    let steer = at.first().and_then(|a| moved_most(rest, a, &[]));
    if let Some((k, delta)) = steer {
        // turned left the value falls: else the axis runs the other way
        axes[k] = Some((Func::Steering, delta > 0.0));
    }
    let taken: Vec<usize> = steer.map(|s| vec![s.0]).unwrap_or_default();
    let pedal = |i: usize, ex: &[usize]| at.get(i).and_then(|a| moved_most(rest, a, ex));
    let throttle = pedal(1, &taken);
    let brake = pedal(2, &taken);
    match (throttle, brake) {
        // one axis for both (pedals on a single axis): the throttle towards the raw maximum
        // (as in Omsi.exe), else the axis is reversed
        (Some((kt, dt)), Some((kb, db))) if kt == kb && dt * db < 0.0 => axes[kt] = Some((Func::ThrottleBrake, dt < 0.0)),
        _ => {
            // a pedal pressed goes towards 1
            if let Some((k, dl)) = throttle {
                axes[k] = Some((Func::Throttle, dl < 0.0));
            }
            if let Some((k, dl)) = brake.filter(|b| Some(b.0) != throttle.map(|t| t.0)) {
                axes[k] = Some((Func::Brake, dl < 0.0));
            }
        }
    }
    let mut ex = taken.clone();
    ex.extend(throttle.map(|t| t.0));
    ex.extend(brake.map(|t| t.0));
    if let Some((k, dl)) = pedal(3, &ex) {
        axes[k] = Some((Func::Clutch, dl < 0.0));
    }
    axes
}

/// The axis that moved most from `rest` to `now` (at least a sixth of its travel), not one of
/// `exclude`: (slot, how far, signed).
pub(crate) fn moved_most(rest: &[Option<f32>; 8], now: &[Option<f32>; 8], exclude: &[usize]) -> Option<(usize, f32)> {
    (0..8)
        .filter(|k| !exclude.contains(k))
        .filter_map(|k| Some((k, now[k]? - rest[k].unwrap_or(0.0))))
        .filter(|(_, d)| d.abs() > 0.33)
        .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
}

/// Write the devices to the content folder's `Inputs/gamectrler.cfg` (OMSI 2's own is only
/// read; the game takes the content folder's first).
pub(crate) fn save_gamectrler(devices: &[crate::controllers::DeviceCfg]) -> Result<std::path::PathBuf, String> {
    let candidate = core::content_dir().unwrap_or_else(core::data_dir).join("Inputs");
    let dir = if (candidate.exists() || std::fs::create_dir_all(&candidate).is_ok()) && omsi_cfg::is_writable(&candidate) {
        candidate
    } else {
        core::data_dir().join("Inputs")
    };
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let p = dir.join("gamectrler.cfg");
    std::fs::write(&p, crate::controllers::cfg_text(devices)).map_err(|e| e.to_string())?;
    omsi_cfg::content_changed();
    Ok(p)
}

/// Settings → Driving keys: "Custom controls", the keys of the Controls page.
pub(crate) fn use_custom_keys(l: &mut Launcher) -> bool {
    if l.state.settings.get("drive_keys").and_then(|v| v.as_str()) == Some("omsi") {
        return false;
    }
    l.state.settings["drive_keys"] = json!("omsi");
    l.state.settings_dirty = 0.3;
    l.state.set_status("Driving keys: Custom controls - the game uses the keys of this page.", false);
    true
}

pub(crate) fn save_keys(l: &mut Launcher, vr_binding: bool) {
    match core::save_keybindings(&l.state.keybindings) {
        Ok(()) => {
            l.state.keybindings_error.clear();
            if let Ok(k) = core::get_keybindings() {
                l.state.keybindings = k;
            }
            l.pages.kb_action_options = None;
            l.pages.kb_filtered_options = None;
            l.pages.controller_action_choices = None;
            // a key changed is a key the player wants to use: with a ready-made layout it
            // would be ignored wherever that layout has a key of its own
            if !vr_binding && use_custom_keys(l) {
                l.state.set_status("Key bindings saved; Driving keys switched to Custom controls so the game uses them.", false);
            } else {
                l.state.set_status("Key bindings saved.", false);
            }
        }
        Err(e) => {
            l.state.keybindings_error = format!("{e:#}");
            l.state.set_status(format!("{e:#}"), true);
        }
    }
}

// --- sessions ---------------------------------------------------------------------------------

// --- mods ----------------------------------------------------------------------------------------

// --- setup -----------------------------------------------------------------------------------------


// --- tutorials --------------------------------------------------------------------------------

#[cfg(test)]
mod wizard_tests {
    #[test]
    fn cancelling_feedback_releases_io_and_invalidates_the_test() {
        let mut pads = super::PadsView::default();
        pads.feedback_test = true;
        pads.wizard = Some(super::Wizard {
            step: super::WIZARD_STEPS.len(), rest: [None; 8], at: Vec::new(), error: None,
            calibration: Some((std::time::Instant::now(), crate::ffb_calibration::Calibration::new(crate::ffb_calibration::PULSE_FORCE))), ff_choice: None, test_strength: crate::ffb_calibration::PULSE_FORCE,
        });
        pads.cancel_feedback_test();
        assert!(!pads.feedback_test);
        assert!(pads.io.is_none());
        let test = &pads.wizard.as_ref().unwrap().calibration.as_ref().unwrap().1;
        assert!(test.result.unwrap().is_err());
        assert_eq!(pads.wizard.as_ref().unwrap().ff_choice, None);
    }

    use crate::controllers::Func;

    #[test]
    fn h_pattern_gears_have_a_clear_name() {
        assert_eq!(super::known_action("kw_s_1_fest").as_deref(), Some("Gear 1 (H-pattern)"));
        assert_eq!(super::known_action("kw_s_R_fest").as_deref(), Some("Gear R (H-pattern)"));
    }

    #[test]
    fn empty_saved_button_slots_do_not_fill_the_controller_list() {
        let mut buttons = vec![(String::new(), "0".to_string()); 131];
        buttons[10].0 = "horn".to_string();
        assert_eq!(super::shown_button_count(&buttons, 18, None), 18);
        assert_eq!(super::shown_button_count(&buttons, 18, Some(128)), 129);
    }

    #[test]
    fn a_wheel_with_three_pedals() {
        // X the wheel; Y throttle, Z brake, Rz clutch - pedals reading 1 up, -1 down (as the
        // G25's run, "reversed")
        let rest = [Some(0.0), Some(1.0), Some(1.0), None, None, Some(1.0), None, None];
        let mut left = rest;
        left[0] = Some(-1.0);
        let mut thr = rest;
        thr[1] = Some(-1.0);
        let mut brk = rest;
        brk[2] = Some(-1.0);
        let mut clu = rest;
        clu[5] = Some(-1.0);
        let a = super::wizard_result(&rest, &[left, thr, brk, clu]);
        assert_eq!(a[0], Some((Func::Steering, false)));
        assert_eq!(a[1], Some((Func::Throttle, true)));
        assert_eq!(a[2], Some((Func::Brake, true)));
        assert_eq!(a[5], Some((Func::Clutch, true)));
    }

    #[test]
    fn pedals_on_one_axis_and_a_wheel_the_other_way() {
        let rest = [Some(0.0), Some(0.0), None, None, None, None, None, None];
        let a = super::wizard_result(&rest, &[[Some(0.9), Some(0.0), None, None, None, None, None, None], [Some(0.0), Some(-1.0), None, None, None, None, None, None], [Some(0.0), Some(1.0), None, None, None, None, None, None], [None; 8]]);
        assert_eq!(a[0], Some((Func::Steering, true)));
        assert_eq!(a[1], Some((Func::ThrottleBrake, true)));
    }
}

#[cfg(test)]
mod pad_action_tests {
    /// Every game action a button can be given is one the game carries out from a
    /// controller (app_events: `view_look_*` / `voice_radio` while held, the gears by name,
    /// the rest through `is_game_action`, the doors through `Player::action`) - Space's
    /// reset of every view among them (#1167).
    #[test]
    fn a_button_can_reset_every_view() {
        assert!(super::PAD_GAME_ACTIONS.contains(&"view_reset_all_directions"));
        assert!(super::PAD_GAME_ACTIONS.contains(&"voice_radio"));
        assert_eq!(super::known_action("voice_radio").as_deref(), Some("Multiplayer: bus radio (hold)"));
        // the main menu, which a controller has no Esc for
        assert!(super::PAD_GAME_ACTIONS.contains(&"open_menu"));
        assert_eq!(super::known_action("open_menu").as_deref(), Some("Open / close the main menu"));
        for a in super::PAD_GAME_ACTIONS {
            let held = a.starts_with("view_look_") || a == "voice_radio";
            let handled = held
                || crate::input_script::is_game_action(a)
                || a.starts_with("gear_")
                || crate::player::door_action(a).is_some();
            assert!(handled, "{a}");
        }
        // held pad actions are game actions so they never reach the bus script
        assert!(crate::input_script::is_game_action("voice_radio"));
        assert!(crate::input_script::is_game_action("view_look_left"));
    }
}

#[cfg(test)]
mod pad_remove_tests {
    use crate::controllers::{cfg_text, parse_cfg};

    /// A device taken out of the list is gone from the file Save writes, and the next one is
    /// shown - the one below it, or above it when it was the last (#636).
    #[test]
    fn a_removed_device_leaves_the_file() {
        let mut devices = parse_cfg("[ctrl]\r\nSideWinder Joystick\r\n0\r\n\r\n[ctrl]\r\nLogitech G25 Racing Wheel USB\r\n1\r\n\r\n[ctrl]\r\nMOZA R3 Base\r\n0\r\n");
        let mut sel = 1;
        assert_eq!(super::remove_device(&mut devices, &mut sel), "Logitech G25 Racing Wheel USB");
        assert_eq!(sel, 1);
        let names = |text: &str| parse_cfg(text).into_iter().map(|d| d.name).collect::<Vec<_>>();
        assert_eq!(names(&cfg_text(&devices)), ["SideWinder Joystick", "MOZA R3 Base"]);
        assert_eq!(super::remove_device(&mut devices, &mut sel), "MOZA R3 Base");
        assert_eq!(sel, 0);
        assert_eq!(super::remove_device(&mut devices, &mut sel), "SideWinder Joystick");
        assert_eq!(sel, 0);
        assert!(names(&cfg_text(&devices)).is_empty());
    }
}
