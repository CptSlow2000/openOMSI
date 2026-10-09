//! The picture a page shows across a box (the bus on its turntable, the map): the
//! application's texture, and the pointer over it kept for the launcher - dragged, the wheel,
//! pressed and let go - since the showroom and the map work it themselves.

use egui_retained::input::Button;
use egui_retained::widgets::Image;
use egui_retained::{Element, Event, EventCx, PaintCx, Pos2, Rect, Vec2};
use std::cell::RefCell;
use std::rc::Rc;

/// The pointer over a stage since the launcher last looked.
#[derive(Clone, Debug)]
pub struct StagePointer {
    pub at: Option<Pos2>,
    pub drag: Vec2,
    pub wheel: f32,
    pub pressed: bool,
    pub released: bool,
    pub down: bool,
    /// The stage's box this frame.
    pub rect: Rect,
}

impl Default for StagePointer {
    fn default() -> Self {
        StagePointer { at: None, drag: Vec2::ZERO, wheel: 0.0, pressed: false, released: false, down: false, rect: Rect::NOTHING }
    }
}

impl StagePointer {
    /// What happened, taken (the next frame starts afresh; `at` and `down` stay).
    pub fn take(&mut self) -> StagePointer {
        let t = self.clone();
        self.drag = Vec2::ZERO;
        self.wheel = 0.0;
        self.pressed = false;
        self.released = false;
        t
    }
}

pub struct Stage {
    pub image: Image,
    pub pointer: Rc<RefCell<StagePointer>>,
    /// The cursor's look over it.
    pub grab: bool,
}

impl Stage {
    pub fn new(placeholder: &str, grab: bool) -> (Stage, Rc<RefCell<StagePointer>>) {
        let mut image = Image::new(None, Vec2::ZERO);
        image.placeholder = placeholder.to_owned();
        let p = Rc::new(RefCell::new(StagePointer::default()));
        (Stage { image, pointer: p.clone(), grab }, p)
    }
}

impl Element for Stage {
    fn class(&self) -> &'static str {
        "stage"
    }
    fn paint(&mut self, cx: &mut PaintCx<'_>) {
        self.pointer.borrow_mut().rect = cx.rect;
        self.image.paint(cx);
    }
    fn event(&mut self, cx: &mut EventCx<'_>, event: &Event) -> bool {
        let mut p = self.pointer.borrow_mut();
        match event {
            Event::PointerMove { pos } => p.at = Some(*pos),
            Event::PointerLeave => {
                if !p.down {
                    p.at = None;
                }
            }
            Event::PointerDown { pos, button: Button::Primary | Button::Secondary } => {
                p.at = Some(*pos);
                p.pressed = true;
                p.down = true;
                cx.capture();
            }
            Event::PointerUp { pos, .. } => {
                p.at = Some(*pos);
                p.released = true;
                p.down = false;
            }
            Event::Drag { pos, delta, .. } => {
                p.at = Some(*pos);
                p.drag += *delta;
            }
            Event::Wheel { delta } => p.wheel += delta.y / 40.0,
            _ => return false,
        }
        cx.repaint();
        // (the pointer's own events are the stage's; the wheel too - the page does not scroll
        // under a map being zoomed)
        cx.target == cx.node || matches!(event, Event::Wheel { .. })
    }
}

impl Stage {
    pub fn set_texture(&mut self, t: Option<egui_retained::epaint::TextureId>, size: Vec2) {
        self.image.texture = t;
        self.image.size = size;
    }
}
