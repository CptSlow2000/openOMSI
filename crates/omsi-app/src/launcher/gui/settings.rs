//! The Settings page: six tabs of rows - choices, switches, sliders - each bound to a key of
//! the settings, written as a table here and laid out by one builder, so a setting is added
//! by adding its row. Every change is saved at once (`settings_dirty`); the game reads them
//! when it starts. A few parts have their own shape: the quality presets, the saved graphics
//! profiles, the radio stations, the navigator's corner, the updates.

use super::kit::{self, len, lp, tr};
use super::theme::ACCENT;
use super::Msg as Top;
use crate::launcher::{Launcher, Page};
use egui_retained::widgets::{Button, Checkbox, TextInput};
use egui_retained::{Color32, NodeId, ScrollAxes, Ui, Visual, taffy};
use omsi_launcher_lib as core;
use serde_json::{Value, json};

pub use crate::launcher::pages::SETTINGS_TABS as TABS;

type Show = fn(&Value) -> bool;
type Fmt = fn(f32) -> String;

fn always(_: &Value) -> bool {
    true
}
fn get<'a>(v: &'a Value, k: &str) -> &'a Value {
    v.get(k).unwrap_or(&Value::Null)
}
fn mode(s: &Value) -> &str {
    get(s, "graphics").as_str().unwrap_or("vanilla_plus")
}
fn not_vanilla(s: &Value) -> bool {
    mode(s) != "vanilla"
}
fn not_vanilla_not_traced(s: &Value) -> bool {
    !matches!(mode(s), "vanilla" | "enhanced_plus")
}
fn not_traced(s: &Value) -> bool {
    mode(s) != "enhanced_plus"
}
fn enhanced_clouds(s: &Value) -> bool {
    matches!(mode(s), "enhanced" | "enhanced_plus") && get(s, "clouds").as_bool() != Some(false)
}
fn triple(s: &Value) -> bool {
    get(s, "triple_screen").as_bool().unwrap_or(false)
}
fn vr(s: &Value) -> bool {
    cfg!(windows) && get(s, "vr").as_bool().unwrap_or(false)
}
fn ambient_on(s: &Value) -> bool {
    get(s, "ambient").as_bool() != Some(false)
}
fn windows(_: &Value) -> bool {
    cfg!(windows)
}

/// What the choices of a row are.
#[derive(Clone, Copy)]
enum Opts {
    Fixed(&'static [(&'static str, &'static str)]),
    Of(fn(&Value) -> Vec<(String, String)>),
}

/// A button's deed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::launcher) enum Act {
    Keys,
    Pads,
    WheelReset,
    SeatReset,
    TrackReset,
    CheckUpdates,
    Github,
    Reset,
}

#[derive(Clone, Copy)]
enum Row {
    Section(&'static str),
    Para(&'static str, Show),
    Select(&'static str, &'static str, Opts, Show),
    Toggle(&'static str, &'static str, Show),
    /// label, key, min, max, step, default, how the value reads, show
    Slider(&'static str, &'static str, f32, f32, f32, f32, Fmt, Show),
    Button(&'static str, &'static str, Act, Show),
    Preset,
    Profiles,
    Radio,
    Corner,
    Updates,
    Fov,
    MtStatus,
}

use Row::*;

fn pct(v: f32) -> String {
    format!("{:.0}%", v * 100.0)
}
fn omsi_pct(v: f32) -> String {
    if (v - 1.0).abs() < 0.01 { "OMSI".into() } else { pct(v) }
}
fn off_or(v: f32, f: impl Fn(f32) -> String) -> String {
    if v <= 0.005 { "Off".into() } else { f(v) }
}

fn api_options(_: &Value) -> Vec<(String, String)> {
    let o: &[(&str, &str)] = if cfg!(windows) {
        &[("auto", "Automatic"), ("vulkan", "Vulkan"), ("dx12", "DirectX 12"), ("gl", "OpenGL"), ("angle", "ANGLE (DirectX 11)")]
    } else {
        &[("auto", "Automatic"), ("vulkan", "Vulkan"), ("gl", "OpenGL")]
    };
    o.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect()
}

fn texmem_options(s: &Value) -> Vec<(String, String)> {
    let adapter_mb = omsi_render::ADAPTER_TEXTURE_MB.load(std::sync::atomic::Ordering::Relaxed) as i64;
    let auto_mb = match (get(s, "texture_memory_auto").as_i64().unwrap_or(0), adapter_mb) {
        (m, 0) => m,
        (0, a) => a,
        (m, a) => m.min(a),
    };
    let mb = |v: i64| if v >= 1000 { format!("{:.1} GB", v as f64 / 1000.0) } else { format!("{v} MB") };
    let auto = if auto_mb > 0 { format!("Automatic ({} here)", mb(auto_mb)) } else { "Automatic".to_string() };
    std::iter::once(("0".to_string(), auto)).chain([("500", "500 MB"), ("1000", "1 GB"), ("1500", "1.5 GB"), ("2000", "2 GB"), ("3000", "3 GB"), ("4000", "4 GB"), ("6000", "6 GB")].iter().map(|(a, b)| (a.to_string(), b.to_string()))).collect()
}

fn languages(_: &Value) -> Vec<(String, String)> {
    core::LANGUAGES.iter().map(|l| (l.0.to_string(), l.1.to_string())).collect()
}

fn has_api(_: &Value) -> bool {
    !cfg!(target_os = "macos")
}

/// The tabs, each two columns of a title and its rows.
fn table() -> [[(&'static str, Vec<Row>); 2]; 6] {
    [
        [
            ("Graphics", vec![
                Select("Graphics", "graphics", Opts::Fixed(&[("vanilla", "Vanilla (as OMSI 2)"), ("vanilla_plus", "Vanilla+"), ("enhanced", "Enhanced"), ("enhanced_plus", "Enhanced+")]), always),
                Preset,
                Select("Anti-aliasing", "msaa", Opts::Fixed(&[("1", "Off"), ("2", "2x MSAA"), ("4", "4x MSAA"), ("8", "8x MSAA")]), always),
                Select("Render scale", "render_scale", Opts::Fixed(&[("auto", "Auto"), ("1", "100%"), ("0.85", "85%"), ("0.75", "75%"), ("0.67", "67%"), ("0.5", "50%")]), always),
                Select("Anisotropic", "anisotropy", Opts::Fixed(&[("1", "Off"), ("2", "2x"), ("4", "4x"), ("8", "8x"), ("16", "16x")]), always),
                Select("Shadow map", "shadow_size", Opts::Fixed(&[("1024", "1024"), ("2048", "2048"), ("4096", "4096")]), not_vanilla),
                Toggle("Ambient occlusion", "ssao", not_vanilla_not_traced),
                Toggle("Sun shadows", "shadows", not_vanilla_not_traced),
                Select("Shadows cast by", "shadow_casters", Opts::Fixed(&[("all", "Every solid mesh"), ("omsi", "[shadow] meshes, as OMSI")]), not_vanilla),
                Toggle("Detail texturing up close", "detail_textures", not_vanilla),
                Slider("Night brightness", "night_brightness", 0.0, 3.0, 0.25, 0.0, |v| off_or(v, |v| format!("+{v:.2}")), not_vanilla),
                Slider("LED glow", "led_glow", 0.0, 15.0, 1.0, 6.0, |v| if v < 0.5 { "Off".into() } else { format!("{}", v as i64) }, not_vanilla),
                Slider("LED mip strength", "led_mips", 0.0, 4.0, 0.05, 1.3, |v| off_or(v, |v| format!("{v:.2}")), not_vanilla),
                Toggle("OMSI's shadow meshes (under vehicles)", "shadow_blobs", always),
                Toggle("Reflection maps (paint, chrome, glass)", "reflections", not_traced),
                Toggle("Clouds", "clouds", always),
                Select("Cloud quality", "cloud_quality", Opts::Fixed(&[("high", "High"), ("low", "Low")]), enhanced_clouds),
                Toggle("Windy trees", "windy_trees", always),
            ]),
            ("Display", vec![
                Toggle("Fullscreen", "fullscreen", always),
                Select("Window size", "resolution", Opts::Fixed(crate::launcher::pages::RESOLUTIONS), always),
                Toggle("V-sync", "vsync", always),
                Select("Frame limit", "max_fps", Opts::Fixed(&[("0", "Screen refresh rate"), ("30", "30 fps"), ("45", "45 fps"), ("60", "60 fps"), ("120", "120 fps"), ("144", "144 fps"), ("1000", "Unlimited")]), always),
                Select("Graphics API", "graphics_api", Opts::Of(api_options), has_api),
                Section("World & memory"),
                Select("View distance", "view_distance", Opts::Fixed(&[("auto", "Default (1200 m)"), ("600", "600 m - fastest"), ("900", "900 m"), ("1200", "1200 m"), ("1500", "1500 m"), ("2000", "2000 m"), ("2500", "2500 m")]), always),
                Select("Object distance", "max_obj_dist", Opts::Fixed(&[("auto", "Automatic"), ("500", "500 m"), ("750", "750 m"), ("900", "900 m"), ("1500", "1500 m"), ("3000", "3000 m")]), always),
                Select("Small objects", "min_obj_size", Opts::Fixed(&[("0.005", "All"), ("0.013", "Normal"), ("0.02", "Fewer (faster)"), ("0.03", "Few (fastest)")]), always),
                Select("Mirrors", "mirror_size", Opts::Fixed(&[("0", "Off"), ("128", "Low (128)"), ("256", "Normal (256)"), ("512", "High (512)"), ("1024", "Very high (1024)")]), always),
                Select("Real-time reflections", "mirror_refresh", Opts::Fixed(&[("off", "None (frozen picture)"), ("eco", "Economical"), ("full", "Full")]), always),
                Select("Texture memory", "texture_memory", Opts::Of(texmem_options), always),
                Toggle("Compress textures on loading", "texture_compression", always),
                Toggle("DXT/BC textures stay compressed on the GPU", "gpu_texture_compression", always),
                Section("Profiles"),
                Profiles,
            ]),
        ],
        [
            ("Keyboard & mouse", vec![
                Select("Driving keys", "drive_keys", Opts::Fixed(&[("omsi", "Custom controls (Controls page)"), ("simple", "W A S D + arrows"), ("wasd", "W A S D only"), ("arrows", "Arrow keys only")]), always),
                Toggle("Steering linearity (keys at OMSI's steady pace)", "steering_linear", always),
                Toggle("Old Steering (the wheel stays, turn it back yourself)", "old_steering", always),
                Toggle("Dynamic steering (slower keys at speed, OMSI's redSteerSpd)", "red_steer_spd", always),
                Slider("Mouse steering sensitivity (O)", "mouse_sens", 0.1, 3.0, 0.05, 1.0, omsi_pct, always),
                Slider("Mouse pedal strength", "mouse_pedal_strength", 0.5, 2.0, 0.05, 1.0, omsi_pct, always),
                Toggle("Smooth mouse steering (off: the wheel follows the cursor at once, as in OMSI)", "mouse_smooth", always),
                Toggle("Hold the cursor while the mouse steers (off: the crosshair stays free, the window's edges are the lock)", "mouse_hold", always),
                Toggle("A right click ends the mouse steering (as in OMSI)", "mouse_right_off", always),
                Toggle("Indicators cancel themselves (as the bus's script does)", "blinker_cancel", always),
                Toggle("The keyboard brake stays on until the throttle (as in OMSI)", "brake_hold", always),
                Toggle("Automatic clutch (manual gearboxes)", "auto_clutch", always),
                Toggle("Automated manual gearbox (shifts a manual gearbox for you by the engine speed)", "auto_shift", always),
                Button("Change the keys", "keyboard", Act::Keys, always),
            ]),
            ("Game controllers", vec![
                Toggle("H-pattern shifter: return to neutral when the gear is released", "momentary_gears", always),
                Slider("Wheel rotation", "wheel_range", 180.0, 1800.0, 30.0, 900.0, |v| format!("{v:.0}°"), always),
                Slider("Full lock at", "wheel_lock", 0.0, 1800.0, 30.0, 0.0, |v| if v < 45.0 { "OMSI".into() } else { format!("{v:.0}°") }, always),
                Slider("Stick steering smoothing", "pad_steer_smooth", 0.0, 300.0, 10.0, 120.0, |v| off_or(v, |v| format!("{v:.0} ms")), always),
                Slider("Stick steering speed", "pad_steer_speed", 0.8, 5.0, 0.1, 2.0, |v| format!("{v:.1} s"), always),
                Slider("Stick dead zone", "pad_deadzone", 0.0, 0.4, 0.01, 0.08, |v| format!("{:.0} %", v * 100.0), always),
                Toggle("Stick steers like a wheel (a wheel seen as a gamepad)", "pad_steer_linear", always),
                Select("Gamepad type", "pad_type", Opts::Fixed(&[("auto", "Automatic"), ("xbox", "Xbox"), ("ps4", "PlayStation 4"), ("ps5", "PlayStation 5")]), always),
                Toggle("Default gamepad buttons (indicators, doors, views, gears, menu, pause)", "pad_buttons", always),
                Toggle("Arrow keys switch the cameras with a wheel too (no glance)", "arrows_switch_cams", always),
                Slider("Throttle pedal strength", "pedal_throttle", 0.5, 2.0, 0.05, 1.0, pedal, always),
                Slider("Brake pedal strength", "pedal_brake", 0.5, 2.0, 0.05, 1.0, pedal, always),
                Toggle("Force feedback and vibration", "ff_enabled", always),
                Toggle("Invert force feedback by default", "ff_invert", always),
                Para("Wheels with a saved direction use their own setting under Controls → Game controllers.", always),
                Slider("Road texture vibration", "ff_road_vib", 0.0, 4.0, 0.05, 1.0, vib, always),
                Slider("Engine vibration", "ff_engine_vib", 0.0, 4.0, 0.05, 1.0, vib, always),
                Slider("Vibration fade-out", "ff_fade", 0.0, 1.5, 0.05, 0.28, |v| off_or(v, |v| format!("{:.0} ms", (v * 1000.0).round())), always),
                Button("Reset wheel settings", "restart_alt", Act::WheelReset, always),
                Button("Set up a wheel or pedals", "sports_esports", Act::Pads, always),
            ]),
        ],
        [
            ("Driver's view", vec![
                Section("Seat position"),
                Slider("Seat forward / back", "seat_y", -0.6, 0.6, 0.01, 0.0, cm, always),
                Slider("Seat up / down", "seat_z", -0.6, 0.6, 0.01, 0.0, cm, always),
                Slider("Seat right / left", "seat_x", -0.6, 0.6, 0.01, 0.0, cm, always),
                Slider("Head pitch", "seat_pitch_deg", -45.0, 45.0, 1.0, 0.0, |v| format!("{v:+.0}°"), always),
                Button("Reset the seat position", "restart_alt", Act::SeatReset, always),
                Fov,
                Slider("Mouse look sensitivity", "look_sens", 0.1, 2.0, 0.05, 1.0, omsi_pct, always),
                Toggle("Right stick turns the view", "right_stick_look", always),
                Slider("Smooth the mouse look", "look_smoothing_ms", 0.0, 200.0, 10.0, 0.0, |v| off_or(v, |v| format!("{v:.0} ms")), always),
                Section("A head at rest"),
                Slider("Head sway at a standstill", "head_idle", 0.0, 1.0, 0.05, 0.0, |v| off_or(v, pct), always),
                Slider("Sway pace", "head_idle_pace", 0.5, 2.0, 0.05, 1.0, pct, always),
                Toggle("Driver's view turns with the steering", "steer_look", always),
                Slider("Steering view angle", "steer_look_angle", 0.0, 60.0, 1.0, 30.0, |v| format!("{v:.0}°"), always),
                Slider("Steering view response", "steer_look_response", 0.05, 1.0, 0.05, 0.25, |v| format!("{:.0} ms", v * 1000.0), always),
                Toggle("Head moves with the bus", "head_movement", always),
                Toggle("Camera glides between viewpoints", "driverview_smooth", always),
                Toggle("Driver's hands in the cab view", "hands_in_cab", always),
                Toggle("Right mouse button turns the view, Shift+right zooms (off: right zooms as in OMSI, the wheel button turns)", "alt_view", always),
                Toggle("Precision mouse zoom (FOV curve instead of the linear way)", "precision_zoom", always),
            ]),
            ("Outside views", vec![
                Toggle("Camera collisions (outside view)", "camera_collision", always),
                Toggle("Driver at the wheel (outside views)", "driver", always),
                Section("Head tracking"),
                Toggle("Head tracking (native TrackIR / OpenTrack)", "head_tracking", always),
                Section("Rotation"),
                Slider("Yaw sensitivity (left / right)", "head_tracking_yaw_sens", 0.0, 100.0, 1.0, 100.0, track, always),
                Slider("Pitch sensitivity (up / down)", "head_tracking_pitch_sens", 0.0, 100.0, 1.0, 100.0, track, always),
                Slider("Roll sensitivity", "head_tracking_roll_sens", 0.0, 100.0, 1.0, 100.0, track, always),
                Toggle("Invert yaw", "head_tracking_invert_yaw", always),
                Toggle("Invert pitch", "head_tracking_invert_pitch", always),
                Toggle("Invert roll", "head_tracking_invert_roll", always),
                Section("Position"),
                Slider("X sensitivity (left / right)", "head_tracking_x_sens", 0.0, 100.0, 1.0, 100.0, track, always),
                Slider("Y sensitivity (up / down)", "head_tracking_y_sens", 0.0, 100.0, 1.0, 100.0, track, always),
                Slider("Z sensitivity (forward / back)", "head_tracking_z_sens", 0.0, 100.0, 1.0, 100.0, track, always),
                Toggle("Invert X", "head_tracking_invert_x", always),
                Toggle("Invert Y", "head_tracking_invert_y", always),
                Toggle("Invert Z", "head_tracking_invert_z", always),
                Button("Reset head-tracking axes", "restart_alt", Act::TrackReset, always),
                Section("Triple screen"),
                Toggle("Three screen projections", "triple_screen", always),
                Toggle("Span three monitors at startup", "triple_span", always),
                Toggle("HUD on centre screen", "triple_hud_center", always),
                Para("Three equal screens in a horizontal row. OpenXR takes priority.", triple),
                Slider("Visible width of one panel", "triple_width_mm", 200.0, 2000.0, 10.0, 600.0, mm, triple),
                Slider("Eye to centre screen", "triple_distance_mm", 200.0, 3000.0, 10.0, 650.0, mm, triple),
                Slider("Both frames at each join", "triple_bezel_mm", 0.0, 100.0, 1.0, 0.0, mm, triple),
                Slider("Left screen inward angle", "triple_left_angle_deg", 0.0, 90.0, 1.0, 45.0, |v| format!("{v:.0}°"), triple),
                Slider("Right screen inward angle", "triple_right_angle_deg", 0.0, 90.0, 1.0, 45.0, |v| format!("{v:.0}°"), triple),
                Slider("Eye above screen centre", "triple_eye_height_mm", -500.0, 500.0, 1.0, 0.0, mm, triple),
                Section("Virtual reality"),
                Toggle("Use OpenXR headset", "vr", windows),
                Select("Eye resolution", "vr_scale", Opts::Fixed(&[("0.5", "50%"), ("0.65", "65%"), ("0.8", "80%"), ("1", "100%")]), vr),
                Select("Head tracking smoothing", "vr_head_smoothing_ms", Opts::Fixed(&[("0", "Off"), ("5", "5 ms"), ("10", "10 ms"), ("20", "20 ms"), ("30", "30 ms")]), vr),
                Select("Bus mirror refresh", "vr_mirror_rate", Opts::Fixed(&[("0", "Off"), ("8", "8/s"), ("16", "16/s"), ("24", "24/s"), ("32", "32/s"), ("48", "48/s"), ("60", "60/s"), ("90", "90/s"), ("120", "120/s"), ("180", "180/s"), ("240", "240/s"), ("360", "360/s"), ("-1", "Every frame")]), vr),
                Para("The rate is shared by all bus mirrors. Higher rates can reduce game FPS.", vr),
                Toggle("Show headset picture on monitor", "vr_desktop_mirror", vr),
                Button("Change the VR keys", "keyboard", Act::Keys, vr),
            ]),
        ],
        [
            ("Volume", vec![
                Slider("Volume", "volume", 0.0, 1.0, 0.05, 0.6, pct, always),
                Slider("Traffic", "vol_ai", 0.0, 1.0, 0.05, 1.0, pct, always),
                Slider("Surroundings", "vol_scenery", 0.0, 1.0, 0.05, 1.0, pct, always),
                Toggle("Doppler effect", "doppler", always),
                Toggle("Ambience (wind, nature, road surfaces)", "ambient", always),
                Slider("Ambience volume", "vol_ambient", 0.0, 1.0, 0.05, 0.8, pct, ambient_on),
                Select("Passenger voices", "pax_voices", Opts::Fixed(&[("all", "Greetings and tickets"), ("tickets", "Only the ticket asked for"), ("off", "Silent")]), always),
            ]),
            ("Radio stations", vec![
                Para("A radio's station button n plays the n-th station, a cassette player the first; Shift+R steps through them. An address is an MP3, AAC or Ogg stream or an .m3u/.pls playlist.", always),
                Radio,
            ]),
        ],
        [
            ("Passengers", vec![
                Select("Boarding", "boarding", Opts::Fixed(&[("auto", "Pay and take the ticket"), ("pay", "The driver sells the ticket"), ("walk", "Just walk in")]), always),
                Toggle("Passengers pay the exact fare", "exact_fare", always),
                Slider("How many passengers", "pax_density", 0.0, 2.0, 0.1, 1.0, pct, always),
                Toggle("Ability to get up (Ctrl+Shift+G)", "get_up", always),
                Section("Traffic"),
                Select("Random traffic", "ai_unsched_factor", Opts::Fixed(&[("25", "25%"), ("50", "50%"), ("75", "75%"), ("100", "100%"), ("150", "150%"), ("200", "200%")]), always),
                Select("Timetable vehicles", "ai_max_scheduled", Opts::Fixed(&[("0", "All"), ("10", "At most 10"), ("25", "At most 25"), ("50", "At most 50")]), always),
                Select("Parked cars", "ai_max_parked", Opts::Fixed(&[("-1", "None"), ("0", "Every space"), ("35", "At most 35"), ("100", "At most 100"), ("250", "At most 250")]), always),
                Toggle("Timetable buses ahead of time wait only at timed stops", "ai_wait_timed_stops_only", always),
            ]),
            ("Simulation", vec![
                Select("Maintenance", "maintenance", Opts::Fixed(&[("0", "Infinite (no wear)"), ("1", "Very bad"), ("2", "Bad"), ("3", "Normal"), ("4", "Good")]), always),
                Toggle("Collisions with vehicles", "collision_vehicles", always),
                Toggle("Collisions with objects (walls, poles)", "collision_objects", always),
                Toggle("Collisions with people", "collision_pedestrians", always),
                Toggle("Start at the real time", "use_real_time", always),
                Toggle("Start on today's date", "use_real_date", always),
                Toggle("Sync the clock with the real time (locks the time)", "time_sync", always),
                Toggle("Sync the weather with METAR (locks the weather)", "metar_sync", always),
                Select("Time speed (not in multiplayer or with the real-time sync)", "time_speed", Opts::Fixed(&[("1", "Real time"), ("2", "x2"), ("4", "x4"), ("8", "x8"), ("15", "x15"), ("30", "x30")]), always),
            ]),
        ],
        [
            ("Interface & online", vec![
                Select("Language", "language", Opts::Of(languages), always),
                Toggle("Translate the remaining texts automatically (offline, downloads 620 MB once)", "machine_translation", always),
                MtStatus,
                Toggle("The launcher rests while a game runs (gives the graphics card to the game)", "launcher_rest", always),
                Toggle("Discord Rich Presence", "discord_status", always),
                Para("Shows the launcher or your map, bus, line and multiplayer status in Discord.", always),
                Toggle("Voice chat through GreenTeaSpeak (multiplayer)", "voice_chat", always),
                Para("Players near you are heard from where they stand, when GreenTeaSpeak runs with the openOMSI plugin and the server names a voice server.", always),
                Slider("Game interface size", "ui_scale", 0.5, 2.0, 0.05, 1.0, pct, always),
                Toggle("Interface grows with the window", "ui_scale_window", always),
                Slider("Interface opacity", "ui_opacity", 0.2, 1.0, 0.05, 0.85, pct, always),
                Toggle("Name of the button under the mouse", "tooltips", always),
                Toggle("Frame rate in the corner", "show_fps", always),
                Toggle("Notes in the top-left corner", "notes", always),
                Toggle("Chat in online games", "chat", always),
                Slider("Chat size", "chat_size", 0.5, 3.0, 0.1, 1.0, pct, always),
                Toggle("Other players' names above their buses", "name_tags", always),
                Section("Navigator"),
                Toggle("Navigator (Shift+N: map, schedule, off)", "navigator", always),
                Toggle("Route arrows (as in OMSI 2)", "nav_arrows", always),
                Toggle("AI vehicles on the map", "nav_ai", always),
                Corner,
            ]),
            ("Updates", vec![
                Toggle("Look for updates when the launcher starts", "update_check", always),
                Toggle("Install updates without asking", "update_auto", always),
                Toggle("Tell me about a new version during a session", "update_notify", always),
                Toggle("Count me in the website's \"playing now\" (anonymous)", "presence", always),
                Updates,
                Button("github.com/openOMSI-org/openOMSI", "open_in_new", Act::Github, always),
                Section("Reset"),
                Button("Reset all settings...", "restart_alt", Act::Reset, always),
            ]),
        ],
    ]
}

fn pedal(v: f32) -> String {
    if (v - 1.0).abs() < 0.01 { "Normal".into() } else if v < 1.0 { format!("Softer x{v:.2}") } else { format!("Stronger x{v:.2}") }
}
fn vib(v: f32) -> String {
    if v < 0.01 { "Off".into() } else if (v - 1.0).abs() < 0.01 { "Normal".into() } else { pct(v) }
}
fn cm(v: f32) -> String {
    format!("{:+.0} cm", v * 100.0)
}
fn mm(v: f32) -> String {
    format!("{v:.0} mm")
}
fn track(v: f32) -> String {
    if v <= 0.0 { "Off".into() } else { format!("{v:.0}%") }
}

/// Where the rows are, to address them by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::launcher) struct At {
    tab: usize,
    col: usize,
    row: usize,
}

#[derive(Clone, Debug)]
pub(in crate::launcher) enum Msg {
    Tab(usize),
    Choose(At, usize),
    Flag(At, bool),
    Value(At, f32),
    Act(Act),
    Preset(usize),
    ProfilePick(usize),
    ProfileName(String),
    ProfileLoad,
    ProfileDelete,
    ProfileSave,
    RadioName(usize, String),
    RadioUrl(usize, String),
    RadioRemove(usize),
    RadioAdd,
    Corner(&'static str),
    Fov(f32),
}

fn m(x: Msg) -> Top {
    Top::Settings(x)
}

/// The widgets of a row.
struct Built {
    at: At,
    row: Row,
    node: NodeId,
    input: Option<NodeId>,
    value: Option<NodeId>,
}

pub(in crate::launcher) struct SettingsPage {
    pub root: NodeId,
    tabs: Vec<NodeId>,
    bodies: Vec<NodeId>,
    rows: Vec<Built>,
    table: [[(&'static str, Vec<Row>); 2]; 6],
    preset: NodeId,
    preset_presets: Vec<(String, Value)>,
    profiles: Option<(NodeId, NodeId, NodeId)>,
    profile: GfxProfiles,
    radio: NodeId,
    radio_list: Option<Vec<(String, String)>>,
    radio_key: usize,
    corners: Vec<(&'static str, NodeId)>,
    update_button: NodeId,
    update_text: NodeId,
    mt: NodeId,
    fov: NodeId,
    fov_value: NodeId,
}

#[derive(Default)]
struct GfxProfiles {
    name: String,
    sel: usize,
    list: Option<Vec<String>>,
    msg: String,
}

impl SettingsPage {
    pub fn build(ui: &mut Ui, host: NodeId) -> SettingsPage {
        let root = ui.column(host);
        ui.add_class(root, "content");
        kit::grow(ui, root);
        kit::page_title(ui, root, "Settings", "Every change is saved at once; the game reads them when it starts.");
        let bar = ui.row(root);
        kit::gap(ui, bar, 4.0);
        ui.style(bar, |s| {
            s.flex_shrink = 0.0;
            s.flex_wrap = taffy::FlexWrap::Wrap;
        });
        let tabs = TABS
            .iter()
            .enumerate()
            .map(|(k, t)| {
                let b = ui.add(bar, Button::new(tr(t)).class("tab"));
                ui.style(b, |s| s.padding = taffy::Rect { left: lp(14.0), right: lp(14.0), top: lp(8.0), bottom: lp(8.0) });
                ui.on_click(b, m(Msg::Tab(k)));
                b
            })
            .collect();
        let table = table();
        let mut page = SettingsPage {
            root,
            tabs,
            bodies: Vec::new(),
            rows: Vec::new(),
            table,
            preset: NodeId::dangling(),
            preset_presets: Vec::new(),
            profiles: None,
            profile: GfxProfiles::default(),
            radio: NodeId::dangling(),
            radio_list: None,
            radio_key: usize::MAX,
            corners: Vec::new(),
            update_button: NodeId::dangling(),
            update_text: NodeId::dangling(),
            mt: NodeId::dangling(),
            fov: NodeId::dangling(),
            fov_value: NodeId::dangling(),
        };
        let table = page.table.clone();
        for (tab, cols) in table.iter().enumerate() {
            let body = ui.row(root);
            kit::grow(ui, body);
            ui.set_scroll(body, ScrollAxes { x: false, y: true });
            ui.style(body, |s| {
                s.gap = taffy::Size { width: lp(14.0), height: lp(14.0) };
                s.align_items = Some(taffy::AlignItems::FlexStart);
                s.flex_wrap = taffy::FlexWrap::Wrap;
            });
            for (col, (title, rows)) in cols.iter().enumerate() {
                let c = kit::titled_card(ui, body, title);
                ui.style(c, |s| {
                    s.flex_grow = 1.0;
                    s.flex_basis = len(0.0);
                    s.min_size.width = len(340.0);
                });
                for (row, r) in rows.iter().enumerate() {
                    let at = At { tab, col, row };
                    page.build_row(ui, c, at, *r);
                }
            }
            page.bodies.push(body);
        }
        page
    }

    fn build_row(&mut self, ui: &mut Ui, c: NodeId, at: At, row: Row) {
        let line = |ui: &mut Ui, label: &str| {
            let r = ui.row(c);
            kit::gap(ui, r, 10.0);
            ui.style(r, |s| s.min_size.height = len(32.0));
            let l = ui.add(r, egui_retained::widgets::Text::wrapped(tr(label)));
            ui.add_class(l, "dim");
            ui.style(l, |s| {
                s.size.width = taffy::Dimension::percent(0.42);
                s.flex_shrink = 0.0;
            });
            r
        };
        let (node, input, value) = match row {
            Section(t) => {
                let n = kit::text(ui, c, &tr(t), "heading");
                ui.style(n, |s| s.margin.top = taffy::LengthPercentageAuto::length(8.0));
                (n, None, None)
            }
            Para(t, _) => (kit::para(ui, c, &tr(t), "faint"), None, None),
            Select(label, _, _, _) => {
                let r = line(ui, label);
                let s = ui.add(r, egui_retained::widgets::Select::new(Vec::new(), None).on_change(move |i| m(Msg::Choose(at, i))));
                kit::grow(ui, s);
                (r, Some(s), None)
            }
            Toggle(label, _, _) => {
                let t = ui.add(c, Checkbox::switch(false, tr(label)).on_change(move |v| m(Msg::Flag(at, v))));
                (t, Some(t), None)
            }
            Slider(label, _, min, max, step, def, _, _) => {
                let r = line(ui, label);
                let s = ui.add(r, egui_retained::widgets::Slider::new(def, min, max).step(step).on_change(move |v| m(Msg::Value(at, v))));
                let v = kit::text(ui, r, "", "strong");
                ui.style(v, |s| s.min_size.width = len(84.0));
                (r, Some(s), Some(v))
            }
            Button(label, icon, act, _) => {
                let b = ui.add(c, egui_retained::widgets::Button::new(tr(label)).icon(icon).class(if act == Act::Reset { "danger" } else if act == Act::Github { "ghost" } else { "button" }));
                ui.style(b, |s| s.align_self = Some(taffy::AlignSelf::FlexStart));
                ui.on_click(b, m(Msg::Act(act)));
                ui.set_name(b, &format!("settings-{act:?}"));
                (b, None, None)
            }
            Preset => {
                let r = line(ui, "Quality preset");
                let s = ui.add(r, egui_retained::widgets::Select::new(Vec::new(), None).on_change(|i| m(Msg::Preset(i))));
                kit::grow(ui, s);
                self.preset = s;
                (r, None, None)
            }
            Fov => {
                let r = line(ui, "Field of view");
                let s = ui.add(r, egui_retained::widgets::Slider::new(0.0, 0.0, 120.0).step(1.0).on_change(|v| m(Msg::Fov(v))));
                let v = kit::text(ui, r, "", "strong");
                ui.style(v, |s| s.min_size.width = len(84.0));
                self.fov = s;
                self.fov_value = v;
                (r, None, None)
            }
            MtStatus => {
                let n = kit::text(ui, c, "", "faint");
                self.mt = n;
                (n, None, None)
            }
            Profiles => {
                let r = line(ui, "Saved profile");
                let sel = ui.add(r, egui_retained::widgets::Select::new(Vec::new(), None).on_change(|i| m(Msg::ProfilePick(i))));
                kit::grow(ui, sel);
                let br = ui.row(c);
                kit::gap(ui, br, 8.0);
                let load = ui.add(br, egui_retained::widgets::Button::new(tr("Load")).icon("download"));
                kit::grow(ui, load);
                ui.on_click(load, m(Msg::ProfileLoad));
                let del = ui.add(br, egui_retained::widgets::Button::new(tr("Delete")).icon("delete").class("danger"));
                kit::grow(ui, del);
                ui.on_click(del, m(Msg::ProfileDelete));
                let name = ui.add(c, TextInput::new("").hint(tr("Profile name")).on_change(|s| m(Msg::ProfileName(s))));
                let save = ui.add(c, egui_retained::widgets::Button::new(tr("Save current graphics as profile")).icon("save").class("primary"));
                ui.on_click(save, m(Msg::ProfileSave));
                let note = kit::para(ui, c, "", "dim");
                self.profiles = Some((sel, name, note));
                (r, None, None)
            }
            Radio => {
                let list = ui.column(c);
                kit::gap(ui, list, 6.0);
                self.radio = list;
                let add = ui.add(c, egui_retained::widgets::Button::new(tr("Add a station")).icon("add"));
                ui.style(add, |s| s.align_self = Some(taffy::AlignSelf::FlexStart));
                ui.on_click(add, m(Msg::RadioAdd));
                (list, None, None)
            }
            Corner => {
                let r = line(ui, "Corner");
                let screen = ui.column(r);
                ui.style(screen, |s| {
                    s.size = taffy::Size { width: len(116.0), height: len(68.0) };
                    s.padding = taffy::Rect::length(5.0_f32);
                    s.gap = taffy::Size { width: lp(8.0), height: lp(8.0) };
                    s.flex_wrap = taffy::FlexWrap::Wrap;
                    s.flex_direction = taffy::FlexDirection::Row;
                });
                ui.visual(screen, Visual::new().background(Color32::from_white_alpha(12)).radius(6.0_f32).border(egui_retained::epaint::Stroke::new(1.0, Color32::from_white_alpha(30))));
                for name in ["top-left", "top-right", "bottom-left", "bottom-right"] {
                    let cell = ui.div(screen);
                    ui.style(cell, |s| s.size = taffy::Size { width: len(49.0), height: len(25.0) });
                    ui.visual(cell, Visual::new().background(Color32::from_white_alpha(20)).radius(3.0_f32).cursor(egui_retained::Cursor::Pointer).on_hover(Visual::new().background(Color32::from_white_alpha(50))).on_select(Visual::new().background(ACCENT)));
                    ui.on_click(cell, m(Msg::Corner(name)));
                    self.corners.push((name, cell));
                }
                (r, None, None)
            }
            Updates => {
                let r = ui.row(c);
                kit::gap(ui, r, 10.0);
                let b = ui.add(r, egui_retained::widgets::Button::new(tr("Check now")).icon("refresh"));
                ui.on_click(b, m(Msg::Act(Act::CheckUpdates)));
                let t = kit::text(ui, r, "", "dim");
                self.update_button = b;
                self.update_text = t;
                (r, None, None)
            }
        };
        self.rows.push(Built { at, row, node, input, value });
    }

    pub fn sync(&mut self, ui: &mut Ui, l: &Launcher) {
        let tab = l.pages.settings_tab.min(TABS.len() - 1);
        for (k, n) in self.tabs.iter().enumerate() {
            ui.set_selected(*n, k == tab);
        }
        for (k, n) in self.bodies.iter().enumerate() {
            ui.set_visible(*n, k == tab);
        }
        let s = &l.state.settings;
        for b in &self.rows {
            if b.at.tab != tab {
                continue;
            }
            match b.row {
                Para(_, show) => {
                    ui.set_visible(b.node, show(s));
                }
                Select(_, key, opts, show) => {
                    ui.set_visible(b.node, show(s));
                    if !show(s) {
                        continue;
                    }
                    let (labels, sel) = select_state(s, key, opts);
                    if let Some(i) = b.input {
                        ui.set_select(i, Some(labels.into_iter().map(|x| tr(&x)).collect()), Some(sel));
                    }
                }
                Toggle(_, key, show) => {
                    ui.set_visible(b.node, show(s));
                    if let Some(i) = b.input {
                        ui.set_checked(i, get(s, key).as_bool().unwrap_or(false));
                    }
                }
                Slider(_, key, _, _, _, def, fmt, show) => {
                    ui.set_visible(b.node, show(s));
                    let v = num(get(s, key)).unwrap_or(def);
                    if let Some(i) = b.input {
                        ui.set_slider(i, v);
                    }
                    if let Some(t) = b.value {
                        ui.set_text(t, &tr(&fmt(v)));
                    }
                }
                Button(_, _, _, show) => {
                    ui.set_visible(b.node, show(s));
                }
                _ => {}
            }
        }
        // the parts of their own
        if tab == 0 {
            let presets = core::graphics_presets_for(mode(s));
            let sel = presets.iter().position(|p| preset_matches(s, &p.1)).unwrap_or(presets.len());
            let mut labels: Vec<String> = presets.iter().map(|p| tr(&p.0)).collect();
            labels.push(tr("Custom"));
            self.preset_presets = presets.iter().map(|p| (p.0.to_string(), p.1.clone())).collect();
            ui.set_select(self.preset, Some(labels), Some(sel));
            if let Some((sel, _name, note)) = self.profiles {
                let names = self.profile.list.get_or_insert_with(|| core::graphics_profiles().into_keys().collect()).clone();
                let labels = if names.is_empty() { vec![tr("No saved profiles")] } else { names };
                ui.set_select(sel, Some(labels), Some(self.profile.sel));
                ui.set_text(note, &self.profile.msg);
                ui.set_visible(note, !self.profile.msg.is_empty());
            }
        }
        if tab == 2 {
            let key = fov_key(s);
            let v = num(get(s, key)).unwrap_or(0.0);
            ui.set_slider(self.fov, v);
            ui.set_text(self.fov_value, &if v < 20.0 { tr("Default") } else { format!("{v:.0}°") });
        }
        if tab == 3 {
            let list = self.radio_list.get_or_insert_with(crate::radio::own_stations).clone();
            if self.radio_key != list.len() {
                self.radio_key = list.len();
                ui.clear(self.radio);
                for (k, (name, url)) in list.iter().enumerate() {
                    let r = ui.row(self.radio);
                    kit::gap(ui, r, 6.0);
                    let n = ui.add(r, TextInput::new(name.clone()).hint(tr("Name")).on_change(move |v| m(Msg::RadioName(k, v))));
                    ui.style(n, |s| s.size.width = taffy::Dimension::percent(0.3));
                    let u = ui.add(r, TextInput::new(url.clone()).hint("https://…").on_change(move |v| m(Msg::RadioUrl(k, v))));
                    kit::grow(ui, u);
                    let d = ui.add(r, Button::new("").icon("delete").class("ghost"));
                    ui.set_tooltip(d, Some(tr("Remove this station")));
                    ui.on_click(d, m(Msg::RadioRemove(k)));
                }
            }
        }
        if tab == 5 {
            let cur = get(s, "navigator_corner").as_str().unwrap_or("bottom-left");
            for (name, n) in &self.corners {
                ui.set_selected(*n, *name == cur);
            }
            use crate::updater::Status;
            let st = l.update.status();
            let busy = matches!(st, Status::Checking | Status::Downloading { .. } | Status::Installing(_) | Status::WaitingForInstaller(_) | Status::Restarting(_));
            ui.set_text(self.update_button, &tr(if busy { "Checking…" } else { "Check now" }));
            ui.set_disabled(self.update_button, busy);
            let text = match &st {
                Status::UpToDate if crate::updater::is_test_build(crate::updater::current_version()) => format!("{} is a test build: it is not updated", crate::updater::current_version()),
                Status::UpToDate => format!("{} is the latest version", crate::updater::current_version()),
                Status::Available(rel) => format!("{} is available", rel.version),
                Status::Failed(_) => "The last check failed".to_string(),
                _ => format!("This is openOMSI {}", crate::updater::current_version()),
            };
            ui.set_text(self.update_text, &text);
            let mt_on = get(s, "machine_translation").as_bool().unwrap_or(false);
            let st = crate::mt::status();
            let show = mt_on && !st.is_empty() && st != "Ready";
            ui.set_visible(self.mt, show);
            ui.set_text(self.mt, &st);
        }
    }

    fn row(&self, at: At) -> Option<Row> {
        self.table.get(at.tab)?.get(at.col)?.1.get(at.row).copied()
    }
}

fn num(v: &Value) -> Option<f32> {
    v.as_f64().map(|f| f as f32).or_else(|| v.as_str().and_then(|s| s.parse().ok()))
}

fn fov_key(s: &Value) -> &'static str {
    if triple(s) && !get(s, "vr").as_bool().unwrap_or(false) { "triple_fov_deg" } else { "fov" }
}

fn options(s: &Value, o: Opts) -> Vec<(String, String)> {
    match o {
        Opts::Fixed(f) => f.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect(),
        Opts::Of(f) => f(s),
    }
}

fn current(s: &Value, key: &str) -> String {
    match get(s, key) {
        Value::String(x) => x.clone(),
        Value::Bool(b) => (*b as u8).to_string(),
        Value::Number(n) => n.to_string(),
        _ => String::new(),
    }
}

/// The labels of a choice and which is chosen (a value written by hand gets an entry).
fn select_state(s: &Value, key: &str, o: Opts) -> (Vec<String>, usize) {
    let opts = options(s, o);
    let cur = current(s, key);
    let same = |v: &str| v == cur || v.parse::<f64>().ok().zip(cur.parse::<f64>().ok()).is_some_and(|(a, b)| (a - b).abs() < 1e-6);
    let mut labels: Vec<String> = opts.iter().map(|x| x.1.clone()).collect();
    let sel = match opts.iter().position(|x| same(&x.0)) {
        Some(i) => i,
        None if !cur.is_empty() => {
            labels.push(cur);
            labels.len() - 1
        }
        None => 0,
    };
    (labels, sel)
}

fn preset_matches(s: &Value, p: &Value) -> bool {
    p.as_object().is_some_and(|o| {
        o.iter().all(|(k, v)| {
            let cur = get(s, k);
            cur == v || num(cur).zip(num(v)).is_some_and(|(a, b)| (a - b).abs() < 1e-6)
        })
    })
}

/// A slider's value as the settings keep it: rounded to its step.
fn stored(key: &str, v: f32, step: f32) -> Value {
    let r = (v / step).round() * step;
    match key {
        "led_glow" | "wheel_range" | "pad_steer_smooth" | "look_smoothing_ms" => json!(r.round() as i64),
        "wheel_lock" => json!(if r < 45.0 { 0.0 } else { r.round() as f64 }),
        _ => json!(((r as f64) * 1000.0).round() / 1000.0),
    }
}

pub(in crate::launcher) fn handle(l: &mut Launcher, msg: Msg) {
    fn page(l: &mut Launcher) -> Option<&mut SettingsPage> {
        l.gui.as_mut().and_then(|g| g.settings.as_mut())
    }
    let dirty = |l: &mut Launcher| l.state.settings_dirty = 0.3;
    match msg {
        Msg::Tab(k) => l.pages.settings_tab = k,
        Msg::Choose(at, i) => {
            let Some(Row::Select(_, key, opts, _)) = page(l).and_then(|p| p.row(at)) else { return };
            let all = options(&l.state.settings, opts);
            let Some((v, _)) = all.get(i).cloned() else { return };
            let s = &mut l.state.settings;
            s[key] = match get(s, key) {
                Value::Bool(_) => json!(v == "1"),
                Value::Number(_) => v.parse::<f64>().map(|f| if f.fract() == 0.0 { json!(f as i64) } else { json!(f) }).unwrap_or(json!(v)),
                _ => json!(v),
            };
            if key == "language" {
                crate::ui_language(&v);
            }
            dirty(l);
        }
        Msg::Flag(at, v) => {
            let Some(Row::Toggle(_, key, _)) = page(l).and_then(|p| p.row(at)) else { return };
            l.state.settings[key] = json!(v);
            if key == "machine_translation" {
                crate::mt::enable(v);
            }
            dirty(l);
        }
        Msg::Value(at, v) => {
            let Some(Row::Slider(_, key, _, _, step, _, _, _)) = page(l).and_then(|p| p.row(at)) else { return };
            l.state.settings[key] = stored(key, v, step);
            if key == "triple_distance_mm" {
                l.state.settings["triple_fov_deg"] = json!(0.0);
            }
            dirty(l);
        }
        Msg::Fov(v) => {
            let key = fov_key(&l.state.settings);
            l.state.settings[key] = json!(if v < 20.0 { 0.0 } else { v.round() as f64 });
            dirty(l);
        }
        Msg::Preset(i) => {
            let preset = page(l).and_then(|p| p.preset_presets.get(i).cloned());
            if let Some((_, Value::Object(o))) = preset {
                for (k, v) in o {
                    l.state.settings[k.as_str()] = v;
                }
                dirty(l);
            }
        }
        Msg::Act(a) => match a {
            Act::Keys => {
                l.pages.controls_tab = 0;
                l.go(Page::Controls);
            }
            Act::Pads => {
                l.pages.controls_tab = 1;
                l.go(Page::Controls);
            }
            Act::WheelReset => {
                let s = &mut l.state.settings;
                s["wheel_range"] = json!(900.0);
                s["wheel_lock"] = json!(0.0);
                s["ff_invert"] = json!(false);
                s["ff_enabled"] = json!(true);
                s["ff_road_vib"] = json!(1.0);
                s["ff_engine_vib"] = json!(1.0);
                s["ff_fade"] = json!(0.28);
                dirty(l);
            }
            Act::SeatReset => {
                for k in ["seat_x", "seat_y", "seat_z", "seat_pitch_deg"] {
                    l.state.settings[k] = json!(0.0);
                }
                dirty(l);
            }
            Act::TrackReset => {
                for k in ["head_tracking_yaw_sens", "head_tracking_pitch_sens", "head_tracking_roll_sens", "head_tracking_x_sens", "head_tracking_y_sens", "head_tracking_z_sens"] {
                    l.state.settings[k] = json!(15.0);
                }
                for k in ["head_tracking_invert_yaw", "head_tracking_invert_pitch", "head_tracking_invert_roll", "head_tracking_invert_x", "head_tracking_invert_y", "head_tracking_invert_z"] {
                    l.state.settings[k] = json!(false);
                }
                dirty(l);
            }
            Act::CheckUpdates => l.update.check(),
            Act::Github => crate::updater::open_url(crate::updater::REPO_URL),
            Act::Reset => l.pages.confirm_reset = true,
        },
        Msg::ProfilePick(i) => {
            if let Some(p) = page(l) {
                p.profile.sel = i;
                if let Some(n) = p.profile.list.as_ref().and_then(|v| v.get(i)) {
                    p.profile.name = n.clone();
                }
            }
        }
        Msg::ProfileName(s) => {
            if let Some(p) = page(l) {
                p.profile.name = s;
            }
        }
        Msg::ProfileLoad => {
            let name = page(l).and_then(|p| p.profile.list.as_ref().and_then(|v| v.get(p.profile.sel).cloned()));
            let Some(name) = name else { return };
            let msg = match core::graphics_profiles().get(&name) {
                Some(pr) => {
                    core::apply_graphics_profile(pr, &mut l.state.settings);
                    dirty(l);
                    format!("Loaded \"{name}\".")
                }
                None => format!("\"{name}\" is gone."),
            };
            if let Some(p) = page(l) {
                p.profile.msg = msg;
            }
        }
        Msg::ProfileDelete => {
            let name = page(l).and_then(|p| p.profile.list.as_ref().and_then(|v| v.get(p.profile.sel).cloned()));
            let Some(name) = name else { return };
            let msg = match core::delete_graphics_profile(&name) {
                Ok(()) => format!("Deleted \"{name}\"."),
                Err(e) => format!("{e:#}"),
            };
            if let Some(p) = page(l) {
                p.profile.msg = msg;
                p.profile.list = None;
                p.profile.sel = 0;
            }
        }
        Msg::ProfileSave => {
            let name = page(l).map(|p| p.profile.name.clone()).unwrap_or_default();
            let res = core::save_graphics_profile(&name, &l.state.settings);
            if let Some(p) = page(l) {
                p.profile.msg = match res {
                    Ok(n) => {
                        p.profile.list = None;
                        if let Some(i) = core::graphics_profiles().keys().position(|k| *k == n) {
                            p.profile.sel = i;
                        }
                        format!("Saved \"{n}\".")
                    }
                    Err(e) => format!("{e:#}"),
                };
            }
        }
        Msg::RadioName(k, v) => radio_edit(l, k, v, true),
        Msg::RadioUrl(k, v) => radio_edit(l, k, v, false),
        Msg::RadioRemove(k) => {
            if let Some(p) = page(l) {
                if let Some(list) = p.radio_list.as_mut() {
                    if k < list.len() {
                        list.remove(k);
                    }
                    if let Err(e) = crate::radio::save_stations(list) {
                        log::warn!("radio.cfg: {e}");
                    }
                }
                p.radio_key = usize::MAX;
            }
        }
        Msg::RadioAdd => {
            if let Some(p) = page(l) {
                p.radio_list.get_or_insert_with(crate::radio::own_stations).push((String::new(), String::new()));
                p.radio_key = usize::MAX;
            }
        }
        Msg::Corner(c) => {
            l.state.settings["navigator_corner"] = json!(c);
            dirty(l);
        }
    }
}

fn radio_edit(l: &mut Launcher, k: usize, v: String, name: bool) {
    let Some(p) = l.gui.as_mut().and_then(|g| g.settings.as_mut()) else { return };
    let list = p.radio_list.get_or_insert_with(crate::radio::own_stations);
    if let Some(e) = list.get_mut(k) {
        if name {
            e.0 = v;
        } else {
            e.1 = v;
        }
        if let Err(err) = crate::radio::save_stations(list) {
            log::warn!("radio.cfg: {err}");
        }
    }
}
