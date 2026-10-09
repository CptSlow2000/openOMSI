//! What is written over the map: the entry point under the mouse (or the one chosen), the
//! first trip's stops with their times where there is room, how many roads, stops and entry
//! points the map has, and a turning arc while it is read.

use super::super::theme::ACCENT;
use crate::launcher::Launcher;
use egui_retained::epaint::{FontFamily, FontId, Stroke};
use egui_retained::{Color32, Element, PaintCx, Pos2, Rect, layout_text, pos2, vec2};

#[derive(Clone, PartialEq, Debug)]
pub struct Label {
    pub at: Pos2,
    pub text: String,
    /// A stop's time, before its name.
    pub time: Option<String>,
    /// The entry point's label (filled with the accent).
    pub entry: bool,
}

#[derive(Default, Clone, PartialEq)]
pub struct MapLabels {
    pub labels: Vec<Label>,
    pub legend: Option<String>,
    pub busy: bool,
}

impl MapLabels {
    /// What to write over the map this frame.
    pub fn of(l: &Launcher) -> MapLabels {
        let mv = &l.mapview;
        let mut labels = Vec::new();
        if let Some(i) = mv.hovered().or_else(|| mv.shown_of(l.state.choice.entry)) {
            if let (Some(name), Some(at)) = (mv.entry_name(i), mv.entry_at(i)) {
                let name = if name.chars().count() > 32 { name.chars().take(31).collect::<String>() + "…" } else { name.to_string() };
                labels.push(Label { at: pos2(at.x, at.y), text: name, time: None, entry: true });
            }
        }
        let trip = l.state.tour().and_then(|t| t.trips.get(l.state.first_trip().unwrap_or(0)));
        for (place, k) in mv.stops_placed() {
            let Some(st) = trip.and_then(|t| t.stops.get(*k)) else { continue };
            let at = mv.project(*place);
            labels.push(Label { at: pos2(at.x, at.y), text: st.name.clone(), time: Some(crate::launcher::state::hhmm(st.arr)), entry: false });
        }
        let legend = mv.counts().map(|(roads, stops, entries)| format!("{roads} {}  ·  {stops} {}  ·  {entries} {}", omsi_ui::tr("roads"), omsi_ui::tr("stops"), omsi_ui::tr("entry points")));
        MapLabels { labels, legend, busy: mv.busy() }
    }
}

fn font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}

impl Element for MapLabels {
    fn paint(&mut self, cx: &mut PaintCx<'_>) {
        let map = cx.rect;
        let hits = |a: &Rect, b: &Rect| a.intersects(*b);
        let inside = |r: &Rect| map.shrink(4.0).contains_rect(*r);
        let mut taken: Vec<Rect> = Vec::new();
        // the legend at the foot
        if let Some(t) = &self.legend {
            let g = layout_text(cx.fonts, cx.ppp, t, font(11.0), Color32::from_gray(205), None);
            let bar = Rect::from_min_size(pos2(map.left() + 12.0, map.bottom() - 32.0), vec2((g.size().x + 20.0).min(map.width() - 24.0), 22.0));
            cx.painter.rect_filled(bar, 5.0, Color32::from_black_alpha(153));
            cx.painter.galley(pos2(bar.left() + 10.0, bar.center().y - g.size().y * 0.5), g, Color32::WHITE);
            taken.push(bar);
        }
        for lb in &self.labels {
            if !map.contains(lb.at) {
                continue;
            }
            if lb.entry {
                // above the point, to its right (or its left at the edge)
                let g = layout_text(cx.fonts, cx.ppp, &lb.text, font(11.5), Color32::from_rgb(18, 14, 8), None);
                let w = g.size().x + 14.0;
                let right = Rect::from_min_size(lb.at + vec2(12.0, -28.0), vec2(w, 20.0));
                let r = if inside(&right) { right } else { Rect::from_min_size(lb.at + vec2(-12.0 - w, -28.0), vec2(w, 20.0)) };
                if inside(&r) {
                    cx.painter.rect_filled(r, 4.0, ACCENT);
                    cx.painter.galley(pos2(r.left() + 7.0, r.center().y - g.size().y * 0.5), g, Color32::WHITE);
                    taken.push(r);
                }
                continue;
            }
            // a stop: its time and name beside it, where nothing is yet
            let time = lb.time.clone().unwrap_or_default();
            let gt = layout_text(cx.fonts, cx.ppp, &time, font(11.0), ACCENT, None);
            let gn = layout_text(cx.fonts, cx.ppp, &lb.text, font(11.0), Color32::from_gray(215), None);
            let w = gt.size().x + gn.size().x + 22.0;
            let right = Rect::from_min_size(lb.at + vec2(9.0, -9.0), vec2(w, 18.0));
            let left = Rect::from_min_size(lb.at + vec2(-9.0 - w, -9.0), vec2(w, 18.0));
            let Some(r) = [right, left].into_iter().find(|r| inside(r) && !taken.iter().any(|t| hits(t, r))) else { continue };
            cx.painter.rect_filled(r, 4.0, Color32::from_rgba_unmultiplied(10, 10, 10, 214));
            let tw = gt.size().x;
            cx.painter.galley(pos2(r.left() + 6.0, r.center().y - gt.size().y * 0.5), gt, Color32::WHITE);
            cx.painter.galley(pos2(r.left() + 12.0 + tw, r.center().y - gn.size().y * 0.5), gn, Color32::WHITE);
            taken.push(r);
        }
        // the map being read: an arc turning out of the way of the panels
        if self.busy {
            let c = pos2(map.right() - 26.0, map.bottom() - 26.0);
            let a = (cx.time * 5.0) as f32;
            cx.painter.arc(c, 7.0, a, a + 4.2, Stroke::new(2.0, Color32::from_gray(200)));
            cx.repaint_after(0.05);
        }
    }
    fn hit_test(&self) -> bool {
        false
    }
}

/// The overlay's box: over the whole of the map it is put in.
pub fn layout(s: &mut egui_retained::taffy::Style) {
    use egui_retained::taffy;
    s.position = taffy::Position::Absolute;
    s.inset = taffy::Rect { left: taffy::LengthPercentageAuto::length(0.0), right: taffy::LengthPercentageAuto::length(0.0), top: taffy::LengthPercentageAuto::length(0.0), bottom: taffy::LengthPercentageAuto::length(0.0) };
}
