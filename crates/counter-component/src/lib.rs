#![allow(clippy::all)]

mod bindings;

use bindings::exports::vello::canvas::app::{self, Guest};
use bindings::vello::canvas::host;
use bindings::vello::canvas::math::Vec2 as HostVec2;
use std::cell::RefCell;

thread_local! {
    static STATE: RefCell<CounterApp> = RefCell::new(CounterApp::new());
}

fn with_state<R>(f: impl FnOnce(&mut CounterApp) -> R) -> R {
    STATE.with(|cell| f(&mut cell.borrow_mut()))
}

#[derive(Clone, Copy, Debug)]
struct Rect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

impl Rect {
    fn contains(&self, point: [f32; 2]) -> bool {
        point[0] >= self.x
            && point[0] <= self.x + self.w
            && point[1] >= self.y
            && point[1] <= self.y + self.h
    }

    fn center(&self) -> [f32; 2] {
        [self.x + self.w * 0.5, self.y + self.h * 0.5]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Button {
    Minus,
    Plus,
}

struct CounterApp {
    size: app::LogicalSize,
    count: i32,
    active_pointer: Option<(u64, Button)>,
    hover: Option<Button>,
    cursor: [f32; 2],
}

impl CounterApp {
    fn new() -> Self {
        Self {
            size: app::LogicalSize {
                width: 0.0,
                height: 0.0,
                scale_factor: 1.0,
            },
            count: 0,
            active_pointer: None,
            hover: None,
            cursor: [0.0, 0.0],
        }
    }

    fn request_redraw(&self) {
        host::request_frame();
    }

    fn layout(&self) -> Layout {
        Layout::from_size(self.size)
    }

    fn set_hover(&mut self, hover: Option<Button>) {
        if self.hover != hover {
            self.hover = hover;
            self.request_redraw();
        }
    }

    fn set_active(&mut self, pointer_id: u64, button: Button) {
        self.active_pointer = Some((pointer_id, button));
        self.request_redraw();
    }

    fn clear_active(&mut self) {
        if self.active_pointer.take().is_some() {
            self.request_redraw();
        }
    }

    fn adjust_count(&mut self, delta: i32) {
        let new = self.count.saturating_add(delta);
        if new != self.count {
            self.count = new;
            self.request_redraw();
        }
    }

    fn reset_count(&mut self) {
        if self.count != 0 {
            self.count = 0;
            self.request_redraw();
        }
    }

    fn button_at(&self, point: [f32; 2]) -> Option<Button> {
        let layout = self.layout();
        if layout.minus.contains(point) {
            Some(Button::Minus)
        } else if layout.plus.contains(point) {
            Some(Button::Plus)
        } else {
            None
        }
    }

    fn draw(&self) {
        let layout = self.layout();
        host::clear(host_color(0.09, 0.1, 0.12, 1.0));

        self.draw_panel(&layout);
        self.draw_buttons(&layout);
        self.draw_label(&layout);
        self.draw_hint(&layout);
    }

    fn draw_panel(&self, layout: &Layout) {
        let panel_size = layout.panel_size();
        host::fill_rect(
            to_vec2(layout.panel_origin()),
            to_vec2([panel_size[0], panel_size[1]]),
            host_color(0.12, 0.14, 0.18, 1.0),
        );
    }

    fn draw_buttons(&self, layout: &Layout) {
        self.draw_button(layout.minus, "-", Button::Minus);
        self.draw_button(layout.plus, "+", Button::Plus);
    }

    fn draw_button(&self, rect: Rect, label: &str, kind: Button) {
        let mut color = host_color(0.24, 0.28, 0.36, 1.0);
        if Some(kind) == self.hover {
            color = host_color(0.3, 0.36, 0.46, 1.0);
        }
        if self
            .active_pointer
            .as_ref()
            .map(|(_, active)| *active == kind)
            .unwrap_or(false)
        {
            color = host_color(0.32, 0.4, 0.52, 1.0);
        }

        host::fill_rect(to_vec2([rect.x, rect.y]), to_vec2([rect.w, rect.h]), color);

        let text_size = rect.h * 0.6;
        let center = rect.center();
        let text_origin = [center[0] - text_size * 0.25, center[1] + text_size * 0.35];
        host::draw_text(
            label,
            to_vec2(text_origin),
            text_size,
            host_color(0.95, 0.96, 0.98, 1.0),
        );
    }

    fn draw_label(&self, layout: &Layout) {
        let text = format!("{}", self.count);
        host::draw_text(
            &text,
            to_vec2(layout.count_label_origin()),
            layout.count_text_size,
            host_color(0.92, 0.94, 0.98, 1.0),
        );
    }

    fn draw_hint(&self, layout: &Layout) {
        let hint = "Use +/- keys or Space/Enter";
        host::draw_text(
            hint,
            to_vec2(layout.hint_origin),
            layout.count_text_size * 0.4,
            host_color(0.6, 0.68, 0.78, 1.0),
        );
    }
}

impl CounterApp {
    fn handle_init(&mut self, initial: app::LogicalSize) {
        self.size = initial;
        self.request_redraw();
    }

    fn handle_resize(&mut self, new: app::LogicalSize) {
        self.size = new;
        self.request_redraw();
    }

    fn handle_pointer_down(&mut self, evt: app::PointerEvent) {
        self.cursor = [evt.position.x, evt.position.y];
        if let Some(button) = self.button_at(self.cursor) {
            self.set_active(evt.pointer_id, button);
        }
    }

    fn handle_pointer_up(&mut self, evt: app::PointerEvent) {
        self.cursor = [evt.position.x, evt.position.y];
        if let Some((id, button)) = self.active_pointer {
            if id == evt.pointer_id && self.button_at(self.cursor) == Some(button) {
                match button {
                    Button::Minus => self.adjust_count(-1),
                    Button::Plus => self.adjust_count(1),
                }
            }
        }
        self.clear_active();
    }

    fn handle_pointer_move(&mut self, evt: app::PointerEvent) {
        self.cursor = [evt.position.x, evt.position.y];
        let hover = self.button_at(self.cursor);
        self.set_hover(hover);
    }

    fn handle_key_down(&mut self, evt: app::KeyEvent) {
        match evt.key.as_str() {
            "+" | "=" => self.adjust_count(1),
            "-" => self.adjust_count(-1),
            "Space" => self.adjust_count(1),
            "Enter" => self.reset_count(),
            other if other.trim() == "+" => self.adjust_count(1),
            other if other.trim() == "-" => self.adjust_count(-1),
            _ => {}
        }
    }

    fn handle_key_up(&mut self, _evt: app::KeyEvent) {}

    fn handle_frame(&mut self, _dt_ms: f32) {
        self.draw();
    }
}

struct Component;

impl Guest for Component {
    fn init(initial: app::LogicalSize) {
        with_state(|state| state.handle_init(initial));
    }

    fn resize(new: app::LogicalSize) {
        with_state(|state| state.handle_resize(new));
    }

    fn pointer_down(evt: app::PointerEvent) {
        with_state(|state| state.handle_pointer_down(evt));
    }

    fn pointer_up(evt: app::PointerEvent) {
        with_state(|state| state.handle_pointer_up(evt));
    }

    fn pointer_move(evt: app::PointerEvent) {
        with_state(|state| state.handle_pointer_move(evt));
    }

    fn key_down(evt: app::KeyEvent) {
        with_state(|state| state.handle_key_down(evt));
    }

    fn key_up(evt: app::KeyEvent) {
        with_state(|state| state.handle_key_up(evt));
    }

    fn frame(dt_ms: f32) {
        with_state(|state| state.handle_frame(dt_ms));
    }
}

struct Layout {
    panel: Rect,
    minus: Rect,
    plus: Rect,
    count_text_size: f32,
    count_origin: [f32; 2],
    hint_origin: [f32; 2],
}

impl Layout {
    fn from_size(size: app::LogicalSize) -> Self {
        let width = size.width.max(1.0);
        let height = size.height.max(1.0);
        let margin = (width.min(height) * 0.08).clamp(12.0, 48.0);

        let panel = Rect {
            x: margin,
            y: margin,
            w: width - margin * 2.0,
            h: height - margin * 2.0,
        };

        let button_height = (panel.h * 0.35).clamp(48.0, 160.0);
        let button_width = (panel.w * 0.25).clamp(96.0, 220.0);
        let button_y = panel.y + panel.h - button_height - margin;
        let button_margin = margin * 0.5;

        let minus = Rect {
            x: panel.x + button_margin,
            y: button_y,
            w: button_width,
            h: button_height,
        };
        let plus = Rect {
            x: panel.x + panel.w - button_margin - button_width,
            y: button_y,
            w: button_width,
            h: button_height,
        };

        let count_text_size = (panel.h * 0.35).clamp(48.0, 160.0);
        let count_origin = [
            panel.x + panel.w * 0.5 - count_text_size * 0.35,
            panel.y + panel.h * 0.4,
        ];

        let hint_origin = [
            panel.x + button_margin,
            panel.y + panel.h - button_margin * 0.5,
        ];

        Self {
            panel,
            minus,
            plus,
            count_text_size,
            count_origin,
            hint_origin,
        }
    }

    fn panel_origin(&self) -> [f32; 2] {
        [self.panel.x, self.panel.y]
    }

    fn panel_size(&self) -> [f32; 2] {
        [self.panel.w, self.panel.h]
    }

    fn count_label_origin(&self) -> [f32; 2] {
        self.count_origin
    }
}

fn host_color(r: f32, g: f32, b: f32, a: f32) -> host::Color {
    host::Color { r, g, b, a }
}

fn to_vec2(value: [f32; 2]) -> HostVec2 {
    HostVec2 {
        x: value[0],
        y: value[1],
    }
}

bindings::export!(Component with_types_in bindings);
