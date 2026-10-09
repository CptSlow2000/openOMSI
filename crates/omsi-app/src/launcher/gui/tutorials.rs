//! The Tutorials page: OMSI 2's own lessons, each started into its situation.

use super::kit::{self, len, lp, tr};
use super::Msg as Top;
use crate::launcher::Launcher;
use egui_retained::widgets::Button;
use egui_retained::{NodeId, ScrollAxes, Ui, taffy};

#[derive(Clone, Debug)]
pub(in crate::launcher) enum Msg {
    Start(usize),
}

pub(in crate::launcher) struct TutorialsPage {
    pub root: NodeId,
}

impl TutorialsPage {
    pub fn build(ui: &mut Ui, host: NodeId) -> TutorialsPage {
        let root = ui.column(host);
        ui.add_class(root, "content");
        kit::grow(ui, root);
        ui.set_scroll(root, ScrollAxes { x: false, y: true });
        kit::page_title(ui, root, "Tutorials", "OMSI 2's own lessons: each opens its situation, with its pages beside the picture (Enter turns the page).");
        static LIST: std::sync::OnceLock<Vec<(usize, String, String)>> = std::sync::OnceLock::new();
        let list = LIST.get_or_init(omsi_launcher_lib::tutorials);
        if list.is_empty() {
            kit::para(ui, root, &tr("No tutorials were found in the OMSI 2 folder (Tutorials)."), "dim");
        }
        let grid = ui.row(root);
        ui.style(grid, |s| {
            s.flex_wrap = taffy::FlexWrap::Wrap;
            s.gap = taffy::Size { width: lp(14.0), height: lp(14.0) };
            s.align_items = Some(taffy::AlignItems::Stretch);
        });
        for (n, title, text) in list {
            let c = kit::card(ui, grid);
            ui.style(c, |s| {
                s.size.width = taffy::Dimension::percent(0.48);
                s.flex_grow = 1.0;
                s.min_size.width = len(320.0);
                s.max_size.height = len(300.0);
                s.overflow = taffy::Point { x: taffy::Overflow::Hidden, y: taffy::Overflow::Hidden };
            });
            let t = kit::text(ui, c, title, "heading");
            ui.visual(t, egui_retained::Visual::new().font_size(17.0_f32));
            // (the lesson's first lines; a long one is cut at the card's foot)
            let body = kit::para(ui, c, text, "dim");
            ui.style(body, |s| {
                s.flex_shrink = 1.0;
                s.min_size.height = len(0.0);
                s.overflow = taffy::Point { x: taffy::Overflow::Hidden, y: taffy::Overflow::Hidden };
            });
            let b = ui.add(c, Button::new(tr("Start the lesson")).icon("play_arrow").class("primary"));
            ui.style(b, |s| {
                s.align_self = Some(taffy::AlignSelf::FlexStart);
                s.margin.top = taffy::LengthPercentageAuto::auto();
                s.flex_shrink = 0.0;
            });
            ui.on_click(b, Top::Tutorials(Msg::Start(*n)));
        }
        TutorialsPage { root }
    }

    pub fn sync(&mut self, _ui: &mut Ui, _l: &Launcher) {}
}

pub(in crate::launcher) fn handle(l: &mut Launcher, m: Msg) {
    match m {
        Msg::Start(n) => l.state.launch_tutorial(n),
    }
}
