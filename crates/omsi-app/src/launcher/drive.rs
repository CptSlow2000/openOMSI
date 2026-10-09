//! The Drive page: three steps in a column on the left - the bus, the day and the weather,
//! the map and the duty - and a stage on the right: the bus on its turntable for the first
//! two, the map itself for the third (see `mapview`: it is dragged, zoomed and clicked, not
//! looked at), with the roadbook beside it. Under the stage a foot sums the choice up and
//! holds the buttons that go on.
//!
//! The order is what a player does: what to drive, then when, then where.

use super::state::{hhmm, trip_index_at};
use super::Launcher;
use omsi_launcher_lib::{display_bus_name, vehicle_type_label, WeatherInfo};

#[derive(Clone)]
pub(crate) struct BusVariant {
    pub(crate) file: String,
    pub(crate) name: String,
    pub(crate) variant: String,
    pub(crate) fresh: bool,
    pub(crate) installed: bool,
    pub(crate) paints: usize,
    pub(crate) incomplete: bool,
}

#[derive(Clone)]
pub(crate) struct BusManufacturer {
    pub(crate) key: String,
    pub(crate) name: String,
    pub(crate) variants: Vec<BusVariant>,
}

#[derive(Default)]
pub struct DriveView {
    /// The Drive page's step: 0 the bus, 1 the day and the weather, 2 the map and the duty.
    pub tab: usize,
}

pub(crate) fn favourites_file() -> std::path::PathBuf {
    omsi_launcher_lib::data_dir().join("favourite-buses.txt")
}

pub(crate) fn fav_key(file: &str) -> String {
    file.replace('\\', "/").to_lowercase()
}

/// The starred buses of the file (one bus file a line).
pub(crate) fn read_favourites() -> std::collections::BTreeSet<String> {
    std::fs::read_to_string(favourites_file()).map(|t| t.lines().map(str::trim).filter(|l| !l.is_empty()).map(fav_key).collect()).unwrap_or_default()
}

pub(crate) fn write_favourites(f: &std::collections::BTreeSet<String>) {
    let text: String = f.iter().map(|l| format!("{l}\n")).collect();
    if let Err(e) = std::fs::write(favourites_file(), text) {
        log::warn!("favourite buses not saved: {e}");
    }
}

/// The icon a weather file deserves: what it says about itself.
pub(crate) fn weather_icon_of(w: &WeatherInfo) -> &'static str {
    if w.snow || w.precip.starts_with("snow") {
        "weather_snowy"
    } else if w.precip.starts_with("rain") {
        "rainy"
    } else if w.fog_m < 1500.0 {
        "foggy"
    } else if w.clouds.to_lowercase().contains("overcast") {
        "cloud"
    } else if w.clouds.to_lowercase().contains("cumulus") {
        "partly_cloudy_day"
    } else {
        "wb_sunny"
    }
}

/// Whether all of a tour's trips have left at `now`, and the trip a start then takes: the one
/// under way or the next, or with a search (`q`, lower case) the first still to come that
/// matches it by name, line or stop (a tour whose number matches keeps the usual one).
pub(crate) fn tour_trip(t: &omsi_launcher_lib::TourInfo, now: f64, q: &str) -> (bool, Option<usize>) {
    let ended = t.runs && t.trips.iter().all(|x| x.departure < now - 120.0);
    let first = trip_index_at(t, now);
    let has = |s: &str| s.to_lowercase().contains(q);
    if q.is_empty() || has(&t.number) {
        return (ended, first);
    }
    let from = if ended { 0 } else { first.unwrap_or(0) };
    let hit = t.trips.iter().enumerate().skip(from).find(|(_, x)| {
        has(&x.name) || has(&x.line) || has(&x.from) || has(&x.terminus) || x.stops.iter().any(|s| has(&s.name))
    });
    (ended, hit.map(|(k, _)| k))
}

/// The livery the bus would wear.
pub(crate) fn paint_line(l: &Launcher) -> String {
    match l.state.bus() {
        Some(bus) if l.state.choice.paint.is_empty() => default_livery_label(bus).to_string(),
        Some(_) => l.state.choice.paint.clone(),
        None => String::new(),
    }
}

/// The chosen day and weather in one line.
pub(crate) fn start_line(l: &Launcher) -> String {
    // on a server: its clock and its weather, whatever this machine has chosen
    if let Some(i) = l.state.joined_server.as_ref().and_then(|a| l.state.server_info.get(a)).and_then(|x| x.1.as_ref().ok()) {
        let weather = if i.weather.is_empty() {
            omsi_ui::tr("the map's (the server's)").into_owned()
        } else {
            format!("{} {}", i.weather, omsi_ui::tr("(the server's)"))
        };
        return format!("{} {} · {weather}", i.time, omsi_ui::tr("(the server's clock)"));
    }
    let weather = match l.state.choice.weather.strip_prefix("metar:") {
        Some(code) => format!("at {code}"),
        None if l.state.choice.weather == "cycle" => omsi_ui::tr("Weather cycle").into_owned(),
        None if crate::weather_model::is_natural(Some(&l.state.choice.weather)) || l.state.choice.weather.is_empty() => omsi_ui::tr("Natural weather").into_owned(),
        None if crate::weather_setup::custom_weather(Some(&l.state.choice.weather)).is_some() => {
            let c = crate::weather_setup::custom_weather(Some(&l.state.choice.weather)).unwrap();
            format!("{} · {}", omsi_ui::tr("Custom"), custom_weather_summary(&c))
        }
        None => l.state.weathers.iter().find(|w| w.file == l.state.choice.weather).map(|w| w.name.clone()).unwrap_or_else(|| "the map's weather".into()),
    };
    let (yy, mm, dd) = super::ui::parse_date(&l.state.choice.date);
    format!("{:02}:{:02}, {dd} {} {yy} · {weather}", l.state.choice.time / 60, l.state.choice.time % 60, super::ui::MONTHS[(mm as usize).clamp(1, 12) - 1])
}

/// The duty in words: the line and the tour, and when the trip the game would take runs.
pub(crate) fn duty_of(l: &Launcher) -> (String, String) {
    let map = l.state.map().map(|m| if m.friendly.is_empty() { m.name.clone() } else { m.friendly.clone() }).unwrap_or_else(|| "-".into());
    match (&l.state.choice.line, &l.state.choice.tour, l.state.choice.free) {
        (_, _, true) | (None, _, _) => (format!("Free drive · {map}"), String::new()),
        (Some(line), Some(t), _) => {
            let trip = l.state.tour().and_then(|t| t.trips.get(l.state.first_trip().unwrap_or(0)));
            let when = trip.map(|x| format!("{} - {} · {:.1} km · {} → {}", hhmm(x.departure), hhmm(x.arrival), x.km, if x.from.is_empty() { "?" } else { &x.from }, x.terminus)).unwrap_or_default();
            (format!("Line {line} · tour {t} · {map}"), when)
        }
        (Some(line), None, _) => (format!("Line {line} · choose a tour · {map}"), String::new()),
    }
}

/// Where the bus is put down.
pub(crate) fn duty_place(l: &Launcher) -> String {
    match l.state.choice.entry {
        e if e < 0 => "starting point: automatic".to_string(),
        e => l
            .state
            .map()
            .and_then(|m| m.entry_points.get(e as usize))
            .map(|x| format!("starting at {}", if x.name.is_empty() { format!("entry {}", x.index + 1) } else { x.name.clone() }))
            .unwrap_or_else(|| "starting point: automatic".into()),
    }
}

/// OMSI takes the manufacturer and the complete type from [friendlyname]. The
/// vehicle folder and rendering configuration do not define this hierarchy.
pub(crate) fn build_bus_manufacturers(vehicles: &[omsi_launcher_lib::VehicleInfo], allowed: Option<&std::collections::HashSet<String>>, fresh: &std::collections::HashSet<String>) -> Vec<BusManufacturer> {
    let mut grouped = std::collections::BTreeMap::<String, BusManufacturer>::new();
    for vehicle in vehicles {
        if !allowed.map(|a| a.contains(&vehicle.file.replace('\\', "/").to_lowercase())).unwrap_or(true) {
            continue;
        }
        let maker = vehicle.manufacturer.trim();
        let key = maker.to_lowercase();
        let group = grouped.entry(key.clone()).or_insert_with(|| BusManufacturer {
            key, name: if maker.is_empty() { "Unknown manufacturer".into() } else { display_bus_name(maker) }, variants: Vec::new(),
        });
        let type_name = vehicle_type_label(&vehicle.type_name, std::path::Path::new(&vehicle.file));
        group.variants.push(BusVariant {
            file: vehicle.file.clone(), name: display_bus_name(&vehicle.name), variant: type_name,
            fresh: fresh.contains(&vehicle.file), installed: vehicle.installed,
            paints: vehicle.paints.len(), incomplete: !vehicle.missing_packs.is_empty(),
        });
    }
    let mut manufacturers: Vec<BusManufacturer> = grouped.into_values().collect();
    for maker in &mut manufacturers {
        // Distinct .bus files remain selectable even when add-ons repeat a friendly
        // type name. Show the pack, and the file only if the pack also repeats it.
        let mut counts = std::collections::HashMap::<String, usize>::new();
        for variant in &maker.variants { *counts.entry(variant.variant.to_lowercase()).or_default() += 1; }
        for variant in &mut maker.variants {
            if counts[&variant.variant.to_lowercase()] > 1 {
                let folder = variant.file.replace('\\', "/").split('/').nth(1).unwrap_or_default().to_string();
                variant.variant = format!("{} · {}", variant.variant, display_bus_name(&folder));
            }
        }
        let mut counts = std::collections::HashMap::<String, usize>::new();
        for variant in &maker.variants { *counts.entry(variant.variant.to_lowercase()).or_default() += 1; }
        for variant in &mut maker.variants {
            if counts[&variant.variant.to_lowercase()] > 1 {
                let stem = std::path::Path::new(&variant.file).file_stem().unwrap_or_default().to_string_lossy();
                variant.variant = format!("{} · {}", variant.variant, display_bus_name(&stem));
            }
        }
        maker.variants.sort_by(|a, b| bus_name_cmp(&a.variant, &b.variant).then_with(|| a.file.cmp(&b.file)));
    }
    manufacturers.sort_by(|a, b| bus_name_cmp(&a.name, &b.name).then_with(|| a.key.cmp(&b.key)));
    manufacturers
}

/// Sort numeric runs wherever they occur: DL9 precedes DL10; case does not change
/// a manufacturer's position.
pub(crate) fn bus_name_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    let a = a.to_lowercase();
    let b = b.to_lowercase();
    let mut a = a.chars().peekable();
    let mut b = b.chars().peekable();
    loop {
        match (a.peek().copied(), b.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, _) => return Ordering::Less,
            (_, None) => return Ordering::Greater,
            (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => {
                let x: String = std::iter::from_fn(|| a.next_if(|c| c.is_ascii_digit())).collect();
                let y: String = std::iter::from_fn(|| b.next_if(|c| c.is_ascii_digit())).collect();
                let x = x.trim_start_matches('0');
                let y = y.trim_start_matches('0');
                let order = x.len().cmp(&y.len()).then_with(|| x.cmp(y));
                if order != Ordering::Equal { return order; }
            }
            (Some(x), Some(y)) => {
                let order = x.cmp(&y);
                if order != Ordering::Equal { return order; }
                a.next(); b.next();
            }
        }
    }
}

pub(super) fn default_livery_label(vehicle: &omsi_launcher_lib::VehicleInfo) -> &str {
    if vehicle.default_paint.trim().is_empty() { "Default paint" } else { vehicle.default_paint.trim() }
}

/// How many liveries a bus type with `repaints` has, as its Livery list counts them (its own
/// paint and the repaints): beside each type in the list, as a family says how many types it
/// has (#717).
pub(super) fn liveries_text(repaints: usize) -> String {
    let n = repaints + 1;
    format!("{n} {}", omsi_ui::tr(if n == 1 { "livery" } else { "liveries" }))
}

/// A type of a bus family in its dropdown: the type and its liveries.
pub(crate) fn variant_option(variant: &BusVariant) -> String {
    format!("{} · {}", variant.variant, liveries_text(variant.paints))
}

pub(crate) fn variant_matches(variant: &BusVariant, q: &str) -> bool {
    q.is_empty() || variant.name.to_lowercase().contains(q) || variant.variant.to_lowercase().contains(q) || display_bus_name(&variant.file).to_lowercase().contains(q)
}

pub(crate) fn manufacturer_matches(model: &BusManufacturer, q: &str) -> bool {
    q.is_empty() || model.name.to_lowercase().contains(q) || model.variants.iter().any(|v| variant_matches(v, q))
}

pub(super) fn natural(s: &str) -> (u64, String) {
    let digits: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
    (digits.parse().unwrap_or(u64::MAX), s.to_string())
}

/// Time from the departure to the arrival of one trip.
pub(crate) fn trip_duration(first: f64, last: f64) -> String {
    let minutes = ((last - first).max(0.0) / 60.0).round() as i64;
    let hours = minutes / 60;
    let remaining = minutes % 60;
    let count = |n: i64, one: &str, many: &str| format!("{n} {}", omsi_ui::tr(if n == 1 { one } else { many }));
    match (hours, remaining) {
        (0, m) => count(m, "minute", "minutes"),
        (h, 0) => count(h, "hour", "hours"),
        (h, m) => format!("{} {}", count(h, "hour", "hours"), count(m, "minute", "minutes")),
    }
}

/// The name of the server the Drive page is joined to (see the Multiplayer page).
pub(crate) fn joined_server_name(l: &Launcher) -> Option<String> {
    let a = l.state.joined_server.as_ref()?;
    let entry = l.state.servers.iter().find(|s| &s.address == a);
    let info = l.state.server_info.get(a).and_then(|x| x.1.as_ref().ok());
    Some(entry.map(|e| e.name.clone()).filter(|n| !n.is_empty()).or_else(|| info.map(|i| i.name.clone())).unwrap_or_else(|| a.clone()))
}

/// After the season changed: a chosen weather that does not fit it goes, and the duty is saved.
pub(crate) fn season_weather_fits(l: &mut Launcher) {
    let w = l.state.choice.weather.clone();
    if let Some(wi) = l.state.weathers.iter().find(|x| x.file == w).cloned() {
        if !l.state.weather_fits(&wi) {
            l.state.choice.weather.clear();
        }
    }
    l.state.touched();
}

/// What the map picture should show, from the current choice: the map, the trip of the
/// chosen tour that a start now would begin with (`State::first_trip`, the same one the
/// launch choice takes), and the entry point the player picked.
pub(crate) fn map_look(l: &Launcher) -> super::mapview::Look {
    let file = l.state.choice.map.clone();
    // (the map list came from the content roots; ask them the same way, case-insensitively,
    // so a mod's map is found wherever its root sits)
    let global = omsi_cfg::find_in_roots(&file)
        .map(|(_, p)| p)
        .unwrap_or_else(|| omsi_cfg::resolve_path(std::path::Path::new(&l.state.config.root), &file));
    let trip = l.state.tour().and_then(|t| t.trips.get(l.state.first_trip().unwrap_or(0))).map(|t| t.name.clone()).unwrap_or_default();
    super::mapview::Look { map: file, global, date: l.state.choice.date.clone(), trip, entry: l.state.choice.entry }
}

/// The phone's Start: as the desktop's.
pub(super) fn start_from_phone(l: &mut Launcher) {
    start(l);
}

pub(crate) fn start(l: &mut Launcher) {
    if l.state.bus().is_none() || l.state.map().is_none() {
        l.state.set_status("Choose a bus and a map first.", true);
        return;
    }
    if l.state.choice.lan_mode == "join" && !l.state.join.0 {
        let t = l.state.join.1.clone();
        l.state.set_status(format!("LAN: {t}"), true);
        return;
    }
    let running = l.state.instances.iter().filter(|i| i.running).count();
    // a second game on one computer is for testing LAN play, not something to do by
    // accident: with one running, the button asks for a second click
    if running > 0 && l.state.second_armed.map(|t| t.elapsed().as_secs() >= 6).unwrap_or(true) {
        l.state.second_armed = Some(std::time::Instant::now());
        l.state.set_status("A game is running already (its window may be behind this one - see Sessions). Click again to start another one anyway.", true);
        return;
    }
    l.state.second_armed = None;
    l.state.choice.save();
    l.state.launch();
    if l.state.choice.lan_mode != "off" {
        l.go(super::Page::Sessions);
    }
}

/// The airport of OMSI's METAR list (`Weather/ICAO.txt`) nearest to where map `map` lies
/// (its `timezone.txt`), else Berlin's; read once per map.
pub(super) fn custom_weather_summary(c:&crate::weather_setup::CustomWeather)->String{
    let precip=match c.precip{
        1=>format!("rain {:.0}%",c.precip_intensity/255.0*100.0),
        2=>format!("snow {:.0}%",c.precip_intensity/255.0*100.0),
        _=>"dry".to_string(),
    };
    let vis=if c.visibility_m>=49_950.0{
        "clear visibility".to_string()
    }else if c.visibility_m>=1000.0{
        format!("{:.1} km",c.visibility_m/1000.0)
    }else{
        format!("{:.0} m",c.visibility_m)
    };
    format!("{:.0} °C · {:.0}% RH · {precip} · {vis}",c.temp_c,c.humidity)
}

pub(super) fn selected_weather_as_custom(root:&str,file:&str)->Option<String>{
    if file.is_empty()||file=="cycle"||file.to_ascii_lowercase().starts_with("metar:")||crate::weather_setup::custom_weather(Some(file)).is_some(){return None}
    let path=omsi_cfg::resolve_path(std::path::Path::new(root),file);
    let w=omsi_content::weather::Weather::load(&path).ok()?;
    let wet=(w.ground_wet[0]/255.0).clamp(0.0,1.0);
    Some(crate::weather_setup::CustomWeather::from_weather(&w,1.0,wet).encode())
}

pub(crate) fn nearest_airport(root: &str, map: &str) -> String {
    static CACHE: std::sync::Mutex<Option<hashbrown::HashMap<String, String>>> = std::sync::Mutex::new(None);
    let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    let cache = cache.get_or_insert_with(Default::default);
    if let Some(c) = cache.get(map) {
        return c.clone();
    }
    // (the airports of the list round Europe and a little beyond, where OMSI's maps lie)
    const AIRPORTS: &[(&str, f64, f64)] = &[
        ("BKPR", 42.57, 21.04), ("EBBR", 50.90, 4.48), ("EDDH", 53.63, 9.99), ("EDDM", 48.35, 11.79),
        ("EDDN", 49.50, 11.08), ("EDDP", 51.42, 12.24), ("EDDS", 48.69, 9.22), ("EDDB", 52.37, 13.52),
        ("EDOP", 53.43, 11.78), ("EETN", 59.41, 24.83), ("EFHF", 60.25, 25.04), ("EGAA", 54.66, -6.22),
        ("EGCC", 53.35, -2.27), ("EGLL", 51.47, -0.45), ("EGPH", 55.95, -3.37), ("EHAM", 52.31, 4.76),
        ("EIDW", 53.42, -6.27), ("EKBI", 55.74, 9.15), ("ELLX", 49.63, 6.21), ("ENBR", 60.29, 5.22),
        ("ENGM", 60.19, 11.10), ("ESKN", 58.79, 16.91), ("EPKK", 50.08, 19.78), ("EPWA", 52.17, 20.97),
        ("EVRA", 56.92, 23.97), ("EYVI", 54.63, 25.29), ("LBSF", 42.70, 23.41), ("LDZA", 45.74, 16.07),
        ("LEAL", 38.28, -0.56), ("LEBB", 43.30, -2.91), ("LEMD", 40.47, -3.56), ("LEPA", 39.55, 2.74),
        ("LFBD", 44.83, -0.72), ("LFLY", 45.73, 5.08), ("LFMD", 43.54, 6.95), ("LFPG", 49.01, 2.55),
        ("LGAV", 37.94, 23.94), ("LHBP", 47.44, 19.26), ("LICJ", 38.18, 13.10), ("LIMC", 45.63, 8.72),
        ("LIRA", 41.80, 12.59), ("LOWI", 47.26, 11.34), ("LOWL", 48.23, 14.19), ("LOWW", 48.11, 16.57),
        ("LPPT", 38.78, -9.14), ("LQSA", 43.82, 18.33), ("LSGG", 46.24, 6.11), ("LSZH", 47.46, 8.55),
        ("LTAC", 40.13, 32.99), ("LWSK", 41.96, 21.62), ("LYBE", 44.82, 20.31), ("LYTV", 42.40, 18.72),
        ("LZIB", 48.17, 17.21), ("UKKK", 50.40, 30.45), ("ULLI", 59.80, 30.26), ("UMKK", 54.89, 20.59),
        ("UMMM", 53.88, 28.03), ("UUEE", 55.97, 37.41), ("USSS", 56.74, 60.80),
    ];
    let dir = std::path::Path::new(map).parent().map(|d| d.to_string_lossy().to_string()).unwrap_or_default();
    let tz = omsi_cfg::resolve_path(std::path::Path::new(root), &format!("{dir}/timezone.txt"));
    let code = omsi_map::TimeZone::load(&tz)
        .ok()
        .and_then(|t| t.lat_lon())
        .and_then(|(lat, lon)| {
            AIRPORTS
                .iter()
                .map(|(c, a, o)| (c, (a - lat).powi(2) + ((o - lon) * lat.to_radians().cos()).powi(2)))
                .min_by(|x, y| x.1.total_cmp(&y.1))
                .map(|x| x.0.to_string())
        })
        .unwrap_or_else(|| "EDDB".into());
    cache.insert(map.to_string(), code.clone());
    code
}

#[cfg(test)]
mod vehicle_picker_tests {
    use super::*;

    fn vehicle(folder: &str, maker: &str, name: &str, file: &str) -> omsi_launcher_lib::VehicleInfo {
        omsi_launcher_lib::VehicleInfo {
            name: format!("{maker} {name}"), manufacturer: maker.into(), type_name: name.into(),
            folder: folder.into(), file: format!("Vehicles/{folder}/{file}.bus"),
            description: String::new(), paints: vec!["Paint".into()], hofs: vec![],
            installed: false, missing_packs: vec![], numbers: vec![], default_paint: "Beige".into(),
        }
    }

    #[test]
    fn a_tour_search_finds_the_next_trip_of_a_route_and_skips_the_ones_gone() {
        let trip = |index: usize, name: &str, departure: f64| omsi_launcher_lib::TripInfo {
            name: name.into(),
            index,
            line: "9106".into(),
            from: "Massy".into(),
            terminus: "Cormeilles".into(),
            departure,
            arrival: departure + 1800.0,
            stops: Vec::new(),
            km: 10.0,
        };
        let tour = |trips: Vec<omsi_launcher_lib::TripInfo>| omsi_launcher_lib::TourInfo {
            number: "12".into(),
            ai_group: String::new(),
            first: 0.0,
            last: 0.0,
            days: "Mon-Fri".into(),
            runs: true,
            next_run: None,
            trips,
        };
        let h = |x: f64| x * 3600.0;
        let morning_b = tour(vec![trip(1, "9106B_HC_MASSY", h(7.0)), trip(2, "9106A_HC_CORMEILLES", h(8.0)), trip(3, "9106A_HC_MASSY", h(13.0))]);
        assert_eq!(tour_trip(&morning_b, h(12.0), "9106b"), (false, None));
        assert_eq!(tour_trip(&morning_b, h(12.0), ""), (false, Some(2)));
        let later_b = tour(vec![trip(1, "9106A_HC_MASSY", h(11.5)), trip(2, "9106B_HC_CORMEILLES", h(12.5)), trip(3, "9106B_HC_MASSY", h(14.0))]);
        assert_eq!(tour_trip(&later_b, h(12.0), "9106b"), (false, Some(1)));
        assert_eq!(tour_trip(&later_b, h(12.0), "12"), (false, Some(1)));
        assert_eq!(tour_trip(&later_b, h(15.0), ""), (true, Some(2)));
        assert_eq!(tour_trip(&later_b, h(15.0), "9106b"), (true, Some(1)));
    }

    #[test]
    fn empty_vehicle_types_use_the_file_name() {
        for empty in ["", "   "] {
            let manufacturers = build_bus_manufacturers(
                &[vehicle("Pack", "MAN", empty, "NL_202")],
                None,
                &Default::default(),
            );
            assert_eq!(manufacturers[0].variants[0].variant, "NL 202");
        }
    }

    #[test]
    fn omsi_manufacturer_groups_dl_and_lions_city_across_packs() {
        let vehicles = vec![
            vehicle("MAN_DL05", "MAN", "DL05", "dl05"),
            vehicle("MAN_DL05", "MAN", "DL07", "dl07"),
            vehicle("MAN_DL05", "MAN", "DL08", "dl08"),
            vehicle("MAN_DL05", "MAN", "DL09", "dl09"),
            vehicle("MAN_LC_MVG", "MAN", "Lion's City (MVG)", "mvg"),
            vehicle("MAN_LC_GUE", "MAN", "Lion's City G (ORN)", "orn"),
        ];
        let manufacturers = build_bus_manufacturers(&vehicles, None, &Default::default());
        assert_eq!(manufacturers.len(), 1);
        assert_eq!(manufacturers[0].name, "MAN");
        assert_eq!(manufacturers[0].variants.len(), 6);
        assert!(manufacturers[0].variants.iter().any(|v| v.variant == "DL07"));
        assert!(manufacturers[0].variants.iter().any(|v| v.variant == "Lion's City G (ORN)"));
        assert!(manufacturer_matches(&manufacturers[0], "orn"));
        assert!(!manufacturer_matches(&manufacturers[0], "not a bus"));
    }

    #[test]
    fn author_defined_manufacturer_and_complete_type_are_preserved() {
        let vehicles = vec![
            vehicle("Pack", "Mercedes-Benz Release", "MB_C2_E6_GN_BVG_Leasing 2", "leasing"),
            vehicle("Pack", "Mercedes-Benz", "O530", "o530"),
        ];
        let manufacturers = build_bus_manufacturers(&vehicles, None, &Default::default());
        assert_eq!(manufacturers.len(), 2);
        let maker = manufacturers.iter().find(|m| m.name == "Mercedes-Benz Release").unwrap();
        assert_eq!(maker.variants[0].variant, "MB C2 E6 GN BVG Leasing 2");
    }

    #[test]
    fn duplicate_names_remain_selectable_and_host_filter_is_respected() {
        let vehicles = vec![
            vehicle("MAN", "MAN", "NL202", "en92"),
            vehicle("MAN", "MAN", "NL202", "en93"),
            vehicle("OtherPack", "MAN", "NL202", "en92"),
        ];
        let manufacturers = build_bus_manufacturers(&vehicles, None, &Default::default());
        let labels: std::collections::HashSet<_> = manufacturers[0].variants.iter().map(|v| &v.variant).collect();
        assert_eq!(labels.len(), 3);
        let allowed = std::collections::HashSet::from([vehicles[1].file.to_lowercase()]);
        let filtered = build_bus_manufacturers(&vehicles, Some(&allowed), &Default::default());
        assert_eq!(filtered[0].variants.len(), 1);
        assert_eq!(filtered[0].variants[0].file, vehicles[1].file);
    }

    /// Each type in a family's dropdown says how many liveries it has, as the Livery list
    /// beside counts them: its own paint and its repaints (#717).
    #[test]
    fn each_bus_type_says_how_many_liveries_it_has() {
        let mut one = vehicle("MAN_SD200", "MAN", "SD77", "sd77");
        one.paints.clear();
        let mut many = vehicle("MAN_SD200", "MAN", "SD78", "sd78");
        many.paints = vec!["BVG".into(), "Werbung".into(), "Neu".into()];
        let manufacturers = build_bus_manufacturers(&[one, many], None, &Default::default());
        let options: Vec<String> = manufacturers[0].variants.iter().map(variant_option).collect();
        assert_eq!(options, vec!["SD77 · 1 livery".to_string(), "SD78 · 4 liveries".to_string()]);
    }

    #[test]
    fn numbers_in_type_names_sort_naturally_and_default_livery_has_its_omsi_name() {
        assert_eq!(bus_name_cmp("DL9", "DL10"), std::cmp::Ordering::Less);
        assert_eq!(bus_name_cmp("MAN", "man"), std::cmp::Ordering::Equal);
        let mut vehicle = vehicle("MAN", "MAN", "DL07", "dl07");
        assert_eq!(default_livery_label(&vehicle), "Beige");
        vehicle.default_paint.clear();
        assert_eq!(default_livery_label(&vehicle), "Default paint");
    }
}

/// A trip without passengers (to or from the depot): no line, or a terminus that says so -
/// Spandau's are "Betriebsfahrt" with the line of the trip they lead to.
pub(crate) fn depot_run(t: &omsi_launcher_lib::TripInfo) -> bool {
    let to = t.terminus.to_lowercase();
    t.line.trim().is_empty()
        || ["betriebsfahrt", "leerfahrt", "dienstfahrt", "not in service", "out of service", "hors service", "zjazd do zajezdni"].iter().any(|w| to.contains(w))
}

#[cfg(test)]
mod depot_run_tests {
    use omsi_launcher_lib::TripInfo;

    fn trip(line: &str, terminus: &str) -> TripInfo {
        TripInfo { name: String::new(), index: 1, line: line.into(), from: "Omnibushof".into(), terminus: terminus.into(), departure: 0.0, arrival: 0.0, stops: Vec::new(), km: 0.0 }
    }

    /// Spandau's tours begin and end with "Betriebsfahrt" runs (#1891).
    #[test]
    fn a_depot_run_is_not_where_a_tour_goes() {
        assert!(super::depot_run(&trip("92", "Betriebsfahrt")));
        assert!(super::depot_run(&trip("", "Hof")));
        assert!(!super::depot_run(&trip("92", "S Spandau")));
    }
}
