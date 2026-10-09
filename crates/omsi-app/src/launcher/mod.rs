//! The launcher: where a duty is put together (bus, map, line and tour, time, weather),
//! the driver's profile, the settings and key bindings, the running games and the mods.
//!
//! It is a window of the game binary itself, drawn with wgpu: the chosen bus stands in a
//! picture drawn by the game's renderer (see `showroom`) whenever it changes, and the
//! interface - flat and dark, every control custom - is drawn with `omsi-ui` straight
//! onto the window. The data side (content
//! lists, timetables, profiles, installs, running games) is `omsi-launcher-core`, the same
//! functions `omsi-launcher --cli` offers a terminal.

pub(crate) mod drive;
mod gui;
pub(crate) mod mapview;
pub mod mobile;
pub mod phone;
mod multiplayer;
pub(crate) mod pages;
mod showroom;
mod season;
mod state;
#[cfg_attr(not(target_os = "android"), allow(unused_imports))]
pub(crate) use state::crash_of;
mod timetable;
mod ui;
mod update;

use omsi_launcher_lib as core;
use omsi_render::{Renderer, SurfaceState};
use std::sync::Arc;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Page {
    Drive,
    Multiplayer,
    Profile,
    Settings,
    Controls,
    Sessions,
    Mods,
    Tutorials,
    Timetable,
    Setup,
}

const PAGES: [(Page, &str, &str); 10] = [
    (Page::Drive, "Drive", "directions_bus"),
    (Page::Multiplayer, "Multiplayer", "groups"),
    (Page::Profile, "Profile", "badge"),
    (Page::Settings, "Settings", "tune"),
    (Page::Controls, "Controls", "keyboard"),
    (Page::Sessions, "Sessions", "sports_esports"),
    (Page::Mods, "Mods", "extension"),
    (Page::Tutorials, "Tutorials", "help"),
    (Page::Timetable, "Timetable", "schedule"),
    (Page::Setup, "Setup", "folder_open"),
];

#[cfg(not(target_os = "android"))]
type Clipboard = arboard::Clipboard;

/// A phone: text copied in the launcher can be pasted in it (the system's clipboard is
/// Java's).
#[cfg(target_os = "android")]
struct Clipboard(String);

#[cfg(target_os = "android")]
impl Clipboard {
    fn new() -> Result<Clipboard, ()> {
        Ok(Clipboard(String::new()))
    }
    fn get_text(&mut self) -> Result<String, ()> {
        Ok(self.0.clone())
    }
    fn set_text(&mut self, t: String) -> Result<(), ()> {
        self.0 = t;
        Ok(())
    }
}

/// How often the launcher made its device again after losing it (see `recover_device`).
static LAUNCHER_RECOVERIES: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

pub struct Launcher {
    instance: wgpu::Instance,
    window: Option<Arc<Window>>,
    surface: Option<SurfaceState<'static>>,
    renderer: Option<Renderer>,
    state: state::State,
    showroom: showroom::Showroom,
    page: Page,
    pub drive: drive::DriveView,
    /// The launcher made for a phone (see `phone`).
    pub phone: phone::PhoneView,
    pub pages: pages::PagesView,
    pub mp: multiplayer::MultiplayerView,
    last: Instant,
    clipboard: Option<Clipboard>,
    exit_after: Option<f32>,
    shot: Option<(f32, std::path::PathBuf)>,
    started: Instant,
    /// Until the first frame is on the screen: the preview loads no bus before it.
    first_frame: bool,
    /// `OMSI_LAUNCHER_INPUT="t=2 click 400,300; t=3 type Bauern; t=4 key Enter; t=5 shot a.png;
    /// t=6 wheel -3; t=7 move 900,400"`: the window worked by a script (logical pixels).
    script: Vec<(f32, String)>,
    /// The chosen map's picture (see `mapview`): where it is this frame and its texture.
    pub mapview: mapview::MapView,
    /// The window has the keyboard / is hidden: without focus it is drawn ten times a
    /// second, hidden not at all (a game started from it is being played).
    focused: bool,
    occluded: bool,
    /// The player came back to the launcher's window while a game runs (clicked it, Alt+Tab):
    /// it is drawn and answers again until the game has the focus back. Before, it stood
    /// still the whole game long, and the session code could not be copied (#825).
    awake_in_game: bool,
    /// The last mouse or key event (an idle launcher draws less often: it kept the GPU busy
    /// at the screen's rate doing nothing).
    last_input: Instant,
    /// A phone's fingers (their pinch zooms the bus), its storage browser, and whether the
    /// on-screen keyboard is up (see `mobile`).
    fingers: mobile::Fingers,
    pub browser: Option<mobile::Browser>,
    ime: bool,
    /// Updates from the GitHub releases (see `crate::updater`, `update.rs`).
    pub update: crate::updater::Updater,
    #[cfg(not(target_os = "android"))]
    discord: Option<crate::discord::Discord>,
    #[cfg(not(target_os = "android"))]
    discord_next_try: Instant,
    /// Background drop of the graphics device given up while a game runs (see `frame`).
    /// Joined before a new device is opened so the two do not meet on the card.
    gpu_rest_drop: Option<std::thread::JoinHandle<()>>,
    /// The interface (on egui_retained, see `gui`); taken out while a frame is made.
    gui: Option<gui::Gui>,
    /// The last key pressed (its code and the Shift/Ctrl/Alt chord) and the modifiers held,
    /// for a key binding the new interface waits for.
    gui_key: Option<(winit::keyboard::KeyCode, i32)>,
    gui_mods: winit::keyboard::ModifiersState,
}

/// Run the launcher window until it is closed.
pub fn run(instance: wgpu::Instance) -> anyhow::Result<()> {
    let event_loop = EventLoop::new()?;
    let mut app = Launcher::new(instance);
    event_loop.run_app(&mut app)?;
    Ok(())
}

impl Launcher {
    /// The launcher, not yet in a window (that comes with `resumed`).
    pub fn new(instance: wgpu::Instance) -> Launcher {
    let started = Instant::now();
    core::cleanup();
    let mut app = Launcher {
        instance,
        window: None,
        surface: None,
        renderer: None,
        state: state::State::new(),
        showroom: showroom::Showroom::new(),
        page: Page::Drive,
        drive: drive::DriveView::default(),
        phone: phone::PhoneView::default(),
        pages: pages::PagesView::default(),
        mp: multiplayer::MultiplayerView::default(),
        last: Instant::now(),
        clipboard: Clipboard::new().ok(),
        // OMSI_LAUNCHER_EXIT=secs, OMSI_LAUNCHER_SHOT=secs:file.png, OMSI_LAUNCHER_PAGE=mods:
        // looking at the window without a person at it
        exit_after: omsi_cfg::flags::OMSI_LAUNCHER_EXIT.parse(),
        shot: omsi_cfg::flags::OMSI_LAUNCHER_SHOT.var().and_then(|v| v.split_once(':').map(|(t, f)| (t.parse().unwrap_or(5.0), std::path::PathBuf::from(f)))),
        started,
        first_frame: true,
        script: omsi_cfg::flags::OMSI_LAUNCHER_INPUT.var()
            .map(|v| {
                v.split(';')
                    .filter_map(|c| {
                        let c = c.trim();
                        let (t, rest) = c.strip_prefix("t=")?.split_once(' ')?;
                        Some((t.parse().ok()?, rest.trim().to_string()))
                    })
                    .collect()
            })
            .unwrap_or_default(),
        mapview: mapview::MapView::new(),
        focused: true,
        occluded: false,
        awake_in_game: false,
        last_input: Instant::now(),
        fingers: Default::default(),
        browser: None,
        ime: false,
        update: Default::default(),
        #[cfg(not(target_os = "android"))]
        discord: None,
        #[cfg(not(target_os = "android"))]
        discord_next_try: Instant::now(),
        gpu_rest_drop: None,
        gui: Some(gui::Gui::new()),
        gui_key: None,
        gui_mods: Default::default(),
    };
    // after an update: the files it set aside go, and the launcher says what happened
    #[cfg(not(target_os = "android"))]
    crate::updater::cleanup_after_update();
    if let Some(v) = crate::updater::just_updated() {
        log::info!("update: this start follows the update to {v}");
        app.update.updated = Some((v, Instant::now()));
    }
    // no original installation found anywhere: the launcher still opens, on Setup, and says
    // what it needs (only starting a session needs the game)
    if omsi_cfg::missing_original_essentials(std::path::Path::new(&app.state.config.root)).len() > 0 {
        app.page = Page::Setup;
        let why = state::root_problem(&app.state.config.root);
        app.state.set_status(why, true);
    }
    if let Some(p) = omsi_cfg::flags::OMSI_LAUNCHER_PAGE.var().map(str::to_string) {
        if let Some((pg, _, _)) = PAGES.iter().find(|(_, n, _)| n.eq_ignore_ascii_case(p.split(':').next().unwrap_or(""))) {
            app.page = *pg;
            // (the phone's tab for it)
            app.phone.tab = match pg {
                Page::Drive => phone::Tab::Play,
                Page::Multiplayer => phone::Tab::Online,
                Page::Mods => phone::Tab::Mods,
                other => {
                    app.phone.page = Some(*other);
                    phone::Tab::More
                }
            };
        }
        // (`OMSI_LAUNCHER_PAGE=more`, `=sheet-bus` …: the phone's More, or one of its sheets)
        match p.as_str() {
            "more" => app.phone.tab = phone::Tab::More,
            "sheet-map" => app.phone.sheet = Some(phone::Sheet::Map),
            "sheet-bus" => app.phone.sheet = Some(phone::Sheet::Bus),
            "sheet-duty" => app.phone.sheet = Some(phone::Sheet::Duty),
            "sheet-time" => app.phone.sheet = Some(phone::Sheet::Time),
            "sheet-livery" => app.phone.sheet = Some(phone::Sheet::Livery),
            _ => {}
        }
        if let Some(step) = p.split(':').nth(1).and_then(|s| s.parse::<usize>().ok()) {
            // (the Drive page's second part is which of its three steps: drive:2 the map)
            app.drive.tab = step.min(2);
            // (the Controls and Settings pages' second part is their tab: controls:1 the game
            // controllers, settings:3 Sound)
            app.pages.controls_tab = step;
            app.pages.settings_tab = step.min(pages::SETTINGS_TABS.len() - 1);
        }
    }
    app
    }

    /// Everything made on the graphics device goes with it: the interface's renderer and its
    /// textures, the bus preview, the map picture (and the map's own drawing). A number kept over a device made anew pointed past the
    /// new device's textures, and the map was drawn with the font atlas instead: the Drive
    /// page's map full of the interface's words after a game (the launcher gives its device
    /// up while one runs) or a lost device.
    fn drop_gpu(&mut self) {
        self.showroom = showroom::Showroom::new();
        self.mapview.drop_gpu();
        if let Some(g) = self.gui.as_mut() {
            g.drop_gpu();
        }
    }

    /// The window, its surface and the renderer, given up for the game (a phone plays in the
    /// launcher's window).
    #[cfg_attr(not(target_os = "android"), allow(dead_code))]
    pub fn release_window(&mut self) -> Option<Arc<Window>> {
        self.pages.pads.release_io();
        self.surface = None;
        self.drop_gpu();
        if let Some(h) = self.gpu_rest_drop.take() {
            let _ = h.join();
        }
        self.renderer = None;
        self.ime = false;
        self.window.take()
    }

    /// Back from a game: the window again (the launcher draws into it from the next resume).
    #[cfg_attr(not(target_os = "android"), allow(dead_code))]
    pub fn adopt_window(&mut self, window: Arc<Window>) {
        self.window = Some(window);
        self.surface = None;
        self.renderer = None;
        self.state.poll_now();
        self.state.load_profile();
    }

    /// The surface for the window the launcher has (created again after the app was in the
    /// background: a phone takes the window's surface away meanwhile).
    fn make_surface(&mut self) {
        let Some(window) = self.window.clone() else { return };
        if self.renderer.is_none() {
            // Finish dropping the device given up for a game before opening another one.
            if let Some(h) = self.gpu_rest_drop.take() {
                let _ = h.join();
            }
            let settings = crate::settings::Settings::load();
            let renderer = match crate::startup::window_renderer(&mut self.instance, &window, showroom_options(&settings)) {
                Ok(r) => r,
                Err(e) => {
                    crate::startup::fatal_message(&format!("openOMSI cannot draw on this computer: {e:#}"));
                    std::process::exit(1);
                }
            };
            self.renderer = Some(renderer);
        }
        let Some(renderer) = self.renderer.as_ref() else { return };
        let size = window.inner_size();
        self.surface = SurfaceState::new_with(&self.instance, window.clone(), renderer, size.width.max(1), size.height.max(1), true).ok();
        self.last = Instant::now();
    }
}

impl ApplicationHandler for Launcher {
    fn device_event(&mut self, _event_loop: &ActiveEventLoop, _id: winit::event::DeviceId, event: DeviceEvent) {
        if matches!(event, DeviceEvent::Added | DeviceEvent::Removed) {
            if let Some(io) = self.pages.pads.io.as_ref() {
                io.refresh();
            }
        }
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        // (a phone: the app went to the background and its window's surface goes with it)
        self.surface = None;
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            if self.surface.is_none() {
                self.make_surface();
            }
            return;
        }
        // (`OMSI_LAUNCHER_SIZE=WxH`: another window size, for looking at the layout)
        let asked = omsi_cfg::flags::OMSI_LAUNCHER_SIZE.var().and_then(|v| v.split_once('x').and_then(|(a, b)| Some((a.parse::<f64>().ok()?, b.parse::<f64>().ok()?))));
        let (fit, at) = match asked {
            Some((iw, ih)) => (winit::dpi::LogicalSize::new(iw, ih), None),
            None => crate::startup::fit_window(event_loop, 1440.0, 880.0),
        };
        let mut attrs = Window::default_attributes().with_title("openOMSI").with_window_icon(crate::startup::window_icon()).with_inner_size(fit);
        if !mobile::mobile() {
            // (no bigger than the window fitted to the screen: a small one at 150 % has less)
            attrs = attrs.with_min_inner_size(winit::dpi::LogicalSize::new(1080.0f64.min(fit.width), 680.0f64.min(fit.height)));
            if let Some(at) = at {
                attrs = attrs.with_position(at);
            }
        }
        if omsi_cfg::flags::OMSI_BACKGROUND.is_set() {
            // (a test window is never shown either: OMSI_LAUNCHER_SHOT draws into a texture of
            // its own, so the pictures of a hidden window come out the same, and nothing pops
            // up on the screen of whoever runs the checks)
            attrs = attrs.with_active(false).with_visible(false);
        }
        let window = match event_loop.create_window(attrs) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                crate::startup::fatal_message(&format!("openOMSI cannot open its window: {e}"));
                event_loop.exit();
                return;
            }
        };
        let settings = crate::settings::Settings::load();
        let renderer = match crate::startup::window_renderer(&mut self.instance, &window, showroom_options(&settings)) {
            Ok(r) => r,
            Err(e) => {
                crate::startup::fatal_message(&format!("openOMSI cannot draw on this computer: {e:#}"));
                event_loop.exit();
                return;
            }
        };
        let size = window.inner_size();
        let surface = match SurfaceState::new_with(&self.instance, window.clone(), &renderer, size.width, size.height, true) {
            Ok(s) => s,
            Err(e) => {
                crate::startup::fatal_message(&format!("openOMSI cannot draw into its window: {e:#}"));
                event_loop.exit();
                return;
            }
        };
        log::info!("launcher window {}x{} (scale {:.2}), adapter {}", size.width, size.height, window.scale_factor(), renderer.adapter_name);
        self.window = Some(window);
        self.surface = Some(surface);
        self.renderer = Some(renderer);
        self.last = Instant::now();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let scale = self.ui_scale();
        if !matches!(event, WindowEvent::RedrawRequested) {
            self.last_input = Instant::now();
        }
        // (the game opens on the screen the launcher stands on, #1959)
        if matches!(event, WindowEvent::Moved(_) | WindowEvent::Resized(_) | WindowEvent::Focused(true)) {
            if let Some(w) = self.window.as_ref() {
                let (at, size) = (w.outer_position().ok(), w.outer_size());
                core::instances::set_screen_at(at.map(|p| (p.x + size.width as i32 / 2, p.y + size.height as i32 / 2)));
            }
        }
        // the new interface takes the pointer, the keys and the text (and two fingers' pinch
        // zooms the bus)
        if let (true, WindowEvent::Touch(t)) = (self.gui.is_some(), &event) {
            self.gui_pinch(*t, scale);
        }
        if let Some(g) = self.gui.as_mut() {
            let input = matches!(
                event,
                WindowEvent::CursorMoved { .. } | WindowEvent::CursorLeft { .. } | WindowEvent::MouseInput { .. } | WindowEvent::MouseWheel { .. } | WindowEvent::KeyboardInput { .. } | WindowEvent::ModifiersChanged(_) | WindowEvent::Touch(_) | WindowEvent::Ime(_)
            );
            if input || matches!(event, WindowEvent::Focused(_)) {
                g.input.on_event(&event, scale);
                // (the raw key and the modifiers held, for a key binding waited for)
                match &event {
                    WindowEvent::ModifiersChanged(m) => self.gui_mods = m.state(),
                    WindowEvent::KeyboardInput { event: k, .. } if k.state.is_pressed() && !k.repeat => {
                        if let winit::keyboard::PhysicalKey::Code(code) = k.physical_key {
                            let md = self.gui_mods;
                            self.gui_key = Some((code, omsi_content::input::chord(md.shift_key(), md.control_key(), md.alt_key())));
                        }
                    }
                    _ => {}
                }
                if g.input.wants_paste() {
                    if let Some(t) = self.clipboard.as_mut().and_then(|c| c.get_text().ok()) {
                        g.input.paste(t);
                    }
                }
                if let Some(w) = self.window.as_ref() {
                    w.request_redraw();
                }
                if input {
                    return;
                }
            }
        }
        match event {
            WindowEvent::CloseRequested => {
                self.pages.pads.cancel_feedback_test();
                showroom::clear_placing_mark();
                event_loop.exit();
            }
            WindowEvent::Focused(f) => self.set_focus(f),
            WindowEvent::Occluded(o) => {
                self.occluded = o;
                if o {
                    self.pages.pads.cancel_feedback_test();
                } else if self.renderer.is_none() {
                    // Back in view without a graphics device (a game just ended, or the
                    // window was covered while resting): draw again so the device is opened
                    // without waiting for the next slow occluded tick.
                    if let Some(w) = self.window.as_ref() {
                        w.request_redraw();
                    }
                }
            }
            WindowEvent::Resized(s) => {
                if let (Some(sf), Some(r)) = (self.surface.as_mut(), self.renderer.as_ref()) {
                    sf.resize(r, s.width, s.height);
                }
            }
            WindowEvent::DroppedFile(path) => {
                // a mod folder or supported archive dropped on the window is installed
                self.page = Page::Mods;
                self.state.install(path.to_string_lossy().to_string());
            }
            WindowEvent::HoveredFile(_) => self.pages.drop_hover = true,
            WindowEvent::HoveredFileCancelled => self.pages.drop_hover = false,
            WindowEvent::RedrawRequested => {
                self.frame(event_loop);
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.check_exit(event_loop);
        // (a screenshot asked for by a script is drawn even when hidden, and so is the frame
        // that gives the graphics device up again when the game is back in front: the
        // game's window hides the launcher's then, and drawn nothing, it kept the device)
        // (likewise the frame that opens it again for a window brought forward)
        let resting = !mobile::mobile() && self.renderer.is_some() && self.state.in_game() && !self.awake();
        let waking = !mobile::mobile() && self.renderer.is_none() && self.state.in_game() && self.awake();
        // Device given up while a game runs: still need a redraw when the game ends so the
        // device is opened again (Occluded alone used to leave the resting picture forever).
        let resume_needed = !mobile::mobile() && self.renderer.is_none() && !self.state.in_game();
        let occluded = self.occluded && self.shot.is_none() && !resting && !waking && !resume_needed
            && !self.script.iter().any(|(_, c)| c.starts_with("shot"));
        let interval = if occluded {
            0.5
        } else if !self.focused && !omsi_cfg::flags::OMSI_BACKGROUND.is_set() {
            0.1
        } else if self.last_input.elapsed().as_secs_f32() > 3.0 && self.script.is_empty() {
            // idle: 20 frames a second keep the preview and the progress bars moving
            0.05
        } else {
            0.0
        };
        let since = self.last.elapsed().as_secs_f32();
        if interval > 0.0 && since < interval {
            event_loop.set_control_flow(winit::event_loop::ControlFlow::WaitUntil(self.last + std::time::Duration::from_secs_f32(interval)));
            return;
        }
        event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
        if occluded {
            // nothing to draw: keep the data side going (polls, installs, the script)
            let dt = since.min(1.0);
            self.last = Instant::now();
            self.run_script();
            self.state.update(dt);
            #[cfg(not(target_os = "android"))]
            self.update_discord();
            self.update_tick(event_loop);
            self.check_exit(event_loop);
            // A game may have ended on this tick: wake so `frame` opens the device again.
            if !mobile::mobile() && self.renderer.is_none() && !self.state.in_game() {
                if let Some(w) = self.window.as_ref() {
                    w.request_redraw();
                }
            }
        } else if let Some(w) = self.window.as_ref() {
            w.request_redraw();
        }
    }
}

impl Launcher {
    /// Physical pixels per interface pixel: the screen's scale, times a zoom that makes the
    /// interface (laid out for a 1440 x 880 window) grow with a bigger window and shrink a
    /// little with a smaller one, so that it fills the window the same way at any size.
    fn ui_scale(&self) -> f32 {
        let Some(w) = self.window.as_ref() else { return 1.0 };
        let dpi = w.scale_factor() as f32;
        let s = w.inner_size();
        let (lw, lh) = (s.width as f32 / dpi, s.height as f32 / dpi);
        if mobile::mobile() {
            // a phone held across: the text at least at the system's own size - smaller, it
            // was hard to read and the buttons hard to hit (the pages scroll where the screen
            // is lower than they are, and lay themselves out for its width), a tablet larger
            return dpi * (lh / 400.0).clamp(1.0, 1.35);
        }
        // (the height counts a little less: on a wide, low screen - 2560 x 1080 - the text
        // stayed the size of a 1440 x 880 window's, tiny across the width; the pages scroll or
        // keep their width, see `draw_ui`)
        dpi * (lw / 1440.0).min(lh / 820.0).clamp(0.8, 2.2)
    }

    /// The graphics device was lost (#274: an AMD Radeon's DX12 driver gave up while the
    /// preview's textures went up, and the launcher then drew on the dead device, with
    /// thousands of errors a second): everything made on it goes, the other interface is
    /// taken - DirectX 12 and Vulkan for each other, remembered in the settings for the game
    /// as well - and the window is drawn again on a new device. Twice at most.
    fn recover_device(&mut self) -> bool {
        let Some(why) = self.renderer.as_ref().and_then(|r| r.device_lost()) else { return false };
        let tries = LAUNCHER_RECOVERIES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let name = self.renderer.as_ref().map(|r| r.adapter_name.clone()).unwrap_or_default();
        let other = if name.contains("(Dx12)") { Some("vulkan") } else if name.contains("(Vulkan)") && cfg!(windows) { Some("dx12") } else if name.contains("(Vulkan)") { Some("gl") } else { None };
        log::error!("launcher: the graphics device was lost on {name} ({why}); {}", match (tries < 2, other) {
            (true, Some(o)) => format!("drawing on {o} from now on"),
            (true, None) => "drawing on a new device".to_string(),
            _ => "giving up".to_string(),
        });
        if tries >= 2 {
            return false;
        }
        if let Some(o) = other {
            std::env::set_var("OMSI_BACKEND", o);
            self.state.settings["graphics_api"] = serde_json::json!(o);
            self.state.settings_dirty = 0.3;
        }
        self.surface = None;
        self.drop_gpu();
        if let Some(h) = self.gpu_rest_drop.take() {
            let _ = h.join();
        }
        self.renderer = None;
        self.make_surface();
        true
    }

    #[cfg(not(target_os = "android"))]
    fn update_discord(&mut self) {
        let enabled = self.state.settings.get("discord_status").and_then(|v| v.as_bool()).unwrap_or(true);
        let launching = self.state.queued_launch.is_some()
            || self.state.launch_hold.is_some_and(|at| at.elapsed().as_secs_f32() < 15.0);
        let game_running = self.state.instances.iter().any(|i| i.running);
        let presence = crate::discord::Presence::for_launcher(enabled, launching, game_running);
        if presence.is_none() {
            if let Some(discord) = self.discord.as_ref() {
                discord.stop();
                if discord.is_finished() {
                    drop(self.discord.take());
                }
            }
            return;
        }
        if let Some(discord) = self.discord.as_ref() {
            if discord.is_stopping() {
                if !discord.is_finished() {
                    return;
                }
                drop(self.discord.take());
            }
        }
        if self.discord.is_none() {
            if !self.state.instances_ready() {
                return;
            }
            if Instant::now() < self.discord_next_try {
                return;
            }
            self.discord_next_try = Instant::now() + std::time::Duration::from_secs(5);
            let app_id = self.state.settings.get("discord_app_id").and_then(|v| v.as_str()).unwrap_or("");
            self.discord = crate::discord::Discord::start(app_id);
        }
        if let Some(discord) = self.discord.as_ref() {
            discord.set(presence);
        }
    }

    /// The window got or lost the keyboard (`WindowEvent::Focused`, or `focus 0/1` of a
    /// launcher script).
    fn set_focus(&mut self, f: bool) {
        self.focused = f;
        if !f {
            self.pages.pads.cancel_feedback_test();
        }
        // (only once the game is on its way: the launcher has the focus while Start is
        // pressed, and gives the device up then as before)
        self.awake_in_game = f && self.renderer.is_none() && self.state.in_game() && self.state.queued_launch.is_none();
    }

    /// Looked at while a game runs (see `awake_in_game`): drawn and answering as usual.
    fn awake(&self) -> bool {
        // (the player asked the launcher not to rest while a game runs: it is always awake)
        !self.rests() || (self.awake_in_game && self.focused && self.state.queued_launch.is_none())
    }

    /// Whether the launcher gives the graphics device up while a game runs (#834: the setting
    /// "The launcher rests while a game runs"; on by default).
    fn rests(&self) -> bool {
        self.state.settings.get("launcher_rest").and_then(|v| v.as_bool()).unwrap_or(true)
    }

    fn frame(&mut self, event_loop: &ActiveEventLoop) {
        if self.recover_device() {
            return;
        }
        let desktop = !mobile::mobile() && self.window.is_some();
        let presence_released = {
            #[cfg(not(target_os = "android"))]
            {
                if self.state.queued_launch.is_some() {
                    if let Some(discord) = self.discord.as_ref() {
                        discord.stop();
                    }
                }
                self.discord.as_ref().is_none_or(|discord| discord.is_finished())
            }
            #[cfg(target_os = "android")]
            { true }
        };
        // (looked at while a game runs: drawn as usual, see `awake_in_game`)
        let awake = self.awake();
        if desktop && self.renderer.is_none() {
            if self.state.in_game() && !awake {
                // nothing is drawn while a game runs; what is clicked or typed meanwhile is not
                // done once the launcher is back (the first frame pressed Start again)
                let now = Instant::now();
                let dt = now.duration_since(self.last).as_secs_f32().min(0.1);
                self.last = now;
                self.run_script();
                self.state.update(dt);
                self.discard_input();
                #[cfg(not(target_os = "android"))]
                self.update_discord();
                // `update` may have seen the game end: open the device this frame instead of
                // returning with the resting picture until another redraw happens to notice.
                if self.state.in_game() && !self.awake() {
                    return;
                }
            }
            if self.state.in_game() {
                log::info!("launcher: its window is looked at while a game runs, the graphics device is opened again");
                // (what was clicked while it stood still is not done: the click that brought
                // it forward pressed whatever lay under it)
                self.discard_input();
            } else {
                self.awake_in_game = false;
                log::info!("launcher: no game runs any more, the graphics device is opened again");
            }
            self.make_surface();
        }
        self.draw_frame(event_loop);
        // a game starts or runs: the frame just drawn says so and stays in the window, and the
        // graphics device is given up until the game ends (with it open, a game on an NVIDIA
        // card without Resizable BAR uploaded at 20 MB/s)
        // (asked again: a script's `focus 0` comes in the frame just drawn)
        if desktop && self.renderer.is_some() && self.state.in_game() && !self.awake()
            && (self.state.queued_launch.is_none() || presence_released)
        {
            log::info!("launcher: a game starts or runs, the graphics device is given up until it ends");
            self.surface = None;
            self.drop_gpu();
            // Dropping a wgpu device can wait on the GPU for seconds (especially while the
            // game is opening the same card). Do it off the UI thread so Windows does not
            // mark the launcher "Not Responding" over the resting picture.
            if let Some(r) = self.renderer.take() {
                if let Some(prev) = self.gpu_rest_drop.take() {
                    let _ = prev.join();
                }
                self.gpu_rest_drop = std::thread::Builder::new()
                    .name("launcher-gpu-rest".into())
                    .spawn(move || drop(r))
                    .ok();
            }
        }
        if let Some(d) = presence_released.then(|| self.state.queued_launch.take()).flatten() {
            // Finish the Discord handoff in the background before starting the child.
            #[cfg(not(target_os = "android"))]
            drop(self.discord.take());
            // The Controls page may still own the same DirectInput wheel non-exclusively.
            // Drop it before the child asks for exclusive foreground access for force feedback.
            self.pages.pads.release_io();
            self.state.spawn_launch(d);
        }
    }

    /// The launcher's picture, put on the window.
    fn draw_frame(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        let dt = now.duration_since(self.last).as_secs_f32().min(0.1);
        self.last = now;
        let (Some(window), Some(_)) = (self.window.clone(), self.surface.as_ref()) else { return };
        let scale = self.ui_scale();
        let phys = window.inner_size();
        let (pw, ph) = (phys.width.max(1), phys.height.max(1));

        self.run_script();
        self.state.update(dt);
        #[cfg(not(target_os = "android"))]
        self.update_discord();
        self.update_tick(event_loop);
        // the preview shows the chosen bus in the chosen light
        let c = &self.state.choice;
        let look = showroom::Look { root: std::path::PathBuf::from(&self.state.config.root), map: c.map.clone(), bus: c.bus.clone(), paint: c.paint.clone(), weather: c.weather.clone(), time: c.time, date: c.date.clone() };
        // (not while a game runs: the launcher looked at meanwhile loads no bus onto the card;
        // nor before the first frame is shown, which the bus's loading would hold up)
        if !self.first_frame && !look.bus.is_empty() && !look.map.is_empty() && !self.state.in_game() {
            self.showroom.want(look);
        }
        if let Some(r) = self.renderer.as_ref() {
            self.showroom.update(r, dt);
        }
        self.gui_frame(event_loop, dt, (pw, ph), scale, &window);
    }

    fn check_exit(&mut self, event_loop: &ActiveEventLoop) {
        if self.exit_after.map(|e| self.started.elapsed().as_secs_f32() >= e).unwrap_or(false) {
            showroom::clear_placing_mark();
            event_loop.exit();
        }
    }

    fn run_script(&mut self) {
        let t = self.started.elapsed().as_secs_f32();
        while let Some((at, cmd)) = self.script.first().cloned() {
            if at > t {
                break;
            }
            self.script.remove(0);
            log::info!("launcher input t={at}: {cmd}");
            let (verb, arg) = cmd.split_once(' ').unwrap_or((cmd.as_str(), ""));
            let xy = || {
                let mut it = arg.split(',').map(|v| v.trim().parse::<f32>().unwrap_or(0.0));
                egui_retained::pos2(it.next().unwrap_or(0.0), it.next().unwrap_or(0.0))
            };
            use egui_retained::input::{Button as B, InputEvent as E};
            let button = |pos, pressed| E::PointerButton { pos, button: B::Primary, pressed };
            let Some(g) = self.gui.as_mut() else { return };
            match verb {
                "move" => g.input.push(E::PointerMoved(xy())),
                "click" => {
                    g.input.push(E::PointerMoved(xy()));
                    g.input.push(button(xy(), true));
                    g.input.push(button(xy(), false));
                }
                // `press <name>`: a click on the node of that name
                "press" => match g.ui.find(arg.trim()) {
                    Some(n) => {
                        let q = g.ui.rect(n).center();
                        g.input.push(E::PointerMoved(q));
                        g.input.push(button(q, true));
                        g.input.push(button(q, false));
                    }
                    None => log::warn!("launcher input: no node named {}", arg.trim()),
                },
                // a drag: down at a place, `move` with the button still held, `up` at the end
                "down" => {
                    g.input.push(E::PointerMoved(xy()));
                    g.input.push(button(xy(), true));
                }
                "up" => {
                    let q = g.ui.pointer().unwrap_or(egui_retained::pos2(-1e4, -1e4));
                    g.input.push(button(q, false));
                }
                "wheel" => g.input.push(E::Wheel(egui_retained::vec2(0.0, arg.trim().parse::<f32>().unwrap_or(0.0) * 40.0))),
                "type" => g.input.push(E::Text(arg.to_string())),
                "key" => {
                    use egui_retained::Key as K;
                    let k = match arg.trim() {
                        "Enter" => Some(K::Enter),
                        "Escape" => Some(K::Escape),
                        "Backspace" => Some(K::Backspace),
                        "Tab" => Some(K::Tab),
                        _ => None,
                    };
                    if let Some(k) = k {
                        g.input.push(E::Key { key: k, pressed: true, repeat: false });
                        g.input.push(E::Key { key: k, pressed: false, repeat: false });
                    }
                }
                "shot" => self.shot = Some((0.0, std::path::PathBuf::from(arg.trim()))),
                // `focus 0` / `focus 1`: the window loses or gets the keyboard
                "focus" => self.set_focus(arg.trim() != "0"),
                "page" => {
                    if let Some((pg, _, _)) = PAGES.iter().find(|(_, n, _)| n.eq_ignore_ascii_case(arg.trim())) {
                        self.go(*pg);
                    }
                }
                _ => log::warn!("launcher input: what is '{cmd}'?"),
            }
        }
    }

    /// What was clicked or typed while the launcher stood still (a game ran) is not done.
    fn discard_input(&mut self) {
        if let Some(g) = self.gui.as_mut() {
            let _ = g.input.take(egui_retained::Vec2::ZERO, 1.0, 0.0);
        }
    }

    pub fn go(&mut self, p: Page) {
        if self.page != p {
            self.page = p;
            self.phone.page = match p {
                Page::Drive => { self.phone.tab = phone::Tab::Play; None }
                Page::Multiplayer => { self.phone.tab = phone::Tab::Online; None }
                Page::Mods => { self.phone.tab = phone::Tab::Mods; None }
                other => { self.phone.tab = phone::Tab::More; Some(other) }
            };
            match p {
                Page::Profile => self.state.load_profile(),
                Page::Mods => self.state.load_mods(),
                Page::Sessions => self.state.poll_now(),
                _ => {}
            }
        }
    }

}


/// The showroom's renderer: a bus on a floor needs none of the game's costly passes - no
/// ambient occlusion, a small shadow map, 4x MSAA for the edges whatever the game uses -
/// and it draws Vanilla+ whatever the game does (`showroom::lighting_for`): no enhanced
/// pipelines, reflection probe or cloud noise either (`RenderOptions::preview_only`).
fn showroom_options(settings: &crate::settings::Settings) -> omsi_render::RenderOptions {
    omsi_render::RenderOptions { msaa: 4, ssao: false, shadow_size: 1024, render_scale: 1.0, preview_only: true, no_enhanced: true, ray_tracing: false, ..settings.render_options() }
}
