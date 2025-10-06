use std::fmt;

#[derive(Clone, Copy, Debug, Default)]
pub struct LogicalSize {
    pub width: f32,
    pub height: f32,
    pub scale_factor: f32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PointerButtons {
    pub primary: bool,
    pub secondary: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum PointerKind {
    Mouse,
    Touch,
    Pen,
}

impl fmt::Display for PointerKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PointerKind::Mouse => f.write_str("mouse"),
            PointerKind::Touch => f.write_str("touch"),
            PointerKind::Pen => f.write_str("pen"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PointerEvent {
    pub kind: PointerKind,
    pub position: [f32; 2],
    pub buttons: PointerButtons,
    pub modifiers: Modifiers,
    pub pointer_id: u64,
}

#[derive(Clone, Debug, Default)]
pub struct KeyEvent {
    pub key: String,
    pub code: String,
    pub modifiers: Modifiers,
    pub is_repeat: bool,
}
