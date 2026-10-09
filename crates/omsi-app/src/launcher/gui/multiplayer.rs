//! The Multiplayer page: hosting a duty or joining a friend's by its code, and the list of
//! dedicated servers with their icon, message, map and players (see `launcher::multiplayer`
//! for what each way does).

use super::kit::{self, len, lp, tr};
use super::theme::{DANGER, OK};
use super::Msg as Top;
use crate::launcher::state::{JoinProto, ServerEntry};
use crate::launcher::{Launcher, Page};
use egui_retained::widgets::{Button, Checkbox, Icon, Image, TextInput};
use egui_retained::{NodeId, ScrollAxes, Ui, Vec2, Visual, taffy};
use serde_json::Value;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

#[derive(Clone, Debug)]
pub(in crate::launcher) enum Msg {
    Tab(usize),
    Host(bool),
    Code(String),
    Join,
    GoDrive,
    AddAddress(String),
    AddName(String),
    Add,
    JoinServer(String),
    Remove(usize),
    Refresh,
    Proto(usize),
}

fn m(x: Msg) -> Top {
    Top::Multiplayer(x)
}

pub(in crate::launcher) struct MultiplayerPage {
    pub root: NodeId,
    tabs: [NodeId; 2],
    code_tab: NodeId,
    servers_tab: NodeId,
    host_switch: NodeId,
    host_info: NodeId,
    host_key: u64,
    code_field: NodeId,
    shown_code: String,
    join_state: NodeId,
    join_icon: NodeId,
    join_text: NodeId,
    join_button: NodeId,
    add_address: NodeId,
    add_name: NodeId,
    list: NodeId,
    list_key: u64,
    protos: [NodeId; 3],
    icons: HashMap<String, egui_retained::epaint::TextureId>,
}

impl MultiplayerPage {
    pub fn build(ui: &mut Ui, host: NodeId) -> MultiplayerPage {
        let root = ui.column(host);
        ui.add_class(root, "content");
        kit::grow(ui, root);
        ui.set_scroll(root, ScrollAxes { x: false, y: true });
        kit::page_title(ui, root, "Multiplayer", "Drive with friends by a code, or on servers that are always on.");
        let bar = ui.row(root);
        kit::gap(ui, bar, 4.0);
        let mut tabs = [NodeId::dangling(); 2];
        for (k, (t, i)) in [("Connect by Code", "link"), ("Servers", "dns")].iter().enumerate() {
            let b = ui.add(bar, Button::new(tr(t)).icon(*i).class("tab"));
            ui.on_click(b, m(Msg::Tab(k)));
            tabs[k] = b;
        }
        // --- by code
        let code_tab = ui.row(root);
        ui.style(code_tab, |s| {
            s.gap = taffy::Size { width: lp(14.0), height: lp(14.0) };
            s.align_items = Some(taffy::AlignItems::Stretch);
            s.flex_wrap = taffy::FlexWrap::Wrap;
        });
        let half = |ui: &mut Ui, title: &str| {
            let c = kit::titled_card(ui, code_tab, title);
            ui.style(c, |s| {
                s.flex_grow = 1.0;
                s.flex_basis = len(0.0);
                s.min_size.width = len(380.0);
                s.min_size.height = len(300.0);
            });
            c
        };
        let hc = half(ui, "Host a game");
        kit::para(ui, hc, &tr("Your next duty opens a session. Give the code to your friends: it works at home and over the internet."), "dim");
        let host_switch = ui.add(hc, Checkbox::switch(false, tr("Host my next duty")).on_change(|v| m(Msg::Host(v))));
        let host_info = ui.column(hc);
        kit::gap(ui, host_info, 6.0);
        kit::grow(ui, host_info);
        let gd = ui.add(hc, Button::new(tr("Go to Drive")).icon("directions_bus"));
        ui.style(gd, |s| s.align_self = Some(taffy::AlignSelf::FlexStart));
        ui.on_click(gd, m(Msg::GoDrive));
        let jc = half(ui, "Connect by Code");
        kit::para(ui, jc, &tr("Paste the code your friend's game shows. The map, time and weather are the host's; you choose your bus."), "dim");
        let code_field = ui.add(jc, TextInput::new("").hint("OMSI-XXXX-XXXX-…").on_change(|s| m(Msg::Code(s))));
        let join_state = ui.row(jc);
        kit::gap(ui, join_state, 8.0);
        let join_icon = ui.add(join_state, Icon::new("check_circle"));
        let join_text = kit::para(ui, join_state, "", "dim");
        kit::grow(ui, join_text);
        let sp = ui.spacer(jc);
        let _ = sp;
        let join_button = ui.add(jc, Button::new(tr("Choose a bus and join")).icon("exit_to_app"));
        ui.style(join_button, |s| s.align_self = Some(taffy::AlignSelf::FlexStart));
        ui.on_click(join_button, m(Msg::Join));
        // --- servers
        let servers_tab = ui.column(root);
        kit::gap(ui, servers_tab, 12.0);
        let add = ui.row(servers_tab);
        kit::gap(ui, add, 10.0);
        let add_address = ui.add(add, TextInput::new("").hint(tr("Server address")).on_change(|s| m(Msg::AddAddress(s))));
        kit::grow(ui, add_address);
        let add_name = ui.add(add, TextInput::new("").hint(tr("Name (optional)")).on_change(|s| m(Msg::AddName(s))));
        ui.style(add_name, |s| s.size.width = len(220.0));
        let ab = ui.add(add, Button::new(tr("Add server")).icon("add").class("primary"));
        ui.on_click(ab, m(Msg::Add));
        let list = ui.column(servers_tab);
        kit::gap(ui, list, 8.0);
        let foot = ui.row(servers_tab);
        kit::gap(ui, foot, 10.0);
        let rb = ui.add(foot, Button::new(tr("Refresh")).icon("refresh"));
        ui.on_click(rb, m(Msg::Refresh));
        kit::text(ui, foot, &tr("Join via"), "dim");
        let mut protos = [NodeId::dangling(); 3];
        for (k, t) in ["Auto", "UDP", "WebSocket"].iter().enumerate() {
            let b = ui.add(foot, Button::new(tr(t)).class("tab"));
            ui.on_click(b, m(Msg::Proto(k)));
            protos[k] = b;
        }
        MultiplayerPage {
            root,
            tabs,
            code_tab,
            servers_tab,
            host_switch,
            host_info,
            host_key: 1,
            code_field,
            shown_code: String::new(),
            join_state,
            join_icon,
            join_text,
            join_button,
            add_address,
            add_name,
            list,
            list_key: 1,
            protos,
            icons: HashMap::new(),
        }
    }

    pub fn sync(&mut self, ui: &mut Ui, l: &Launcher) {
        let tab = l.mp.tab.min(1);
        for (k, n) in self.tabs.iter().enumerate() {
            ui.set_selected(*n, k == tab);
        }
        ui.set_visible(self.code_tab, tab == 0);
        ui.set_visible(self.servers_tab, tab == 1);
        if tab == 0 {
            self.sync_code(ui, l);
        } else {
            self.sync_servers(ui, l);
        }
    }

    fn sync_code(&mut self, ui: &mut Ui, l: &Launcher) {
        let hosting = l.state.choice.lan_mode == "host";
        ui.set_checked(self.host_switch, hosting);
        // the running session's code, when there is one
        let session = l.state.instances.iter().filter(|i| i.running).filter_map(|i| i.lan_status.clone()).find(|s| s.get("role").and_then(Value::as_str) == Some("host"));
        let code = session.as_ref().and_then(|s| s.get("code").and_then(Value::as_str)).map(str::to_string);
        let tunnel = session.as_ref().is_some_and(|s| s.get("tunnel").and_then(Value::as_str).is_some());
        let key = {
            let mut h = std::collections::hash_map::DefaultHasher::new();
            (hosting, &code, tunnel).hash(&mut h);
            h.finish()
        };
        if key != self.host_key {
            self.host_key = key;
            ui.clear(self.host_info);
            match &code {
                Some(code) => {
                    kit::text(ui, self.host_info, &tr("Session code"), "dim");
                    let r = ui.row(self.host_info);
                    kit::gap(ui, r, 8.0);
                    let c = kit::text(ui, r, code, "strong");
                    ui.visual(c, Visual::new().font(egui_retained::epaint::FontFamily::Monospace).font_size(16.0_f32));
                    kit::grow(ui, c);
                    let b = ui.add(r, Button::new(tr("Copy")).icon("content_copy").class("primary"));
                    ui.on_click(b, Top::Copy(code.clone(), tr("Session code copied")));
                    kit::text(ui, self.host_info, &tr(if tunnel { "Friends can join from anywhere" } else { "Getting ready for friends on the internet…" }), "faint");
                }
                None => {
                    kit::para(ui, self.host_info, &tr(if hosting { "Start a duty on the Drive page: the code appears here." } else { "Turn on hosting, then start a duty on the Drive page." }), "");
                }
            }
        }
        let joining = l.state.choice.lan_mode == "join" && l.state.joined_server.is_none();
        let shown = if joining { l.state.choice.lan_addr.clone() } else { String::new() };
        if shown != self.shown_code && ui.focused() != Some(self.code_field) {
            ui.with::<TextInput, _>(self.code_field, |t| t.set(&shown));
        }
        self.shown_code = shown;
        ui.set_visible(self.join_state, joining);
        if joining {
            let (ok, text) = &l.state.join;
            ui.with::<Icon, _>(self.join_icon, |i| i.name = (if *ok { "check_circle" } else { "error" }).into());
            ui.visual(self.join_icon, Visual::new().color(if *ok { OK } else { DANGER }));
            ui.set_text(self.join_text, text);
            ui.set_class(self.join_text, "dim", *ok);
            ui.set_class(self.join_text, "danger-text", !*ok);
        }
        let can = joining && l.state.join.0 && !l.state.choice.lan_addr.is_empty();
        ui.set_class(self.join_button, "primary", can);
    }

    fn sync_servers(&mut self, ui: &mut Ui, l: &Launcher) {
        for (k, n) in self.protos.iter().enumerate() {
            ui.set_selected(*n, k == l.mp.proto.min(2));
        }
        // the servers' icons, made textures once they came
        for e in &l.state.servers {
            if self.icons.contains_key(&e.address) {
                continue;
            }
            if let Some((_, Ok(i))) = l.state.server_info.get(&e.address) {
                if let Ok(img) = image::load_from_memory(&i.icon) {
                    let img = img.to_rgba8();
                    let id = ui.load_image(&format!("server {}", e.address), [img.width() as usize, img.height() as usize], img.as_raw());
                    self.icons.insert(e.address.clone(), id);
                }
            }
        }
        let key = {
            let mut h = std::collections::hash_map::DefaultHasher::new();
            for e in &l.state.servers {
                (&e.name, &e.address, self.icons.contains_key(&e.address)).hash(&mut h);
                match l.state.server_info.get(&e.address).map(|x| &x.1) {
                    Some(Ok(i)) => (0u8, &i.name, &i.motd, &i.map, &i.time, &i.weather, i.players, i.max_players).hash(&mut h),
                    Some(Err(e)) => (1u8, e).hash(&mut h),
                    None => 2u8.hash(&mut h),
                }
            }
            h.finish()
        };
        if key == self.list_key {
            return;
        }
        self.list_key = key;
        ui.clear(self.list);
        if l.state.servers.is_empty() {
            let c = kit::card(ui, self.list);
            ui.style(c, |s| {
                s.flex_direction = taffy::FlexDirection::Row;
                s.align_items = Some(taffy::AlignItems::Center);
                s.gap = taffy::Size { width: lp(18.0), height: lp(0.0) };
            });
            let i = ui.add(c, Icon::new("dns"));
            ui.visual(i, Visual::new().font_size(30.0_f32));
            kit::para(ui, c, &tr("No servers yet. Add a server by the address its owner gives you."), "dim");
        }
        for (k, e) in l.state.servers.iter().enumerate() {
            let info = l.state.server_info.get(&e.address).map(|x| x.1.clone());
            let c = kit::card(ui, self.list);
            ui.style(c, |s| {
                s.flex_direction = taffy::FlexDirection::Row;
                s.align_items = Some(taffy::AlignItems::Center);
                s.gap = taffy::Size { width: lp(14.0), height: lp(0.0) };
                s.padding = taffy::Rect::length(12.0_f32);
            });
            match self.icons.get(&e.address) {
                Some(tex) => {
                    let im = ui.add(c, Image::new(Some(*tex), Vec2::splat(64.0)));
                    ui.style(im, |s| s.size = taffy::Size { width: len(64.0), height: len(64.0) });
                    ui.visual(im, Visual::new().radius(8.0_f32));
                }
                None => {
                    let letter = e.name.chars().chain(info.as_ref().and_then(|i| i.as_ref().ok()).map(|i| i.name.clone()).unwrap_or_default().chars()).next().unwrap_or('S').to_uppercase().to_string();
                    let b = ui.column(c);
                    ui.style(b, |s| {
                        s.size = taffy::Size { width: len(64.0), height: len(64.0) };
                        s.align_items = Some(taffy::AlignItems::Center);
                        s.justify_content = Some(taffy::JustifyContent::Center);
                        s.flex_shrink = 0.0;
                    });
                    ui.add_class(b, "stage");
                    let t = kit::text(ui, b, &letter, "dim");
                    ui.visual(t, Visual::new().font_size(26.0_f32));
                }
            }
            let tc = ui.column(c);
            kit::grow(ui, tc);
            kit::gap(ui, tc, 3.0);
            let title = if !e.name.is_empty() { e.name.clone() } else { info.as_ref().and_then(|i| i.as_ref().ok()).map(|i| i.name.clone()).unwrap_or_else(|| e.address.clone()) };
            let t = kit::text(ui, tc, &title, "strong");
            ui.visual(t, Visual::new().font_size(15.0_f32));
            match &info {
                Some(Ok(i)) => {
                    kit::text(ui, tc, &i.motd, "");
                    let map = std::path::Path::new(&i.map.replace('\\', "/")).parent().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| i.map.clone());
                    kit::text(ui, tc, &format!("{map} · {} · {}", i.time, if i.weather.is_empty() { tr("the map's weather") } else { i.weather.clone() }), "faint");
                    let p = ui.row(c);
                    kit::gap(ui, p, 6.0);
                    kit::text(ui, p, &format!("{}/{}", i.players, i.max_players), "ok-text");
                    let ic = ui.add(p, Icon::new("signal_cellular_alt"));
                    ui.visual(ic, Visual::new().color(OK));
                }
                Some(Err(err)) => {
                    kit::text(ui, tc, &format!("{} {err}", tr("Can't reach the server:")), "danger-text");
                    kit::text(ui, tc, &e.address, "faint");
                }
                None => {
                    kit::text(ui, tc, &tr("Asking the server…"), "dim");
                }
            }
            let j = ui.add(c, Button::new(tr("Join")).icon("exit_to_app").class("primary"));
            ui.on_click(j, m(Msg::JoinServer(e.address.clone())));
            let d = ui.add(c, Button::new("").icon("delete").class("ghost"));
            ui.set_tooltip(d, Some(tr("Remove from the list")));
            ui.on_click(d, m(Msg::Remove(k)));
        }
    }
}

impl Launcher {
    /// What the Multiplayer page asks each frame: the servers' status now and then, whether
    /// the code typed is one.
    pub(super) fn gui_multiplayer_tick(&mut self) {
        if self.page != Page::Multiplayer {
            return;
        }
        if self.mp.tab == 1 {
            for e in self.state.servers.clone() {
                self.state.ask_server(&e.address, 15.0);
            }
        } else if self.state.choice.lan_mode == "join" && self.state.joined_server.is_none() {
            self.state.check_join();
        }
    }
}

pub(in crate::launcher) fn handle(l: &mut Launcher, msg: Msg) {
    match msg {
        Msg::Tab(k) => l.mp.tab = k,
        Msg::Host(on) => {
            l.state.choice.lan_mode = if on { "host".into() } else { "off".into() };
            l.state.joined_server = None;
            l.state.touched();
            if on {
                // (the way in for friends behind strict routers: fetched now if it is missing)
                std::thread::spawn(omsi_net::tunnel::ensure_cloudflared);
            }
        }
        Msg::Code(a) => {
            l.state.choice.lan_addr = a.trim().to_string();
            l.state.choice.lan_mode = if a.trim().is_empty() { "off".into() } else { "join".into() };
            l.state.joined_server = None;
            l.state.touched();
        }
        Msg::Join => {
            let can = l.state.choice.lan_mode == "join" && l.state.joined_server.is_none() && l.state.join.0 && !l.state.choice.lan_addr.is_empty();
            if can {
                l.go(Page::Drive);
            } else {
                l.state.set_status("Paste a session code first", true);
            }
        }
        Msg::GoDrive => l.go(Page::Drive),
        Msg::AddAddress(a) => l.mp.add_address = a.trim().to_string(),
        Msg::AddName(n) => l.mp.add_name = n,
        Msg::Add => {
            let addr = l.mp.add_address.trim().to_string();
            if addr.is_empty() {
                l.state.set_status("Type the server's address (an IP, a name or a link)", true);
            } else if l.state.servers.iter().any(|s| s.address.eq_ignore_ascii_case(&addr)) {
                l.state.set_status("That server is in the list already", true);
            } else {
                l.state.servers.push(ServerEntry { name: l.mp.add_name.trim().to_string(), address: addr.clone() });
                l.state.save_servers();
                l.state.ask_server(&addr, 0.0);
                l.mp.add_address.clear();
                l.mp.add_name.clear();
                if let Some(p) = l.gui.as_mut().and_then(|g| g.multiplayer.as_mut()) {
                    let (a, n) = (p.add_address, p.add_name);
                    if let Some(g) = l.gui.as_mut() {
                        g.ui.with::<TextInput, _>(a, |t| t.set(""));
                        g.ui.with::<TextInput, _>(n, |t| t.set(""));
                    }
                }
            }
        }
        Msg::JoinServer(a) => {
            l.state.ask_server(&a, 5.0);
            let proto = [JoinProto::Auto, JoinProto::Udp, JoinProto::WebSocket][l.mp.proto.min(2)];
            l.state.join_server(&a, proto);
            if l.state.joined_server.as_deref() == Some(a.as_str()) {
                l.go(Page::Drive);
            }
        }
        Msg::Remove(k) => {
            if k < l.state.servers.len() {
                let gone = l.state.servers.remove(k);
                l.state.save_servers();
                l.mp.selected = None;
                if let Some(g) = l.gui.as_mut() {
                    if let Some(id) = g.multiplayer.as_mut().and_then(|p| p.icons.remove(&gone.address)) {
                        g.ui.free_image(id);
                    }
                }
            }
        }
        Msg::Refresh => {
            for e in l.state.servers.clone() {
                l.state.ask_server(&e.address, 0.0);
            }
        }
        Msg::Proto(k) => l.mp.proto = k,
    }
}
