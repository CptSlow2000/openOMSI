//! The day and the weather: the clock and the date, the season, the traffic, what the game
//! starts with, and the weather - a preset, the natural weather, METAR, a cycle, or one's own.

use super::super::kit::{self, len, lp, tr};
use super::super::theme::ACCENT;
use super::{Msg, m};
use crate::launcher::drive as logic;
use crate::launcher::Launcher;
use crate::weather_setup::{self as ws, CustomWeather};
use egui_retained::widgets::{Button, Checkbox, Icon, Select, Slider, TextInput};
use egui_retained::{NodeId, ScrollAxes, Ui, Visual, taffy};

#[derive(Clone, Debug)]
pub(in crate::launcher) enum TimeMsg {
    Time(String),
    TimeStep(i32),
    Date(String),
    DateStep(i32),
    NowTime,
    NowDate,
    Season(usize),
    Phase(usize),
    Traffic(f32),
    Passengers(bool),
    Schedule(bool),
    Autostart(bool),
    OnFoot(bool),
    Weather(String),
    EditAsCustom,
    Presets,
    Custom(Field, f32),
    CustomFlag(Field, bool),
    Airport(String),
    AirportPick(usize),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::launcher) enum Field {
    Visibility,
    Brightness,
    WindDir,
    WindSpeed,
    Temp,
    Humidity,
    Cloud,
    Precip,
    Intensity,
    Wet,
    Snow,
    SnowRoad,
}

fn t(x: TimeMsg) -> super::super::Msg {
    m(Msg::Time(x))
}

const SEASONS: [&str; 5] = ["auto", "spring", "summer", "autumn", "winter"];
const PHASES: [&str; 3] = ["early", "mid", "late"];

pub(in crate::launcher::gui) struct TimeStep {
    server: NodeId,
    server_title: NodeId,
    server_time: NodeId,
    server_weather: NodeId,
    own: NodeId,
    time: NodeId,
    date: NodeId,
    now_time: NodeId,
    now_date: NodeId,
    seasons: Vec<NodeId>,
    phase_row: NodeId,
    phases: Vec<NodeId>,
    traffic: NodeId,
    traffic_value: NodeId,
    pax: NodeId,
    sched: NodeId,
    autostart: NodeId,
    on_foot: NodeId,
    edit_custom: NodeId,
    airport_row: NodeId,
    airport: NodeId,
    airports: NodeId,
    airport_list: Vec<(String, String)>,
    cards: NodeId,
    cards_key: String,
    editor: NodeId,
    editor_built: bool,
    shown: (String, String),
}

fn segmented(ui: &mut Ui, parent: NodeId, labels: &[&str], msg: fn(usize) -> TimeMsg) -> (NodeId, Vec<NodeId>) {
    let row = ui.row(parent);
    kit::gap(ui, row, 4.0);
    let items = labels
        .iter()
        .enumerate()
        .map(|(k, l)| {
            let b = ui.add(row, Button::new(tr(l)).class("tab"));
            kit::grow(ui, b);
            ui.on_click(b, t(msg(k)));
            b
        })
        .collect();
    (row, items)
}

impl TimeStep {
    pub fn build(ui: &mut Ui, p: NodeId) -> TimeStep {
        // on a server: its world, nothing to choose
        let server = ui.column(p);
        kit::gap(ui, server, 8.0);
        let head = ui.row(server);
        kit::gap(ui, head, 8.0);
        ui.add(head, Icon::new("lock"));
        let server_title = kit::text(ui, head, "", "heading");
        let r = kit::labelled(ui, server, "Time", 110.0);
        let server_time = kit::text(ui, r, "", "strong");
        let r = kit::labelled(ui, server, "Weather", 110.0);
        let server_weather = kit::text(ui, r, "", "strong");
        kit::para(ui, server, &tr("On a server the map, the time, the date and the weather are the same for everybody: the server keeps the world's clock. You choose your bus and your duty."), "dim");
        // one's own world
        let own = ui.column(p);
        kit::grow(ui, own);
        kit::gap(ui, own, 10.0);
        ui.set_scroll(own, ScrollAxes { x: false, y: true });
        let cols = ui.row(own);
        kit::gap(ui, cols, 12.0);
        ui.style(cols, |s| s.align_items = Some(taffy::AlignItems::FlexStart));
        let field = |ui: &mut Ui, parent: NodeId, label: &str, step: fn(i32) -> TimeMsg, set: fn(String) -> TimeMsg, hint: &str| -> (NodeId, NodeId) {
            let c = ui.column(parent);
            kit::grow(ui, c);
            kit::gap(ui, c, 6.0);
            kit::text(ui, c, &tr(label), "dim");
            let r = ui.row(c);
            kit::gap(ui, r, 4.0);
            let minus = ui.add(r, Button::new("").icon("remove"));
            ui.on_click(minus, t(step(-1)));
            let f = ui.add(r, TextInput::new("").hint(hint).on_submit(move |s| t(set(s))));
            kit::grow(ui, f);
            ui.style(f, |s| s.min_size.width = len(96.0));
            ui.visual(f, Visual::new().font_size(16.0_f32));
            let plus = ui.add(r, Button::new("").icon("add"));
            ui.on_click(plus, t(step(1)));
            let now = ui.add(c, Button::new(""));
            (f, now)
        };
        let (time, now_time) = field(ui, cols, "Time", TimeMsg::TimeStep, TimeMsg::Time, "HH:MM");
        let (date, now_date) = field(ui, cols, "Date", TimeMsg::DateStep, TimeMsg::Date, "YYYY-MM-DD");
        ui.set_text(now_time, &tr("Current time"));
        ui.on_click(now_time, t(TimeMsg::NowTime));
        ui.set_text(now_date, &tr("Current date"));
        ui.on_click(now_date, t(TimeMsg::NowDate));
        kit::text(ui, own, &tr("Season"), "dim");
        let (_, seasons) = segmented(ui, own, &["By date", "Spring", "Summer", "Autumn", "Winter"], TimeMsg::Season);
        let (phase_row, phases) = segmented(ui, own, &["Early", "Mid", "Late"], TimeMsg::Phase);
        let tr_row = ui.row(own);
        kit::gap(ui, tr_row, 10.0);
        let tl = kit::text(ui, tr_row, &tr("Cars around"), "dim");
        ui.style(tl, |s| s.size.width = len(110.0));
        let traffic = ui.add(tr_row, Slider::new(30.0, 0.0, 120.0).step(1.0).on_change(|v| t(TimeMsg::Traffic(v))));
        let traffic_value = kit::text(ui, tr_row, "", "strong");
        ui.style(traffic_value, |s| s.min_size.width = len(30.0));
        let grid = ui.row(own);
        ui.style(grid, |s| {
            s.flex_wrap = taffy::FlexWrap::Wrap;
            s.gap = taffy::Size { width: lp(12.0), height: lp(8.0) };
        });
        let sw = |ui: &mut Ui, label: &str, f: fn(bool) -> TimeMsg| {
            let c = ui.add(grid, Checkbox::switch(false, tr(label)).on_change(move |v| t(f(v))));
            ui.style(c, |s| s.min_size.width = taffy::Dimension::percent(0.46));
            c
        };
        let pax = sw(ui, "Passengers", TimeMsg::Passengers);
        let sched = sw(ui, "Timetable buses", TimeMsg::Schedule);
        let autostart = ui.add(own, Checkbox::switch(false, tr("Put the bus into service on start (Shift+U)")).on_change(|v| t(TimeMsg::Autostart(v))));
        let on_foot = ui.add(own, Checkbox::switch(false, tr("Start on foot (place a bus from the game menu)")).on_change(|v| t(TimeMsg::OnFoot(v))));
        ui.add(own, egui_retained::widgets::Separator);
        let wh = ui.row(own);
        kit::gap(ui, wh, 8.0);
        ui.add(wh, Icon::new("partly_cloudy_day"));
        kit::text(ui, wh, &tr("Weather"), "heading");
        let edit_custom = ui.add(own, Button::new(tr("Edit selected weather as custom")).icon("tune"));
        ui.on_click(edit_custom, t(TimeMsg::EditAsCustom));
        let airport_row = kit::labelled(ui, own, "Airport", 80.0);
        let airport = ui.add(airport_row, TextInput::new("").hint("ICAO").on_change(|s| t(TimeMsg::Airport(s))));
        ui.style(airport, |s| s.size.width = len(90.0));
        let airports = ui.add(airport_row, Select::new(Vec::new(), None).on_change(|i| t(TimeMsg::AirportPick(i))));
        kit::grow(ui, airports);
        let cards = ui.row(own);
        ui.style(cards, |s| {
            s.flex_wrap = taffy::FlexWrap::Wrap;
            s.gap = taffy::Size { width: lp(8.0), height: lp(8.0) };
            s.flex_shrink = 0.0;
        });
        let editor = ui.column(own);
        kit::gap(ui, editor, 8.0);
        TimeStep {
            server,
            server_title,
            server_time,
            server_weather,
            own,
            time,
            date,
            now_time,
            now_date,
            seasons,
            phase_row,
            phases,
            traffic,
            traffic_value,
            pax,
            sched,
            autostart,
            on_foot,
            edit_custom,
            airport_row,
            airport,
            airports,
            airport_list: Vec::new(),
            cards,
            cards_key: String::new(),
            editor,
            editor_built: false,
            shown: (String::new(), String::new()),
        }
    }

    pub fn sync(&mut self, ui: &mut Ui, l: &Launcher) {
        let joined = logic::joined_server_name(l);
        ui.set_visible(self.server, joined.is_some());
        ui.set_visible(self.own, joined.is_none());
        if let Some(name) = joined {
            let info = l.state.joined_server.as_ref().and_then(|a| l.state.server_info.get(a)).and_then(|x| x.1.as_ref().ok()).cloned();
            ui.set_text(self.server_title, &format!("{} {name}", tr("Set by")));
            ui.set_text(self.server_time, &info.as_ref().map(|i| i.time.clone()).unwrap_or_default());
            ui.set_text(self.server_weather, &info.as_ref().map(|i| if i.weather.is_empty() { tr("the map's") } else { i.weather.clone() }).unwrap_or_default());
            return;
        }
        let c = &l.state.choice;
        let time = format!("{:02}:{:02}", c.time / 60, c.time % 60);
        if self.shown.0 != time && ui.focused() != Some(self.time) {
            self.shown.0 = time.clone();
            ui.with::<TextInput, _>(self.time, |f| f.set(&time));
        }
        if self.shown.1 != c.date && ui.focused() != Some(self.date) {
            self.shown.1 = c.date.clone();
            let d = c.date.clone();
            ui.with::<TextInput, _>(self.date, |f| f.set(&d));
        }
        let follows = |k: &str| l.state.settings.get(k).and_then(|v| v.as_bool()).unwrap_or(false);
        ui.set_visible(self.now_time, !follows("use_real_time"));
        ui.set_visible(self.now_date, !follows("use_real_date"));
        let s = SEASONS.iter().position(|x| *x == c.season).unwrap_or(0);
        for (k, n) in self.seasons.iter().enumerate() {
            ui.set_selected(*n, k == s);
        }
        ui.set_visible(self.phase_row, s > 0);
        let ph = PHASES.iter().position(|x| *x == c.phase).unwrap_or(1);
        for (k, n) in self.phases.iter().enumerate() {
            ui.set_selected(*n, k == ph);
        }
        ui.set_slider(self.traffic, c.traffic);
        ui.set_text(self.traffic_value, &format!("{:.0}", c.traffic));
        ui.set_checked(self.pax, c.passengers);
        ui.set_checked(self.sched, c.schedule);
        ui.set_checked(self.autostart, c.autostart);
        ui.set_checked(self.on_foot, c.on_foot);
        ui.set_visible(self.edit_custom, logic::selected_weather_as_custom(&l.state.config.root, &c.weather).is_some());
        // METAR: the airport
        let metar = c.weather.strip_prefix("metar:").map(str::to_string);
        ui.set_visible(self.airport_row, metar.is_some());
        if let Some(code) = &metar {
            if self.airport_list.is_empty() {
                self.airport_list = ws::metar_airports(std::path::Path::new(&l.state.config.root));
            }
            let code = code.to_uppercase();
            if ui.focused() != Some(self.airport) {
                ui.update::<TextInput>(self.airport, |f| f.value != code && { f.set(&code); true });
            }
            let sel = self.airport_list.iter().position(|a| a.0.eq_ignore_ascii_case(&code));
            ui.set_select(self.airports, Some(self.airport_list.iter().map(|a| a.1.clone()).collect()), sel);
        }
        // the custom weather's editor, or the cards
        let custom = ws::custom_weather(Some(&c.weather));
        ui.set_visible(self.cards, custom.is_none());
        ui.set_visible(self.editor, custom.is_some());
        if let Some(w) = custom {
            self.sync_editor(ui, &w);
            return;
        }
        let home = logic::nearest_airport(&l.state.config.root, &c.map);
        let mut items: Vec<(String, String, String, &str, bool)> = vec![
            (String::new(), tr("Natural weather"), tr("Develops by itself through the day and the season"), "wb_sunny", false),
            (CustomWeather::default().encode(), tr("Custom weather"), tr("Set visibility, wind, clouds, rain, temperature and road state"), "tune", false),
        ];
        let code = metar.clone().unwrap_or_else(|| home.clone());
        items.push((format!("metar:{code}"), tr("Current weather"), format!("METAR of {code} (fetched at the start)"), "public", false));
        items.push(("cycle".into(), tr("Weather cycle"), tr("Changes every 25-60 minutes, as the month allows"), "autorenew", false));
        for w in &l.state.weathers {
            if !l.state.weather_fits(w) {
                continue;
            }
            let vis = if w.fog_m >= 20000.0 { "clear air".to_string() } else { format!("{:.0} m", w.fog_m) };
            items.push((w.file.clone(), w.name.clone(), format!("{:.0} °C · {} · {vis}", w.temp, w.precip), logic::weather_icon_of(w), l.state.fresh.contains_key(&w.file)));
        }
        let chosen = c.weather.clone();
        let key = format!("{chosen}|{}", items.iter().map(|i| i.0.as_str()).collect::<Vec<_>>().join(";"));
        if key == self.cards_key {
            return;
        }
        self.cards_key = key;
        ui.clear(self.cards);
        for (file, name, meta, icon, fresh) in items {
            let on = file == chosen || (file.starts_with("metar:") && chosen.starts_with("metar:"));
            let card = ui.row(self.cards);
            ui.add_class(card, "list-row");
            ui.style(card, |s| {
                s.size.width = taffy::Dimension::percent(0.485);
                s.gap = taffy::Size { width: lp(10.0), height: lp(0.0) };
                s.min_size.height = len(58.0);
            });
            ui.visual(card, Visual::new().background(egui_retained::Color32::from_white_alpha(4)));
            ui.set_selected(card, on);
            ui.on_click(card, t(TimeMsg::Weather(file.clone())));
            let i = ui.add(card, Icon::new(icon));
            ui.visual(i, Visual::new().font_size(17.0_f32));
            if on {
                ui.visual(i, Visual::new().color(ACCENT));
            }
            let texts = ui.column(card);
            kit::grow(ui, texts);
            kit::gap(ui, texts, 2.0);
            let tl = ui.row(texts);
            kit::gap(ui, tl, 6.0);
            kit::text(ui, tl, &name, "strong");
            if fresh {
                let b = kit::text(ui, tl, &tr("NEW"), "badge");
                ui.add_class(b, "badge-accent");
            }
            kit::text(ui, texts, &meta, "faint");
        }
    }

    fn sync_editor(&mut self, ui: &mut Ui, w: &CustomWeather) {
        if !self.editor_built {
            self.editor_built = true;
            let back = ui.add(self.editor, Button::new(tr("Choose a weather preset")).icon("arrow_back"));
            ui.on_click(back, t(TimeMsg::Presets));
            type Fmt = fn(f32, f32) -> String;
            let sliders: [(Field, &str, f32, f32, f32, Fmt); 6] = [
                (Field::Visibility, "Visibility", 50.0, 50_000.0, 50.0, |x, _| if x >= 49_950.0 { "unlimited".into() } else if x >= 1000.0 { format!("{:.1} km", x / 1000.0) } else { format!("{x:.0} m") }),
                (Field::Brightness, "Brightness", 0.0, 1.5, 0.05, |x, _| format!("{:.0} %", x * 100.0)),
                (Field::WindDir, "Wind direction", 0.0, 355.0, 5.0, |x, _| format!("{x:.0}°")),
                (Field::WindSpeed, "Wind speed", 0.0, 40.0, 0.5, |x, _| format!("{x:.1} m/s")),
                (Field::Temp, "Temperature", -30.0, 45.0, 1.0, |x, _| format!("{x:.0} °C")),
                (Field::Humidity, "Humidity", 0.0, 100.0, 1.0, |x, temp| format!("{x:.0} % · dew {:.0} °C", ws::dew_point_c(temp, x))),
            ];
            for (f, label, lo, hi, step, fmt) in sliders {
                self.slider_row(ui, f, label, lo, hi, step, fmt);
            }
            let r = kit::labelled(ui, self.editor, "Cloud type", 120.0);
            let s = ui.add(r, Select::new(ws::CUSTOM_CLOUDS.iter().map(|x| (*x).to_string()).collect(), Some(0)).on_change(|i| t(TimeMsg::Custom(Field::Cloud, i as f32))));
            kit::grow(ui, s);
            ui.set_name(s, "custom-cloud");
            let r = kit::labelled(ui, self.editor, "Precipitation", 120.0);
            let s = ui.add(r, Select::new(ws::CUSTOM_PRECIP.iter().map(|x| (*x).to_string()).collect(), Some(0)).on_change(|i| t(TimeMsg::Custom(Field::Precip, i as f32))));
            kit::grow(ui, s);
            ui.set_name(s, "custom-precip");
            self.slider_row(ui, Field::Intensity, "Precipitation intensity", 0.0, 255.0, 1.0, |x, _| format!("{x:.0} / 255"));
            self.slider_row(ui, Field::Wet, "Road wetness", 0.0, 1.0, 0.05, |x, _| format!("{:.0} %", x * 100.0));
            let r = ui.row(self.editor);
            kit::gap(ui, r, 16.0);
            let a = ui.add(r, Checkbox::switch(false, tr("Snow cover")).on_change(|v| t(TimeMsg::CustomFlag(Field::Snow, v))));
            ui.set_name(a, "custom-snow");
            let b = ui.add(r, Checkbox::switch(false, tr("Snow on road")).on_change(|v| t(TimeMsg::CustomFlag(Field::SnowRoad, v))));
            ui.set_name(b, "custom-snow-road");
        }
        for (f, v) in [(Field::Visibility, w.visibility_m), (Field::Brightness, w.brightness), (Field::WindDir, w.wind_dir), (Field::WindSpeed, w.wind_speed), (Field::Temp, w.temp_c), (Field::Humidity, w.humidity), (Field::Intensity, w.precip_intensity), (Field::Wet, w.road_wetness)] {
            if let Some(n) = ui.find(&format!("custom-{f:?}")) {
                ui.set_slider(n, v);
            }
            if let Some(n) = ui.find(&format!("custom-{f:?}-value")) {
                let text = value_text(f, v, w.temp_c);
                ui.set_text(n, &text);
            }
        }
        if let Some(n) = ui.find("custom-cloud") {
            ui.set_select(n, None, Some(w.cloud));
        }
        if let Some(n) = ui.find("custom-precip") {
            ui.set_select(n, None, Some(w.precip.clamp(0, 2) as usize));
        }
        if let Some(n) = ui.find("custom-snow") {
            ui.set_checked(n, w.snow_cover);
        }
        if let Some(n) = ui.find("custom-snow-road") {
            ui.set_checked(n, w.snow_on_road);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn slider_row(&mut self, ui: &mut Ui, f: Field, label: &str, lo: f32, hi: f32, step: f32, _fmt: fn(f32, f32) -> String) {
        let r = ui.row(self.editor);
        kit::gap(ui, r, 10.0);
        let l = kit::text(ui, r, &tr(label), "dim");
        ui.style(l, |s| {
            s.size.width = len(120.0);
            s.flex_shrink = 0.0;
        });
        let s = ui.add(r, Slider::new(lo, lo, hi).step(step).on_change(move |v| t(TimeMsg::Custom(f, v))));
        ui.set_name(s, &format!("custom-{f:?}"));
        let v = kit::text(ui, r, "", "strong");
        ui.style(v, |s| s.min_size.width = len(110.0));
        ui.set_name(v, &format!("custom-{f:?}-value"));
    }
}

fn value_text(f: Field, x: f32, temp: f32) -> String {
    match f {
        Field::Visibility => if x >= 49_950.0 { "unlimited".into() } else if x >= 1000.0 { format!("{:.1} km", x / 1000.0) } else { format!("{x:.0} m") },
        Field::Brightness | Field::Wet => format!("{:.0} %", x * 100.0),
        Field::WindDir => format!("{x:.0}°"),
        Field::WindSpeed => format!("{x:.1} m/s"),
        Field::Temp => format!("{x:.0} °C"),
        Field::Humidity => format!("{x:.0} % · dew {:.0} °C", ws::dew_point_c(temp, x)),
        Field::Intensity => format!("{x:.0} / 255"),
        _ => String::new(),
    }
}

fn parse_time(s: &str) -> Option<i32> {
    let (h, m) = s.trim().split_once(':').unwrap_or((s.trim(), "0"));
    let (h, m): (i32, i32) = (h.trim().parse().ok()?, m.trim().parse().ok()?);
    ((0..24).contains(&h) && (0..60).contains(&m)).then_some(h * 60 + m)
}

fn date_add(date: &str, days: i32) -> Option<String> {
    let (y, mo, d) = crate::launcher::ui::parse_date(date);
    let dim = |y: i32, m: u32| [31, if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31][(m as usize - 1).min(11)];
    let (mut y, mut mo, mut d) = (y as i32, mo, d as i32 + days);
    while d < 1 {
        mo = if mo == 1 { y -= 1; 12 } else { mo - 1 };
        d += dim(y, mo);
    }
    while d > dim(y, mo) {
        d -= dim(y, mo);
        mo = if mo == 12 { y += 1; 1 } else { mo + 1 };
    }
    Some(format!("{y:04}-{mo:02}-{d:02}"))
}

fn date_changed(l: &mut Launcher, d: String) {
    l.state.choice.date = d;
    l.state.choice.season = "auto".into();
    l.state.choice.own_date = None;
    l.state.load_lines();
    l.state.touched();
}

pub(super) fn handle(l: &mut Launcher, msg: TimeMsg) {
    let c = &mut l.state.choice;
    match msg {
        TimeMsg::Time(s) => match parse_time(&s) {
            Some(v) => {
                c.time = v;
                l.state.touched();
            }
            None => l.state.set_status(tr("The time is HH:MM, from 00:00 to 23:59."), true),
        },
        TimeMsg::TimeStep(d) => {
            c.time = (c.time + d * 15).rem_euclid(24 * 60);
            l.state.touched();
        }
        TimeMsg::Date(s) => {
            let s = s.trim().to_string();
            let ok = s.len() == 10 && date_add(&s, 0).is_some_and(|n| n == s);
            if ok {
                date_changed(l, s);
            } else {
                l.state.set_status(tr("The date is YYYY-MM-DD."), true);
            }
        }
        TimeMsg::DateStep(d) => {
            if let Some(n) = date_add(&c.date.clone(), d) {
                date_changed(l, n);
            }
        }
        TimeMsg::NowTime => {
            if let Some((_, _, _, h, mi)) = omsi_launcher_lib::local_now() {
                c.time = h * 60 + mi;
                l.state.touched();
            }
        }
        TimeMsg::NowDate => {
            if let Some((yy, mo, d, _, _)) = omsi_launcher_lib::local_now() {
                date_changed(l, format!("{yy:04}-{mo:02}-{d:02}"));
            }
        }
        TimeMsg::Season(k) => {
            l.state.set_season(SEASONS[k.min(4)]);
            logic::season_weather_fits(l);
        }
        TimeMsg::Phase(k) => {
            l.state.choice.phase = PHASES[k.min(2)].to_string();
            l.state.season_chosen();
            logic::season_weather_fits(l);
        }
        TimeMsg::Traffic(v) => {
            c.traffic = v;
            l.state.touched();
        }
        TimeMsg::Passengers(v) => {
            c.passengers = v;
            l.state.touched();
        }
        TimeMsg::Schedule(v) => {
            c.schedule = v;
            l.state.touched();
        }
        TimeMsg::Autostart(v) => {
            c.autostart = v;
            l.state.touched();
        }
        TimeMsg::OnFoot(v) => {
            c.on_foot = v;
            l.state.touched();
        }
        TimeMsg::Weather(f) => {
            c.weather = f;
            l.state.touched();
        }
        TimeMsg::EditAsCustom => {
            if let Some(custom) = logic::selected_weather_as_custom(&l.state.config.root, &l.state.choice.weather) {
                l.state.choice.weather = custom;
                l.state.touched();
            }
        }
        TimeMsg::Presets => {
            c.weather.clear();
            l.state.touched();
        }
        TimeMsg::Custom(f, v) => custom(l, f, v, None),
        TimeMsg::CustomFlag(f, b) => custom(l, f, 0.0, Some(b)),
        TimeMsg::Airport(s) => {
            let a: String = s.chars().filter(|ch| ch.is_ascii_alphabetic()).take(4).collect::<String>().to_uppercase();
            c.weather = if a.is_empty() { "metar:".into() } else { format!("metar:{a}") };
            l.state.touched();
        }
        TimeMsg::AirportPick(i) => {
            let list = ws::metar_airports(std::path::Path::new(&l.state.config.root));
            if let Some(a) = list.get(i) {
                l.state.choice.weather = format!("metar:{}", a.0);
                l.state.touched();
            }
        }
    }
}

/// One value of the custom weather changed.
fn custom(l: &mut Launcher, f: Field, v: f32, flag: Option<bool>) {
    let Some(mut w) = ws::custom_weather(Some(&l.state.choice.weather)) else { return };
    match f {
        Field::Visibility => w.visibility_m = v,
        Field::Brightness => w.brightness = v,
        Field::WindDir => w.wind_dir = v,
        Field::WindSpeed => w.wind_speed = v,
        Field::Temp => w.temp_c = v,
        Field::Humidity => w.humidity = v,
        Field::Cloud => w.cloud = v as usize,
        Field::Precip => w.precip = v as i32,
        Field::Intensity => w.precip_intensity = v,
        Field::Wet => w.road_wetness = v,
        Field::Snow => w.snow_cover = flag.unwrap_or(w.snow_cover),
        Field::SnowRoad => w.snow_on_road = flag.unwrap_or(w.snow_on_road),
    }
    w.normalize();
    l.state.choice.weather = w.encode();
    l.state.touched();
}
