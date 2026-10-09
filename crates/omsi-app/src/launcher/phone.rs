//! The launcher made for a phone (or a tablet), not the desktop's pages squeezed onto it:
//! a tab bar at the foot (Play, Online, Mods, More), a Play screen with the bus large and
//! the duty as four big cards over one Start button, and every choice made on a sheet of
//! its own that fills the screen - a list of big rows, a search field, a way back.
//! The pages a phone needs less often (Settings, Controls, Profile, …) open from More,
//! full width, under a bar with a way back.

use super::Page;

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum Tab {
    #[default]
    Play,
    Online,
    Mods,
    More,
}

/// A choice made on a sheet of its own.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sheet {
    Map,
    Bus,
    Livery,
    /// HOF, fleet number and registration plate.
    Vehicle,
    Duty,
    Tour,
    Time,
    /// Spawn and service options that the desktop Drive page exposes.
    Start,
    /// The selected duty's roadbook and IBIS hint.
    Roadbook,
}

#[derive(Default)]
pub struct PhoneView {
    pub tab: Tab,
    pub sheet: Option<Sheet>,
    /// A page of More that is open.
    pub page: Option<Page>,
    pub filter: String,
}

