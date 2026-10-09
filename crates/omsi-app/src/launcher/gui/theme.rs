//! The launcher's look in the Development Tools' design language: a near-black page, a
//! sidebar a shade lighter, cards with rounded corners, flat controls, the openOMSI orange for
//! what is chosen and what goes on. A light theme of the same tokens.

use egui_retained::epaint::{FontFamily, Stroke};
use egui_retained::{Color32, Cursor, Rule, Theme, Visual, taffy};

pub const ACCENT: Color32 = Color32::from_rgb(0xF4, 0x7F, 0x30);
pub const OK: Color32 = Color32::from_rgb(0x3A, 0xA6, 0x55);
pub const WARN: Color32 = Color32::from_rgb(0xE8, 0xB0, 0x3A);
pub const DANGER: Color32 = Color32::from_rgb(0xE2, 0x4B, 0x4B);

/// The colours of a theme.
struct Palette {
    page: Color32,
    sidebar: Color32,
    card: Color32,
    control: Color32,
    control_hover: Color32,
    control_press: Color32,
    field: Color32,
    popup: Color32,
    line: Color32,
    text: Color32,
    text_strong: Color32,
    text_dim: Color32,
    text_faint: Color32,
    nav_hover: Color32,
    selected: Color32,
    row_selected: Color32,
}

const DARK: Palette = Palette {
    page: Color32::from_rgb(0x13, 0x15, 0x19),
    sidebar: Color32::from_rgb(0x17, 0x19, 0x1E),
    card: Color32::from_rgb(0x1B, 0x1E, 0x25),
    control: Color32::from_rgb(0x2A, 0x2D, 0x35),
    control_hover: Color32::from_rgb(0x34, 0x38, 0x41),
    control_press: Color32::from_rgb(0x3E, 0x42, 0x4C),
    field: Color32::from_rgb(0x0F, 0x11, 0x15),
    popup: Color32::from_rgb(0x20, 0x23, 0x2B),
    line: Color32::from_rgba_premultiplied(18, 18, 18, 18),
    text: Color32::from_rgb(0xC6, 0xC8, 0xCC),
    text_strong: Color32::from_rgb(0xEC, 0xED, 0xEF),
    text_dim: Color32::from_rgb(0x8C, 0x8F, 0x96),
    text_faint: Color32::from_rgb(0x66, 0x69, 0x70),
    nav_hover: Color32::from_rgb(0x22, 0x25, 0x2C),
    selected: Color32::from_rgb(0x70, 0x3D, 0x21),
    row_selected: Color32::from_rgb(0x2A, 0x22, 0x1D),
};

const LIGHT: Palette = Palette {
    page: Color32::from_rgb(0xF4, 0xF5, 0xF7),
    sidebar: Color32::from_rgb(0xEA, 0xEC, 0xEF),
    card: Color32::from_rgb(0xFF, 0xFF, 0xFF),
    control: Color32::from_rgb(0xE6, 0xE8, 0xEC),
    control_hover: Color32::from_rgb(0xDC, 0xDF, 0xE4),
    control_press: Color32::from_rgb(0xD0, 0xD4, 0xDA),
    field: Color32::from_rgb(0xF7, 0xF8, 0xFA),
    popup: Color32::from_rgb(0xFF, 0xFF, 0xFF),
    line: Color32::from_rgba_premultiplied(0, 0, 0, 22),
    text: Color32::from_rgb(0x2B, 0x2E, 0x34),
    text_strong: Color32::from_rgb(0x12, 0x14, 0x18),
    text_dim: Color32::from_rgb(0x5E, 0x63, 0x6B),
    text_faint: Color32::from_rgb(0x8A, 0x8F, 0x97),
    nav_hover: Color32::from_rgb(0xDE, 0xE1, 0xE6),
    selected: Color32::from_rgb(0xFC, 0xDD, 0xC8),
    row_selected: Color32::from_rgb(0xFE, 0xF1, 0xE8),
};

fn lp(v: f32) -> taffy::LengthPercentage {
    taffy::LengthPercentage::length(v)
}

/// The theme: dark, or light.
pub fn theme(dark: bool) -> Theme {
    let p = if dark { &DARK } else { &LIGHT };
    let mut t = Theme::default();
    t.defaults.color = p.text;
    t.defaults.font_size = 14.0;
    t.defaults.font = FontFamily::Proportional;
    t.selection = ACCENT.gamma_multiply(0.42);
    t.caret = ACCENT;
    let v = Visual::new;
    let on_accent = Color32::WHITE;
    t.rule("accent", Rule::visual(v().background(ACCENT)));
    t.rule("field", Rule::visual(v().background(if dark { p.control } else { p.control_press })));
    t.rule("page", Rule::visual(v().background(p.page)));
    t.rule("header", Rule::visual(v().background(p.page)).with_layout(|s| {
        s.padding = taffy::Rect { left: lp(16.0), right: lp(12.0), top: lp(10.0), bottom: lp(10.0) };
        s.gap = taffy::Size { width: lp(8.0), height: lp(0.0) };
        s.flex_shrink = 0.0;
    }));
    t.rule("sidebar", Rule::visual(v().background(p.sidebar)).with_layout(|s| {
        s.size.width = taffy::Dimension::length(190.0);
        s.flex_shrink = 0.0;
        s.padding = taffy::Rect { left: lp(8.0), right: lp(8.0), top: lp(10.0), bottom: lp(10.0) };
        s.gap = taffy::Size { width: lp(0.0), height: lp(2.0) };
    }));
    t.rule("content", Rule::default().with_layout(|s| {
        s.flex_grow = 1.0;
        s.flex_basis = taffy::Dimension::length(0.0);
        s.padding = taffy::Rect { left: lp(22.0), right: lp(22.0), top: lp(18.0), bottom: lp(18.0) };
        s.gap = taffy::Size { width: lp(12.0), height: lp(12.0) };
    }));
    t.rule("card", Rule::visual(v().background(p.card).radius(10.0_f32)).with_layout(|s| {
        s.padding = taffy::Rect::length(14.0_f32);
        s.gap = taffy::Size { width: lp(8.0), height: lp(8.0) };
        s.flex_shrink = 0.0;
    }));
    // text
    t.rule("title", Rule::visual(v().font_size(23.0_f32).color(p.text_strong)));
    t.rule("subtitle", Rule::visual(v().font_size(13.0_f32).color(p.text_dim)));
    t.rule("heading", Rule::visual(v().font_size(14.5_f32).color(p.text_strong)));
    t.rule("strong", Rule::visual(v().color(p.text_strong)));
    t.rule("dim", Rule::visual(v().color(p.text_dim).font_size(13.0_f32)));
    t.rule("faint", Rule::visual(v().color(p.text_faint).font_size(12.0_f32)));
    t.rule("small", Rule::visual(v().font_size(12.0_f32)));
    t.rule("accent-text", Rule::visual(v().color(ACCENT)));
    t.rule("ok-text", Rule::visual(v().color(OK)));
    t.rule("warn-text", Rule::visual(v().color(WARN)));
    t.rule("danger-text", Rule::visual(v().color(DANGER)));
    t.rule("brand", Rule::visual(v().font_size(21.0_f32).color(ACCENT)));
    t.rule("app-name", Rule::visual(v().font_size(21.0_f32).color(p.text_strong)));
    // controls
    let button = v().background(p.control).color(p.text_strong).radius(6.0_f32).cursor(Cursor::Pointer).transition(0.1_f32)
        .on_hover(v().background(p.control_hover))
        .on_press(v().background(p.control_press))
        .on_focus(v().border(Stroke::new(1.0, ACCENT.gamma_multiply(0.6))))
        .on_disable(v().color(p.text_faint).cursor(Cursor::Default));
    t.rule("button", Rule::visual(button));
    t.rule("primary", Rule::visual(v().background(ACCENT).color(on_accent).radius(6.0_f32).cursor(Cursor::Pointer).transition(0.1_f32)
        .on_hover(v().background(Color32::from_rgb(0xFF, 0x92, 0x48)))
        .on_press(v().background(Color32::from_rgb(0xD9, 0x6C, 0x22)))
        .on_disable(v().background(p.control).color(p.text_faint).cursor(Cursor::Default))).with_layout(|s| {
        s.padding = taffy::Rect { left: lp(18.0), right: lp(18.0), top: lp(9.0), bottom: lp(9.0) };
    }));
    t.rule("danger", Rule::visual(v().background(DANGER.gamma_multiply(0.22)).color(DANGER).radius(6.0_f32).cursor(Cursor::Pointer)
        .on_hover(v().background(DANGER.gamma_multiply(0.32)))));
    t.rule("ghost", Rule::visual(v().color(p.text_dim).radius(6.0_f32).cursor(Cursor::Pointer).on_hover(v().background(p.nav_hover).color(p.text_strong))));
    t.rule("link", Rule::visual(v().color(ACCENT).cursor(Cursor::Pointer).on_hover(v().color(Color32::from_rgb(0xFF, 0xA0, 0x60)))).with_layout(|s| {
        s.padding = taffy::Rect::length(0.0_f32);
    }));
    t.rule("nav", Rule::visual(v().radius(6.0_f32).color(p.text_dim).font_size(15.0_f32).cursor(Cursor::Pointer).transition(0.1_f32)
        .on_hover(v().background(p.nav_hover).color(p.text_strong))
        .on_select(v().background(p.selected).color(ACCENT))).with_layout(|s| {
        s.size.height = taffy::Dimension::length(34.0);
    }));
    t.rule("tab", Rule::visual(v().radius(6.0_f32).color(p.text_dim).cursor(Cursor::Pointer)
        .on_hover(v().color(p.text_strong).background(p.nav_hover))
        .on_select(v().background(p.selected).color(ACCENT))));
    // (a chosen row keeps its texts' colours, grey and orange among them: a faint tint and an
    // orange edge say it is chosen, not a fill they would not read on)
    t.rule("list-row", Rule::visual(v().radius(8.0_f32).cursor(Cursor::Pointer).transition(0.08_f32).border(Stroke::new(1.0, Color32::TRANSPARENT))
        .on_hover(v().background(p.nav_hover))
        .on_select(v().background(p.row_selected).border(Stroke::new(1.0, ACCENT.gamma_multiply(0.7)))))
        .with_layout(|s| {
            s.padding = taffy::Rect { left: lp(10.0), right: lp(10.0), top: lp(7.0), bottom: lp(7.0) };
            s.flex_shrink = 0.0;
        }));
    t.rule("list", Rule::visual(v().background(p.field).radius(8.0_f32).border(Stroke::new(1.0, p.line))).with_layout(|s| {
        s.padding = taffy::Rect::length(4.0_f32);
        s.gap = taffy::Size { width: lp(2.0), height: lp(2.0) };
    }));
    t.rule("text-input", Rule::visual(v().background(p.field).color(p.text_strong).radius(6.0_f32).cursor(Cursor::Text)
        .border(Stroke::new(1.0, p.line))
        .on_hover(v().border(Stroke::new(1.0, p.text_faint)))
        .on_focus(v().border(Stroke::new(1.0, ACCENT)))));
    t.rule("select", Rule::visual(v().background(p.control).color(p.text_strong).radius(6.0_f32).cursor(Cursor::Pointer).transition(0.1_f32)
        .on_hover(v().background(p.control_hover))
        .on_focus(v().border(Stroke::new(1.0, ACCENT.gamma_multiply(0.6))))));
    t.rule("menu", Rule::visual(v().background(p.popup).radius(8.0_f32).border(Stroke::new(1.0, p.line))));
    t.rule("menu-item", Rule::visual(v().radius(5.0_f32).color(p.text).cursor(Cursor::Pointer)
        .on_hover(v().background(p.control_hover))
        .on_select(v().color(ACCENT))).with_layout(|s| {
        s.padding = taffy::Rect { left: lp(10.0), right: lp(10.0), top: lp(6.0), bottom: lp(6.0) };
    }));
    t.rule("checkbox", Rule::visual(v().cursor(Cursor::Pointer).color(p.text)));
    t.rule("switch", Rule::visual(v().cursor(Cursor::Pointer).color(p.text)));
    t.rule("slider", Rule::visual(v().cursor(Cursor::Pointer)));
    t.rule("separator", Rule::visual(v().background(p.line)));
    t.rule("badge", Rule::visual(v().background(p.control).color(p.text_strong).radius(4.0_f32).font_size(12.0_f32)).with_layout(|s| {
        s.padding = taffy::Rect { left: lp(6.0), right: lp(6.0), top: lp(1.0), bottom: lp(1.0) };
    }));
    t.rule("badge-accent", Rule::visual(v().background(ACCENT).color(Color32::from_rgb(0x1A, 0x10, 0x08))));
    t.rule("icon", Rule::visual(v().color(p.text_dim)));
    t.rule("status", Rule::visual(v().background(p.page).color(p.text_dim).font_size(12.5_f32)).with_layout(|s| {
        s.padding = taffy::Rect { left: lp(14.0), right: lp(14.0), top: lp(6.0), bottom: lp(6.0) };
        s.gap = taffy::Size { width: lp(8.0), height: lp(0.0) };
        s.flex_shrink = 0.0;
    }));
    t.rule("tooltip", Rule::visual(v().background(p.popup).radius(6.0_f32).border(Stroke::new(1.0, p.line))));
    t.rule("stage", Rule::visual(v().background(p.field).radius(10.0_f32).border(Stroke::new(1.0, p.line))));
    t.rule("dialog", Rule::visual(v().background(p.card).radius(12.0_f32).border(Stroke::new(1.0, p.line))).with_layout(|s| {
        s.padding = taffy::Rect::length(20.0_f32);
        s.gap = taffy::Size { width: lp(10.0), height: lp(10.0) };
    }));
    t.rule("scrim", Rule::visual(v().background(Color32::from_black_alpha(150))));
    t
}

/// Roboto for every text; the Material Symbols for the icons (see `icon_mask`).
pub fn fonts() -> egui_retained::epaint::text::FontDefinitions {
    use egui_retained::epaint::text::{FontData, FontDefinitions};
    let mut defs = FontDefinitions::default();
    defs.font_data.insert("roboto".into(), std::sync::Arc::new(FontData::from_static(include_bytes!("../../../../../assets/fonts/Roboto-VariableFont_wdth,wght.ttf"))));
    if let Some(f) = defs.families.get_mut(&FontFamily::Proportional) {
        f.insert(0, "roboto".into());
        // (arrows, box drawing and the like, which neither Roboto nor Ubuntu has: "→")
        if !f.iter().any(|n| n == "Hack") {
            f.push("Hack".into());
        }
    }
    defs
}

/// An icon of the Material Symbols (and the game's own SVGs) as an alpha mask.
pub fn icon_mask(name: &str, px: u32) -> Option<Vec<u8>> {
    omsi_ui::icons::rasterize(name, px)
}
