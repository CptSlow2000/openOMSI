//! Small parts the pages are built of: cards, headings, labelled rows, a status dot.

use egui_retained::widgets::{Text, TextAlign};
use egui_retained::{Color32, Element, MeasureCx, NodeId, PaintCx, Ui, Vec2, taffy};

pub fn lpa(v: f32) -> taffy::LengthPercentageAuto {
    taffy::LengthPercentageAuto::length(v)
}

pub fn lp(v: f32) -> taffy::LengthPercentage {
    taffy::LengthPercentage::length(v)
}

pub fn len(v: f32) -> taffy::Dimension {
    taffy::Dimension::length(v)
}

/// A translated text.
pub fn tr(s: &str) -> String {
    omsi_ui::tr(s).into_owned()
}

/// A card: a rounded box of the page's parts. Returns the card.
pub fn card(ui: &mut Ui, parent: NodeId) -> NodeId {
    let c = ui.column(parent);
    ui.add_class(c, "card");
    c
}

/// A card with a heading over its content.
pub fn titled_card(ui: &mut Ui, parent: NodeId, title: &str) -> NodeId {
    let c = card(ui, parent);
    let h = ui.text(c, tr(title));
    ui.add_class(h, "heading");
    c
}

/// A page's title and the line under it saying what it is for.
pub fn page_title(ui: &mut Ui, parent: NodeId, title: &str, sub: &str) -> NodeId {
    let head = ui.column(parent);
    ui.style(head, |s| {
        s.gap = taffy::Size { width: lp(0.0), height: lp(3.0) };
        s.flex_shrink = 0.0;
    });
    let t = ui.text(head, tr(title));
    ui.add_class(t, "title");
    if !sub.is_empty() {
        let s = ui.add(head, Text::wrapped(tr(sub)));
        ui.add_class(s, "subtitle");
    }
    head
}

/// A row: `label` on the left (`width` points), and what the caller adds right of it.
pub fn labelled(ui: &mut Ui, parent: NodeId, label: &str, width: f32) -> NodeId {
    let r = ui.row(parent);
    ui.style(r, |s| s.gap = taffy::Size { width: lp(10.0), height: lp(0.0) });
    let l = ui.add(r, Text::new(tr(label)));
    ui.add_class(l, "dim");
    ui.style(l, |s| {
        s.size.width = len(width);
        s.flex_shrink = 0.0;
    });
    r
}

/// Text of a class.
pub fn text(ui: &mut Ui, parent: NodeId, t: &str, class: &str) -> NodeId {
    let n = ui.text(parent, t.to_owned());
    if !class.is_empty() {
        ui.add_class(n, class);
    }
    n
}

/// Wrapped text of a class.
pub fn para(ui: &mut Ui, parent: NodeId, t: &str, class: &str) -> NodeId {
    let n = ui.add(parent, Text::wrapped(t.to_owned()));
    if !class.is_empty() {
        ui.add_class(n, class);
    }
    n
}

/// Right-aligned text that takes the rest of a row.
pub fn right_text(ui: &mut Ui, parent: NodeId, t: &str, class: &str) -> NodeId {
    let n = ui.add(parent, Text::new(t.to_owned()).align(TextAlign::Right));
    ui.style(n, |s| s.flex_grow = 1.0);
    if !class.is_empty() {
        ui.add_class(n, class);
    }
    n
}

/// Let `node` grow to fill its row or column.
pub fn grow(ui: &mut Ui, node: NodeId) {
    ui.style(node, |s| {
        s.flex_grow = 1.0;
        s.flex_basis = taffy::Dimension::length(0.0);
        s.min_size = taffy::Size { width: taffy::Dimension::length(0.0), height: taffy::Dimension::length(0.0) };
    });
}

/// A gap between children.
pub fn gap(ui: &mut Ui, node: NodeId, g: f32) {
    ui.style(node, |s| s.gap = taffy::Size { width: lp(g), height: lp(g) });
}

/// A coloured dot (the status bar's state).
pub struct Dot {
    pub color: Color32,
}

impl Dot {
    pub fn new(color: Color32) -> Dot {
        Dot { color }
    }
}

impl Element for Dot {
    fn measure(&mut self, _cx: &mut MeasureCx<'_>, _k: [Option<f32>; 2], _a: [Option<f32>; 2]) -> Vec2 {
        Vec2::splat(10.0)
    }
    fn paint(&mut self, cx: &mut PaintCx<'_>) {
        cx.painter.circle_filled(cx.content.center(), 4.0, self.color);
    }
    fn hit_test(&self) -> bool {
        false
    }
}
