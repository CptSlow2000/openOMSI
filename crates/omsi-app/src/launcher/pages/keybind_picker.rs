//! The keyboard page's "Add binding" picker: every action a key can be given - the
//! installed key-language files' and OMSI's events, the triggers of every installed bus's
//! scripts (mods' too: found in the background, and kept in a cache for the next start) and
//! the ones the keyboard file has - searchable by name, by what it does and by the bus that
//! uses it.

use super::*;
use std::collections::HashMap;

/// An action the picker offers: its name, what it does, where it was found (the language
/// files, OMSI's events, a bus's script, the keyboard file) and the bus files that use it.
#[derive(Clone)]
pub(crate) struct KeyActionOption {
    pub(crate) action: String,
    pub(crate) label: String,
    pub(crate) sources: Vec<String>,
    pub(crate) bus_paths: Vec<String>,
}

/// Every action the vehicles' section can be given: the language files' texts, OMSI's events
/// (in their own spelling), the buses' script triggers and what the keyboard file has.
fn action_options(names: &crate::describe::ControlNames, script_actions: &HashMap<String, Vec<String>>, bindings: &Value) -> Vec<KeyActionOption> {
    let mut actions: HashMap<String, KeyActionOption> = HashMap::new();
    let mut found = |action: &str, label: Option<&str>, source: Option<String>| {
        let option = actions.entry(action.to_ascii_lowercase()).or_insert_with(|| KeyActionOption {
            action: action.to_string(),
            label: label.map_or_else(|| action_text(names, action), str::to_string),
            sources: Vec::new(),
            bus_paths: Vec::new(),
        });
        if let Some(source) = source.filter(|s| !option.sources.contains(s)) {
            option.sources.push(source);
        }
    };
    for (action, label) in names.actions() {
        found(&action, Some(&label), Some("Installed key-language files".into()));
    }
    for (action, label) in names.events() {
        found(&action, Some(&label), Some("OMSI event".into()));
    }
    for (action, sources) in script_actions {
        found(action, None, None);
        for source in sources {
            found(action, None, Some(format!("Bus script: {source}")));
        }
    }
    for action in bindings.get("vehicles").and_then(Value::as_array).into_iter().flatten().filter_map(|b| b.get("action").and_then(Value::as_str)) {
        found(action, None, Some("Configured binding".into()));
    }
    // OMSI's event browser uses `events()`, which keeps the spelling of the language files'
    // spelling table: the picker shows that one
    let events: HashMap<String, (String, String)> = names.events().into_iter().map(|(a, l)| (a.to_ascii_lowercase(), (a, l))).collect();
    let mut options: Vec<_> = actions.into_iter().map(|(key, mut option)| {
        if let Some((action, label)) = events.get(&key) {
            option.action = action.clone();
            option.label = label.clone();
        }
        option.bus_paths = bus_source_paths(&option.sources);
        option
    }).collect();
    options.sort_by(|a, b| a.label.to_lowercase().cmp(&b.label.to_lowercase()).then_with(|| a.action.to_lowercase().cmp(&b.action.to_lowercase())));
    options
}

/// The actions a controller's button can be given on the launcher's page, and their labels:
/// the keyboard file's vehicle actions, the H-pattern gates and the game's own (built once
/// and kept, not every frame).
pub(crate) fn controller_action_choices(names: &crate::describe::ControlNames, bindings: &Value) -> (Vec<String>, Vec<String>) {
    let mut actions: Vec<String> = vec!["<none>".into()];
    actions.extend(bindings.get("vehicles").and_then(|a| a.as_array()).map(|a| a.iter().filter_map(|b| b.get("action").and_then(|x| x.as_str()).map(String::from)).collect::<Vec<_>>()).unwrap_or_default());
    // H-pattern shifters use OMSI's "_fest" actions: pressing the gate selects the gear,
    // releasing it fires "_fest_off", which lets the bus script return to neutral.
    for a in ["kw_s_R_fest", "kw_s_1_fest", "kw_s_2_fest", "kw_s_3_fest", "kw_s_4_fest", "kw_s_5_fest", "kw_s_6_fest", "kw_s_7_fest", "kw_s_8_fest", "kw_s_9_fest", "kw_s_10_fest"] {
        if !actions.iter().any(|x| x.eq_ignore_ascii_case(a)) {
            actions.push(a.to_string());
        }
    }
    // the game's own view actions (looking around while held, the cameras, the views)
    for a in PAD_GAME_ACTIONS {
        if !actions.iter().any(|x| x == a) {
            actions.insert(1, a.to_string());
        }
    }
    actions.dedup();
    let labels: Vec<String> = actions.iter().enumerate().map(|(i, a)| if i == 0 { a.clone() } else { action_text(names, a) }).collect();
    (actions, labels)
}

/// The actions whose name or label has `query` and that a bus matching `source_query` uses.
pub(crate) fn filter_action_options(options: &[KeyActionOption], query: &str, source_query: &str) -> Vec<KeyActionOption> {
    let query = query.trim().to_lowercase();
    let source_query = normalize_source_query(source_query);
    options.iter().filter(|option| {
        (query.is_empty()
            || option.action.to_lowercase().contains(&query)
            || option.label.to_lowercase().contains(&query))
            && (source_query.is_empty()
                || option.sources.iter().any(|source| {
                    let path = source.strip_prefix("Bus script: ").unwrap_or(source);
                    let path = path.split_once(" (").map(|(path, _)| path).unwrap_or(path);
                    normalize_source_query(path).contains(&source_query)
                }))
    }).cloned().collect()
}

/// Lower case, every run of other characters one space: "MAN A26" finds `MAN_A26_3D.bus`.
pub(crate) fn normalize_source_query(text: &str) -> String {
    let mut normalized = String::new();
    for c in text.chars() {
        if c.is_alphanumeric() {
            normalized.extend(c.to_lowercase());
        } else if !normalized.ends_with(' ') {
            normalized.push(' ');
        }
    }
    normalized.trim().to_string()
}

/// The bus folders and files that match, those that start so first.
pub(crate) fn source_suggestions(paths: &[String], query: &str) -> Vec<String> {
    let query = normalize_source_query(query);
    if query.is_empty() {
        return Vec::new();
    }
    let mut matches: Vec<(bool, String, String)> = paths.iter().filter_map(|path| {
        let normalized = normalize_source_query(path);
        normalized.contains(&query).then(|| (!normalized.starts_with(&query), normalized, path.clone()))
    }).collect();
    matches.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    matches.into_iter().map(|(_, _, path)| path).collect()
}

/// The bus file of a source (`Bus script: Pack/Bus.bus (Maker Type)`).
fn source_path(source: &str) -> String {
    let source = source.strip_prefix("Bus script: ").unwrap_or(source);
    let source = source.split_once(": ").map(|(_, path)| path).unwrap_or(source);
    source.split_once(" (").map(|(path, _)| path).unwrap_or(source).to_string()
}

/// The bus files among an action's sources, each once.
fn bus_source_paths(sources: &[String]) -> Vec<String> {
    let mut paths = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for source in sources.iter().filter_map(|s| s.strip_prefix("Bus script: ")) {
        let path = source_path(source);
        if seen.insert(path.to_ascii_lowercase()) {
            paths.push(path);
        }
    }
    paths
}

pub(crate) fn bus_usage_label(bus_count: usize, total_bus_count: usize) -> String {
    format!("Used by {bus_count}/{total_bus_count} buses")
}

/// The last scan of the buses' scripts, kept on this computer (the data folder's
/// `cache/key-actions.json`) so the picker is full at once on the next start: the actions
/// found, the bus files that use them and how many were read, for one OMSI folder. Nothing
/// leaves the computer; a new scan replaces it every time the picker is first opened.
#[derive(serde::Deserialize, serde::Serialize)]
struct ScriptActionCache {
    version: u32,
    root: String,
    actions: HashMap<String, Vec<String>>,
    source_paths: Vec<String>,
    total_buses: usize,
}

fn script_action_cache_path() -> std::path::PathBuf {
    core::data_dir().join("cache").join("key-actions.json")
}

fn load_script_action_cache(path: &std::path::Path, root: &str) -> Result<Option<ScriptActionCache>, String> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.to_string()),
    };
    let cache: ScriptActionCache = serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    Ok((cache.version == 1 && cache.root == root).then_some(cache))
}

fn save_script_action_cache(path: &std::path::Path, cache: &ScriptActionCache) -> Result<(), String> {
    let parent = path.parent().ok_or_else(|| "cache path has no parent directory".to_string())?;
    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let bytes = serde_json::to_vec(cache).map_err(|error| error.to_string())?;
    std::fs::write(path, bytes).map_err(|error| error.to_string())
}

/// The picker's actions: the cache read once, the scan started once per session (in the
/// background) and what it sent taken in, the list rebuilt when it changed.
pub(crate) fn ensure_key_action_catalog(l: &mut Launcher) {
    let root = std::path::PathBuf::from(&l.state.config.root);
    if l.pages.kb_script_actions_root.as_ref() != Some(&root) {
        l.pages.kb_script_actions = None;
        l.pages.kb_script_actions_rx = None;
        l.pages.kb_script_cache_loaded = false;
        l.pages.kb_script_scan_actions = None;
        l.pages.kb_script_scan_paths.clear();
        l.pages.kb_script_actions_root = Some(root);
        l.pages.kb_source_paths.clear();
        l.pages.kb_source_path_set.clear();
        l.pages.kb_source_suggestions = None;
        l.pages.kb_script_scan = (0, 0, String::new());
        l.pages.kb_script_total_buses = 0;
        l.pages.kb_script_scan_complete = false;
        l.pages.kb_action_options = None;
        l.pages.kb_filtered_options = None;
        l.pages.controller_action_choices = None;
    }
    if !l.pages.kb_script_cache_loaded {
        l.pages.kb_script_cache_loaded = true;
        match load_script_action_cache(&script_action_cache_path(), &l.state.config.root) {
            Ok(Some(cache)) => {
                l.pages.kb_script_actions = Some(cache.actions);
                l.pages.kb_source_paths = cache.source_paths;
                l.pages.kb_source_path_set = l.pages.kb_source_paths.iter().map(|path| path.to_lowercase()).collect();
                l.pages.kb_script_scan = (cache.total_buses, cache.total_buses, String::new());
                l.pages.kb_script_total_buses = cache.total_buses;
                l.pages.kb_source_suggestions = None;
                l.pages.kb_action_options = None;
                l.pages.kb_filtered_options = None;
                log::info!("key action catalog: loaded {} actions from cache", l.pages.kb_script_actions.as_ref().map_or(0, |actions| actions.len()));
            }
            Ok(None) => {}
            Err(error) => log::warn!("key action catalog cache could not be loaded: {error}"),
        }
    }
    if l.pages.kb_script_actions_rx.is_none() && !l.pages.kb_script_scan_complete {
        let _ = core::content_dir();
        // (unbounded: the scan runs to its end, and is cached, also while the picker is shut
        // and nobody reads it)
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            crate::describe::ControlNames::scan_script_actions(|update| { let _ = tx.send(update); });
        });
        l.pages.kb_script_scan_actions = Some(HashMap::new());
        l.pages.kb_script_scan_paths.clear();
        l.pages.kb_source_path_set.clear();
        l.pages.kb_script_scan = (0, 0, String::new());
        l.pages.kb_script_actions_rx = Some(rx);
    }
    let scan_was_complete = l.pages.kb_script_scan_complete;
    if let Some(rx) = l.pages.kb_script_actions_rx.as_ref() {
        loop {
            let update = match rx.try_recv() {
                Ok(update) => update,
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    log::warn!("key action scan stopped before it completed");
                    l.pages.kb_script_scan_complete = true;
                    break;
                }
            };
            l.pages.kb_script_scan = (update.done, update.total, update.current);
            l.pages.kb_script_total_buses = update.total;
            for (_, source) in &update.discovered {
                let path = source_path(source);
                if l.pages.kb_source_path_set.insert(path.to_lowercase()) {
                    l.pages.kb_script_scan_paths.push(path);
                }
            }
            if let Some(actions) = l.pages.kb_script_scan_actions.as_mut() {
                for (action, source) in update.discovered {
                    let sources = actions.entry(action.to_ascii_lowercase()).or_default();
                    if !sources.iter().any(|existing| existing.eq_ignore_ascii_case(&source)) {
                        sources.push(source);
                    }
                }
            }
            if update.complete {
                l.pages.kb_script_scan_complete = true;
                let actions = l.pages.kb_script_scan_actions.take().unwrap_or_default();
                let source_paths = std::mem::take(&mut l.pages.kb_script_scan_paths);
                l.pages.kb_source_path_set = source_paths.iter().map(|path| path.to_lowercase()).collect();
                l.pages.kb_source_paths = source_paths;
                l.pages.kb_source_paths.sort_by_key(|path| normalize_source_query(path));
                let cache = ScriptActionCache {
                    version: 1,
                    root: l.state.config.root.clone(),
                    actions: actions.clone(),
                    source_paths: l.pages.kb_source_paths.clone(),
                    total_buses: update.total,
                };
                if let Err(error) = save_script_action_cache(&script_action_cache_path(), &cache) {
                    log::warn!("key action catalog cache could not be saved: {error}");
                }
                l.pages.kb_script_actions = Some(actions);
                l.pages.kb_source_suggestions = None;
                // (the scan's thread is done: its channel shut is no failure)
                break;
            }
        }
    }
    if l.pages.kb_script_scan_complete {
        l.pages.kb_script_actions_rx = None;
    }
    if l.pages.kb_script_scan_complete && !scan_was_complete {
        l.pages.kb_action_options = None;
        l.pages.kb_filtered_options = None;
        l.pages.controller_action_choices = None;
    }
    if l.pages.kb_action_options.is_none() {
        let empty = HashMap::new();
        let scripts = l.pages.kb_script_actions.as_ref().unwrap_or(&empty);
        l.pages.kb_action_options = Some(action_options(control_names(l), scripts, &l.state.keybindings));
    }
}



#[cfg(test)]
mod keybind_picker_tests {
    use super::*;

    #[test]
    fn empty_search_shows_the_catalog_and_configured_custom_actions() {
        let names = crate::describe::ControlNames::from_table("ENG", &[
            ("door", "Front door"),
            ("horn", "Horn"),
            ("ivu_ticket_cancel", "IVU: Cancel ticket"),
        ]);
        let bindings = json!({
            "vehicles": [{ "action": "door" }, { "action": "mod_custom_action" }],
            "game": [{ "action": "sim_pause" }],
        });
        let script_sources = HashMap::from([
            ("mod_custom_action".into(), vec![
                "VehiclePack/Vehicle.bus".into(),
                "IVUPack/IVU.bus".into(),
                "AnotherPack/Another.bus".into(),
                "ThirdPack/Third.bus".into(),
                "FourthPack/Fourth.bus".into(),
            ]),
        ]);

        let all = action_options(&names, &script_sources, &bindings);
        let actions: Vec<&str> = all.iter().map(|option| option.action.as_str()).collect();
        assert_eq!(actions.len(), 4);
        assert!(actions.contains(&"door"));
        assert!(actions.contains(&"horn"));
        assert!(actions.contains(&"mod_custom_action"));
        let mod_action = all.iter().find(|option| option.action == "mod_custom_action").unwrap();
        assert!(mod_action.sources.contains(&"Configured binding".into()));
        assert_eq!(mod_action.bus_paths.len(), 5);

        let matches = filter_action_options(&all, "front door", "");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].action, "door");

        let matches = filter_action_options(&all, "vehiclepack", "");
        assert!(matches.is_empty());

        let matches = filter_action_options(&all, "ivu", "");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].action, "ivu_ticket_cancel");

        let matches = filter_action_options(&all, "", "ivupack");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].action, "mod_custom_action");
    }

    #[test]
    fn empty_query_keeps_catalog_entries_past_twelve() {
        let names = crate::describe::ControlNames::from_table("ENG", &[]);
        let script_actions: HashMap<String, Vec<String>> = (0..40)
            .map(|i| (format!("vehicle_action_{i:02}"), Vec::new()))
            .collect();
        let options = action_options(&names, &script_actions, &json!({}));
        assert_eq!(filter_action_options(&options, "", "").len(), 40);
    }

    #[test]
    fn bus_usage_label_shows_the_action_count_out_of_the_scan_total() {
        assert_eq!(bus_usage_label(5, 25), "Used by 5/25 buses");
        assert_eq!(bus_usage_label(1, 25), "Used by 1/25 buses");
    }

    #[test]
    fn script_action_cache_round_trips_and_is_scoped_to_the_install_root() {
        let dir = std::env::temp_dir().join(format!(
            "openomsi_action_cache_{}_{}",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos(),
        ));
        let path = dir.join("key-actions.json");
        let cache = ScriptActionCache {
            version: 1,
            root: "C:\\OMSI 2".into(),
            actions: HashMap::from([(
                "cruise_control".into(),
                vec!["BusPack/Bus.bus (Example Bus)".into()],
            )]),
            source_paths: vec!["BusPack/Bus.bus".into()],
            total_buses: 12,
        };

        save_script_action_cache(&path, &cache).unwrap();
        let loaded = load_script_action_cache(&path, "C:\\OMSI 2").unwrap().unwrap();
        assert_eq!(loaded.actions, cache.actions);
        assert_eq!(loaded.source_paths, cache.source_paths);
        assert_eq!(loaded.total_buses, cache.total_buses);
        assert!(load_script_action_cache(&path, "D:\\OMSI 2").unwrap().is_none());

        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn source_search_matches_folder_prefix_and_spaced_bus_filename() {
        let names = crate::describe::ControlNames::from_table("ENG", &[]);
        let script_actions = HashMap::from([
            ("neoman_special_control".into(), vec!["NEOMAN_Overhaul_v3/Vehicle.bus".into()]),
            ("man_a26_cruise".into(), vec!["MAN_A26_3D.bus".into()]),
        ]);
        let options = action_options(&names, &script_actions, &json!({}));

        let neo = filter_action_options(&options, "", "NEO");
        assert_eq!(neo.len(), 1);
        assert_eq!(neo[0].action, "neoman_special_control");

        let a26 = filter_action_options(&options, "", "MAN A26");
        assert_eq!(a26.len(), 1);
        assert_eq!(a26[0].action, "man_a26_cruise");
    }

    #[test]
    fn source_suggestions_match_folder_and_filename_fragments() {
        let paths = [
            "NEOMAN_Overhaul_v3/Vehicle.bus".to_string(),
            "MAN_A26_3D.bus".to_string(),
            "OtherPack/Other.bus".to_string(),
        ];
        assert_eq!(source_suggestions(&paths, "NEO"), ["NEOMAN_Overhaul_v3/Vehicle.bus"]);
        assert_eq!(source_suggestions(&paths, "MAN A26"), ["MAN_A26_3D.bus"]);
    }

    /// A controller's button can be given the keyboard file's vehicle actions (a bus's own
    /// trigger the picker added among them), the H-pattern gates and the game's own actions -
    /// not the game section's keys, which a button would send to the bus.
    #[test]
    fn controller_choices_include_keyboard_catalog_actions() {
        let names = crate::describe::ControlNames::from_table("ENG", &[("door", "Front door")]);
        let bindings = json!({
            "vehicles": [{ "action": "custom_cruise_control" }, { "action": "door" }],
            "game": [{ "action": "chat_open" }],
        });
        let (actions, labels) = controller_action_choices(&names, &bindings);
        assert_eq!(actions.first().map(String::as_str), Some("<none>"));
        let custom = actions.iter().position(|action| action == "custom_cruise_control").unwrap();
        assert!(labels[custom].contains("Custom cruise control"));
        let door = actions.iter().position(|action| action == "door").unwrap();
        assert_eq!(labels[door], action_text(&names, "door"));
        assert!(actions.contains(&"kw_s_1_fest".to_string()));
        assert!(actions.contains(&"doors_all".to_string()));
        assert!(!actions.contains(&"chat_open".to_string()));
        // <none>, the two the file has, the eleven H-pattern gates and the game's own
        assert_eq!(actions.len(), 1 + 2 + 11 + PAD_GAME_ACTIONS.len());
    }

}
