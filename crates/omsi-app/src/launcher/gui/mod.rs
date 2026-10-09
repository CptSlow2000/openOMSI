//! The launcher's interface on `egui_retained` (the org's egui fork): a tree of nodes built
//! once per page and kept in step with the launcher's state each frame, drawn with egui-wgpu
//! onto the window. Clicks and edits come back as [`Msg`]s, which `Launcher::gui_message`
//! carries out on the state: the data side is `state.rs`, the pages' own state and saving
//! (`pages.rs`, `drive.rs`, `timetable.rs`, ...) and `omsi-launcher-core`.
//!
//! The look is the Development Tools' (see `theme`): header, sidebar, cards, status bar; on a
//! phone or in a narrow window a tab bar instead of the sidebar (see `phone`).

mod browser;
mod dialogs;
mod phone;
mod drive;
mod kit;
mod timetable;
mod multiplayer;
mod controls;
mod settings;
mod tutorials;
mod setup;
mod mods;
mod sessions;
mod profile;
mod stage;
pub(super) mod theme;

use super::{Launcher, Page, PAGES};
use egui_retained::input::WinitInput;
use egui_retained::render::Renderer;
use egui_retained::widgets::{Button, Text};
use egui_retained::{Frame, Layer, NodeId, Ui, Vec2, taffy};

/// What the interface asks the launcher to do.
#[derive(Clone, Debug)]
pub(super) enum Msg {
    Go(Page),
    ToggleTheme,
    /// Text to put on the clipboard, and the status line that says so.
    Copy(String, String),
    Drive(drive::Msg),
    Timetable(timetable::Msg),
    Multiplayer(multiplayer::Msg),
    Controls(controls::Msg),
    Settings(settings::Msg),
    Tutorials(tutorials::Msg),
    Setup(setup::Msg),
    Mods(mods::Msg),
    Sessions(sessions::Msg),
    Profile(profile::Msg),
    Dialog(dialogs::Msg),
    Phone(phone::Msg),
    Browser(browser::Msg),
}

/// The interface: its tree, its renderer and the input since the last frame.
pub(super) struct Gui {
    pub ui: Ui,
    pub input: WinitInput,
    renderer: Option<Renderer>,
    pub dark: bool,
    shell: Option<Shell>,
    dialogs: Option<dialogs::Dialogs>,
    phone: Option<phone::Phone>,
    browser: Option<browser::BrowserView>,
    drive: Option<drive::DrivePage>,
    timetable: Option<timetable::TimetablePage>,
    multiplayer: Option<multiplayer::MultiplayerPage>,
    controls: Option<controls::ControlsPage>,
    settings: Option<settings::SettingsPage>,
    tutorials: Option<tutorials::TutorialsPage>,
    setup: Option<setup::SetupPage>,
    mods: Option<mods::ModsPage>,
    sessions: Option<sessions::SessionsPage>,
    profile: Option<profile::ProfilePage>,
    /// The bus preview's texture in the renderer, and the showroom picture it shows.
    preview: Option<(egui_retained::epaint::TextureId, u64)>,
    map: Option<(egui_retained::epaint::TextureId, u64)>,
    /// When the interface asked to be drawn again (an animation, a caret).
    due: Option<std::time::Instant>,
}

struct Shell {
    app: NodeId,
    side: NodeId,
    nav: Vec<(Page, NodeId)>,
    host: NodeId,
    pages: Vec<(Page, NodeId)>,
    status_dot: NodeId,
    status: NodeId,
    theme_button: NodeId,
    driver: NodeId,
    driver_level: NodeId,
    badges: Vec<(Page, NodeId)>,
}

impl Gui {
    pub fn new() -> Gui {
        let dark = load_theme_dark();
        let mut ui = Ui::with_fonts(theme::fonts());
        ui.set_theme(theme::theme(dark));
        ui.set_icon_source(theme::icon_mask);
        Gui { ui, input: WinitInput::new(), renderer: None, dark, shell: None, dialogs: None, phone: None, browser: None, drive: None, timetable: None, multiplayer: None, controls: None, settings: None, tutorials: None, setup: None, mods: None, sessions: None, profile: None, preview: None, map: None, due: None }
    }

    /// The graphics device went: everything made on it with it.
    pub fn drop_gpu(&mut self) {
        self.renderer = None;
        self.preview = None;
        self.map = None;
    }

    fn build_shell(&mut self) -> Shell {
        let ui = &mut self.ui;
        let root = ui.root(Layer::Base);
        let app = ui.column(root);
        ui.add_class(app, "page");
        ui.style(app, |s| {
            s.flex_grow = 1.0;
            s.size = taffy::Size { width: taffy::Dimension::percent(1.0), height: taffy::Dimension::percent(1.0) };
        });
        // the header: openOMSI Launcher, and the theme on the right
        let header = ui.row(app);
        ui.add_class(header, "header");
        let brand = ui.text(header, "openOMSI");
        ui.add_class(brand, "brand");
        ui.spacer(header);
        let theme_button = ui.add(header, Button::new(""));
        ui.on_click(theme_button, Msg::ToggleTheme);
        ui.add(app, egui_retained::widgets::Separator);
        // the sidebar and the page
        let body = ui.row(app);
        ui.style(body, |s| {
            s.flex_grow = 1.0;
            s.flex_basis = taffy::Dimension::length(0.0);
            s.align_items = Some(taffy::AlignItems::Stretch);
            s.min_size.height = taffy::Dimension::length(0.0);
            s.padding = taffy::Rect { left: kit::lp(10.0), right: kit::lp(10.0), top: kit::lp(7.0), bottom: kit::lp(7.0) };
        });
        let side = ui.column(body);
        ui.add_class(side, "sidebar");
        let mut nav = Vec::new();
        let mut badges = Vec::new();
        for (p, name, icon) in PAGES {
            let b = ui.add(side, Button::new(omsi_ui::tr(name).into_owned()).icon(icon).class("nav").align(egui_retained::widgets::TextAlign::Left));
            ui.style(b, |s| s.justify_content = Some(taffy::JustifyContent::FlexStart));
            ui.on_click(b, Msg::Go(p));
            ui.set_name(b, &format!("nav-{name}"));
            nav.push((p, b));
            if matches!(p, Page::Sessions | Page::Mods) {
                let badge = ui.add(b, Text::new(""));
                ui.add_class(badge, "badge");
                ui.style(badge, |s| s.margin.left = taffy::LengthPercentageAuto::auto());
                ui.set_visible(badge, false);
                badges.push((p, badge));
            }
        }
        ui.spacer(side);
        // the driver at the sidebar's foot
        let card = ui.row(side);
        ui.add_class(card, "list-row");
        kit::gap(ui, card, 10.0);
        ui.on_click(card, Msg::Go(Page::Profile));
        let avatar = ui.add(card, egui_retained::widgets::Icon::new("account_circle"));
        ui.visual(avatar, egui_retained::Visual::new().font_size(20.0_f32));
        let who = ui.column(card);
        ui.style(who, |s| s.margin.left = taffy::LengthPercentageAuto::length(4.0));
        let driver = ui.text(who, "");
        ui.add_class(driver, "strong");
        let driver_level = ui.text(who, "");
        ui.add_class(driver_level, "faint");
        let version = ui.text(side, format!("openOMSI {}", crate::startup::VERSION));
        ui.add_class(version, "faint");
        ui.style(version, |s| s.margin = taffy::Rect { left: kit::lpa(10.0), right: kit::lpa(0.0), top: kit::lpa(6.0), bottom: kit::lpa(2.0) });
        let host = ui.column(body);
        ui.style(host, |s| {
            s.flex_grow = 1.0;
            s.flex_basis = taffy::Dimension::length(0.0);
            s.min_size = taffy::Size { width: taffy::Dimension::length(0.0), height: taffy::Dimension::length(0.0) };
        });
        // the status bar
        ui.add(app, egui_retained::widgets::Separator);
        let bar = ui.row(app);
        ui.add_class(bar, "status");
        let status_dot = ui.add(bar, kit::Dot::new(theme::OK));
        let status = ui.text(bar, "");
        Shell { app, side, nav, host, pages: Vec::new(), status_dot, status, theme_button, driver, driver_level, badges }
    }

    /// The page's tree, built the first time it is shown.
    fn page_node(&mut self, page: Page) -> Option<NodeId> {
        let shell = self.shell.as_mut()?;
        if let Some((_, n)) = shell.pages.iter().find(|(p, _)| *p == page) {
            return Some(*n);
        }
        let host = shell.host;
        let node = match page {
            Page::Drive => {
                let d = drive::DrivePage::build(&mut self.ui, host);
                let n = d.root;
                self.drive = Some(d);
                n
            }
            Page::Profile => {
                let p = profile::ProfilePage::build(&mut self.ui, host);
                let n = p.root;
                self.profile = Some(p);
                n
            }
            Page::Sessions => {
                let p = sessions::SessionsPage::build(&mut self.ui, host);
                let n = p.root;
                self.sessions = Some(p);
                n
            }
            Page::Mods => {
                let p = mods::ModsPage::build(&mut self.ui, host);
                let n = p.root;
                self.mods = Some(p);
                n
            }
            Page::Setup => {
                let p = setup::SetupPage::build(&mut self.ui, host);
                let n = p.root;
                self.setup = Some(p);
                n
            }
            Page::Tutorials => {
                let p = tutorials::TutorialsPage::build(&mut self.ui, host);
                let n = p.root;
                self.tutorials = Some(p);
                n
            }
            Page::Settings => {
                let p = settings::SettingsPage::build(&mut self.ui, host);
                let n = p.root;
                self.settings = Some(p);
                n
            }
            Page::Controls => {
                let p = controls::ControlsPage::build(&mut self.ui, host);
                let n = p.root;
                self.controls = Some(p);
                n
            }
            Page::Multiplayer => {
                let p = multiplayer::MultiplayerPage::build(&mut self.ui, host);
                let n = p.root;
                self.multiplayer = Some(p);
                n
            }
            Page::Timetable => {
                let p = timetable::TimetablePage::build(&mut self.ui, host);
                let n = p.root;
                self.timetable = Some(p);
                n
            }
        };
        self.shell.as_mut()?.pages.push((page, node));
        Some(node)
    }

    /// Bring the tree in step with the launcher.
    pub fn sync(&mut self, l: &Launcher) {
        if self.shell.is_none() {
            let s = self.build_shell();
            self.shell = Some(s);
            self.dialogs = Some(dialogs::Dialogs::build(&mut self.ui));
            let (app, host) = self.shell.as_ref().map(|s| (s.app, s.host)).unwrap_or((NodeId::dangling(), NodeId::dangling()));
            self.phone = Some(phone::Phone::build(&mut self.ui, app, host));
            self.browser = Some(browser::BrowserView::build(&mut self.ui));
        }
        // a phone, or a window too narrow for the sidebar: the phone's layout
        let narrow = super::mobile::mobile() || self.ui.screen_size().x < phone::NARROW;
        let phone_shown = narrow.then(|| phone::shown(l));
        if let Some(d) = self.dialogs.as_mut() {
            d.sync(&mut self.ui, l);
        }
        if let Some(b) = self.browser.as_mut() {
            b.sync(&mut self.ui, l);
        }
        let shown = match phone_shown {
            Some(phone::Shown::Page(p, _)) => self.page_node(p),
            Some(_) => None,
            None => self.page_node(l.page),
        };
        if let Some(p) = self.phone.as_mut() {
            p.sync(&mut self.ui, l, narrow);
        }
        let Some(shell) = self.shell.as_ref() else { return };
        let ui = &mut self.ui;
        ui.set_visible(shell.side, !narrow);
        for (p, n) in &shell.nav {
            ui.set_selected(*n, *p == l.page);
        }
        for (_, n) in &shell.pages {
            ui.set_visible(*n, Some(*n) == shown);
        }
        ui.set_text(shell.theme_button, &omsi_ui::tr(if self.dark { "Light theme" } else { "Dark theme" }));
        // the badges: games running, installs under way
        let running = l.state.instances.iter().filter(|i| i.running).count();
        let jobs = l.state.jobs.iter().filter(|j| j.finished.is_none()).count();
        for (p, b) in &shell.badges {
            let n = if *p == Page::Sessions { running } else { jobs };
            ui.set_visible(*b, n > 0);
            ui.set_text(*b, &n.to_string());
        }
        let (level, name) = match &l.state.profile {
            Some(p) => (p.level, p.name.clone()),
            None => (1, l.state.config.profile.clone()),
        };
        ui.set_text(shell.driver, if name.is_empty() { "No driver" } else { &name });
        ui.set_text(shell.driver_level, &format!("{} {level}", omsi_ui::tr("Level")));
        let (text, err, at) = l.state.status.clone();
        let busy = l.state.jobs.iter().any(|j| j.finished.is_none()) || l.state.loading_content || l.state.queued_launch.is_some();
        let fresh = err || at.elapsed().as_secs_f32() < 7.5;
        let shown_text = if fresh && !text.is_empty() { text.lines().next().unwrap_or("").to_string() } else { omsi_ui::tr("Ready").into_owned() };
        ui.set_text(shell.status, &shown_text);
        let color = if err { theme::DANGER } else if busy { theme::ACCENT } else { theme::OK };
        ui.update::<kit::Dot>(shell.status_dot, |d| std::mem::replace(&mut d.color, color) != color);
        ui.set_tooltip(shell.status, (fresh && text.lines().count() > 1).then(|| text.clone()));
        match l.page {
            Page::Drive => {
                if let Some(d) = self.drive.as_mut() {
                    d.sync(&mut self.ui, l);
                }
            }
            Page::Profile => {
                if let Some(p) = self.profile.as_mut() {
                    p.sync(&mut self.ui, l);
                }
            }
            Page::Sessions => {
                if let Some(p) = self.sessions.as_mut() {
                    p.sync(&mut self.ui, l);
                }
            }
            Page::Mods => {
                if let Some(p) = self.mods.as_mut() {
                    p.sync(&mut self.ui, l);
                }
            }
            Page::Setup => {
                if let Some(p) = self.setup.as_mut() {
                    p.sync(&mut self.ui, l);
                }
            }
            Page::Tutorials => {
                if let Some(p) = self.tutorials.as_mut() {
                    p.sync(&mut self.ui, l);
                }
            }
            Page::Settings => {
                if let Some(p) = self.settings.as_mut() {
                    p.sync(&mut self.ui, l);
                }
            }
            Page::Controls => {
                if let Some(p) = self.controls.as_mut() {
                    p.sync(&mut self.ui, l);
                }
            }
            Page::Multiplayer => {
                if let Some(p) = self.multiplayer.as_mut() {
                    p.sync(&mut self.ui, l);
                }
            }
            Page::Timetable => {
                if let Some(p) = self.timetable.as_mut() {
                    p.sync(&mut self.ui, l);
                }
            }
        }
    }

    pub fn toggle_theme(&mut self) {
        self.dark = !self.dark;
        save_theme_dark(self.dark);
        self.ui.set_theme(theme::theme(self.dark));
    }

    /// The page background's colour (the frame is cleared to it).
    pub fn clear_color(&self) -> wgpu::Color {
        if self.dark { wgpu::Color { r: 0.0065, g: 0.0075, b: 0.0097, a: 1.0 } } else { wgpu::Color { r: 0.9, g: 0.91, b: 0.93, a: 1.0 } }
    }

    /// The bus preview and the map picture, where a page shows them: their textures made or
    /// renewed for the renderer.
    pub fn pictures(&mut self, l: &mut Launcher) {
        let (Some(renderer), Some(gr)) = (l.renderer.as_mut(), self.renderer.as_mut()) else { return };
        let ppp = self.ui.pixels_per_point();
        let stages: Vec<NodeId> = self.drive.as_ref().and_then(|d| d.preview_node()).filter(|n| self.ui.shown(*n) && l.page == Page::Drive).into_iter().chain(self.phone.as_ref().map(|p| p.stage_node()).filter(|n| self.ui.shown(*n))).collect();
        for node in stages {
            let r = self.ui.rect(node);
            let (w, h) = ((r.width() * ppp) as u32, (r.height() * ppp) as u32);
            if w > 8 && h > 8 {
                if let Some(view) = l.showroom.preview(renderer, w, h) {
                    let gen = l.showroom.generation;
                    match self.preview {
                        Some((id, g)) if g == gen => {
                            let _ = id;
                        }
                        Some((id, _)) => {
                            gr.update(&renderer.device, id, &view, wgpu::FilterMode::Linear);
                            self.preview = Some((id, gen));
                            self.ui.request_repaint();
                        }
                        None => {
                            let id = gr.register(&renderer.device, &view, wgpu::FilterMode::Linear);
                            self.preview = Some((id, gen));
                        }
                    }
                }
            }
            let tex = self.preview.filter(|_| l.showroom.has_picture()).map(|p| p.0);
            set_stage(&mut self.ui, node, tex, Vec2::new(w as f32, h as f32));
        }
        if let Some(node) = self.drive.as_ref().and_then(|d| d.map_node()).filter(|n| self.ui.shown(*n) && l.page == Page::Drive) {
            if let Some(view) = l.mapview.picture(renderer) {
                let gen = l.mapview.generation;
                match self.map {
                    Some((_, g)) if g == gen => {}
                    Some((id, _)) => {
                        gr.update(&renderer.device, id, &view, wgpu::FilterMode::Linear);
                        self.map = Some((id, gen));
                        self.ui.request_repaint();
                    }
                    None => {
                        let id = gr.register(&renderer.device, &view, wgpu::FilterMode::Linear);
                        self.map = Some((id, gen));
                    }
                }
            }
            let (pw, ph) = l.mapview.pixels();
            let tex = self.map.filter(|_| l.mapview.status().is_empty()).map(|p| p.0);
            set_stage(&mut self.ui, node, tex, Vec2::new(pw as f32, ph as f32));
        }
    }

    /// Draw `frame` onto the window's surface texture.
    pub fn render(&mut self, l: &mut Launcher, frame: &Frame, target: &wgpu::TextureView, size: [u32; 2]) {
        let Some(renderer) = l.renderer.as_ref() else { return };
        let clear = self.clear_color();
        let gr = self.renderer.get_or_insert_with(|| Renderer::new(&renderer.device, renderer.format()));
        let mut enc = renderer.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("launcher") });
        gr.draw(&renderer.device, &renderer.queue, &mut enc, target, size, frame, Some(clear));
        renderer.queue.submit([enc.finish()]);
    }

    /// The frame's picture into a file (`OMSI_LAUNCHER_SHOT`): drawn into a texture of its own.
    pub fn picture(&mut self, l: &mut Launcher, frame: &Frame, size: [u32; 2]) -> Option<image::RgbaImage> {
        let renderer = l.renderer.as_ref()?;
        let clear = self.clear_color();
        let gr = self.renderer.get_or_insert_with(|| Renderer::new(&renderer.device, renderer.format()));
        let px = gr.picture(&renderer.device, &renderer.queue, size, frame, clear);
        image::RgbaImage::from_raw(size[0], size[1], px)
    }

    /// Keep the frame's texture changes (a frame not drawn: the fonts still follow).
    pub fn textures_only(&mut self, l: &Launcher, frame: &Frame) {
        let (Some(renderer), Some(gr)) = (l.renderer.as_ref(), self.renderer.as_mut()) else { return };
        gr.draw_textures_only(&renderer.device, &renderer.queue, frame);
    }

    pub(super) fn drive_mut(&mut self) -> Option<&mut drive::DrivePage> {
        self.drive.as_mut()
    }
}

/// A stage's texture, changed only when it is another (painting again is asked for by
/// whoever renewed the picture behind it).
fn set_stage(ui: &mut Ui, node: NodeId, tex: Option<egui_retained::epaint::TextureId>, size: Vec2) {
    let now = ui.get::<stage::Stage>(node).map(|s| (s.image.texture, s.image.size));
    if now != Some((tex, size)) {
        ui.with::<stage::Stage, _>(node, |s| s.set_texture(tex, size));
    }
}

/// The theme the player chose last (`~/.openomsi/launcher-theme.txt`: `dark` or `light`).
fn theme_file() -> std::path::PathBuf {
    omsi_launcher_lib::data_dir().join("launcher-theme.txt")
}

fn load_theme_dark() -> bool {
    std::fs::read_to_string(theme_file()).map(|t| t.trim() != "light").unwrap_or(true)
}

fn save_theme_dark(dark: bool) {
    if let Err(e) = std::fs::write(theme_file(), if dark { "dark\n" } else { "light\n" }) {
        log::warn!("launcher theme not saved: {e}");
    }
}

impl Launcher {
    /// A frame of the new interface: the tree brought in step with the state, the input
    /// answered, the messages carried out, the pictures renewed, and the frame drawn.
    pub(super) fn gui_frame(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, dt: f32, (pw, ph): (u32, u32), scale: f32, window: &std::sync::Arc<winit::window::Window>) {
        self.gui_controls_tick();
        self.gui_multiplayer_tick();
        self.gui_timetable_tick();
        self.gui_mods_tick();
        let Some(mut g) = self.gui.take() else { return };
        // (the renderer from the first frame: a frame not shown - a hidden window - still
        // hands its textures over, the fonts' atlas among them)
        if let (None, Some(r)) = (g.renderer.as_ref(), self.renderer.as_ref()) {
            g.renderer = Some(Renderer::new(&r.device, r.format()));
        }
        // (what the tree reads from the state but the state only works out mutably)
        let _ = self.state.has_last_situation();
        g.sync(self);
        let input = g.input.take(Vec2::new(pw as f32 / scale, ph as f32 / scale), scale, dt);
        // (nothing changed and no input: the picture on the screen is the frame - not drawn
        // again, the launcher idles without the GPU)
        let dirty = g.ui.needs_repaint() || !input.events.is_empty() || g.due.is_some_and(|t| std::time::Instant::now() >= t) || self.shot.is_some() || self.first_frame;
        let screen = input.screen;
        let frame = g.ui.run(input);
        if omsi_cfg::flags::OMSI_GUI_DEBUG.is_set() {
            log::info!("gui frame: {} primitives, screen {:?} pt, ppp {:.2}, {} texture sets", frame.primitives.len(), screen, frame.pixels_per_point, frame.textures.set.len());
        }
        window.set_cursor(frame.platform.cursor.to_winit());
        if let Some(t) = frame.platform.copied.clone() {
            if let Some(c) = self.clipboard.as_mut() {
                let _ = c.set_text(t);
            }
        }
        if let Some(url) = frame.platform.open_url.as_deref() {
            crate::updater::open_url(url);
        }
        if super::mobile::mobile() {
            let want = frame.platform.text_focus.is_some();
            if want != self.ime {
                self.ime = want;
                window.set_ime_allowed(want);
            }
        }
        let msgs = g.ui.drain::<Msg>();
        let (mut bus, map) = g.drive.as_ref().map(|d| d.pointers()).unwrap_or_default();
        if let Some(p) = g.phone.as_ref() {
            let t = p.stage_ptr.borrow_mut().take();
            bus.drag += t.drag;
            bus.wheel += t.wheel;
        }
        self.gui = Some(g);
        self.gui_stage(bus, map, scale);
        let changed = !msgs.is_empty();
        for m in msgs {
            self.gui_message(m);
        }
        let Some(mut g) = self.gui.take() else { return };
        g.pictures(self);
        // OMSI_LAUNCHER_SHOT: the window's picture into a file (a hidden window gives one too)
        let t = self.started.elapsed().as_secs_f32();
        if let Some((at, file)) = self.shot.clone() {
            if t >= at {
                self.shot = None;
                if let Some(img) = g.picture(self, &frame, [pw, ph]) {
                    let _ = img.save(&file);
                    log::info!("launcher: picture written to {}", file.display());
                }
            }
        }
        let (Some(renderer), Some(surface)) = (self.renderer.as_ref(), self.surface.as_mut()) else {
            self.gui = Some(g);
            return;
        };
        g.due = frame.repaint_after.map(|s| std::time::Instant::now() + std::time::Duration::from_secs_f32(s));
        if !dirty {
            g.textures_only(self, &frame);
            self.first_frame = false;
            self.gui = Some(g);
            self.check_exit(event_loop);
            return;
        }
        let current = match surface.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(f) | wgpu::CurrentSurfaceTexture::Suboptimal(f) => Some(f),
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                surface.resize(renderer, pw, ph);
                None
            }
            _ => None,
        };
        match current {
            Some(f) => {
                let view = f.texture.create_view(&Default::default());
                g.render(self, &frame, &view, [pw, ph]);
                window.pre_present_notify();
                f.present();
                if std::mem::take(&mut self.first_frame) {
                    log::info!("launcher: first frame presented in {:.2} s", self.started.elapsed().as_secs_f64());
                }
            }
            None => g.textures_only(self, &frame),
        }
        if omsi_cfg::flags::OMSI_GUI_DEBUG.is_set() && (changed || g.ui.needs_repaint() || frame.repaint_after == Some(0.0)) {
            log::info!("gui redraw: changed {changed}, {}, after {:?}", g.ui.repaint_reasons(), frame.repaint_after);
        }
        if changed || g.ui.needs_repaint() || frame.repaint_after == Some(0.0) {
            window.request_redraw();
        }
        // (the first frame made, shown or not - a hidden window shows none: what waited for
        // it, the bus preview's loading, goes ahead)
        if std::mem::take(&mut self.first_frame) {
            log::info!("launcher: first frame made in {:.2} s", self.started.elapsed().as_secs_f64());
        }
        self.gui = Some(g);
        self.check_exit(event_loop);
    }

    /// A message of the interface, carried out.
    pub(super) fn gui_message(&mut self, m: Msg) {
        match m {
            Msg::Go(p) => self.go(p),
            Msg::Copy(text, note) => {
                if let Some(c) = self.clipboard.as_mut() {
                    let _ = c.set_text(text);
                }
                self.state.set_status(note, false);
            }
            Msg::ToggleTheme => {
                if let Some(g) = self.gui.as_mut() {
                    g.toggle_theme();
                }
            }
            Msg::Drive(d) => drive::handle(self, d),
            Msg::Timetable(m) => timetable::handle(self, m),
            Msg::Multiplayer(m) => multiplayer::handle(self, m),
            Msg::Controls(m) => controls::handle(self, m),
            Msg::Settings(m) => settings::handle(self, m),
            Msg::Tutorials(m) => tutorials::handle(self, m),
            Msg::Setup(m) => setup::handle(self, m),
            Msg::Mods(m) => mods::handle(self, m),
            Msg::Sessions(m) => sessions::handle(self, m),
            Msg::Profile(p) => profile::handle(self, p),
            Msg::Dialog(m) => dialogs::handle(self, m),
            Msg::Phone(m) => phone::handle(self, m),
            Msg::Browser(m) => browser::handle(self, m),
        }
    }
}
