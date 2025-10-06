use std::collections::VecDeque;
use std::fmt;

use crate::component::vello::canvas::host::{Host as GuestHost, LogLevel};
use crate::component::vello::canvas::math::{Color as WitColor, Vec2 as WitVec2};

#[derive(Clone, Copy, Debug, Default)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub fn from_wit(color: WitColor) -> Self {
        Self {
            r: color.r,
            g: color.g,
            b: color.b,
            a: color.a,
        }
    }

    pub fn to_peniko(self) -> vello::peniko::Color {
        vello::peniko::Color::new([self.r, self.g, self.b, self.a])
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub fn from_wit(vec: WitVec2) -> Self {
        Self { x: vec.x, y: vec.y }
    }
}

#[derive(Debug, Clone)]
pub enum DrawCommand {
    FillRect {
        origin: Vec2,
        size: Vec2,
        color: Color,
    },
    DrawText {
        text: String,
        origin: Vec2,
        size: f32,
        color: Color,
    },
}

#[derive(Debug, Default, Clone)]
pub struct FrameOutput {
    pub clear_color: Option<Color>,
    pub commands: Vec<DrawCommand>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Phase {
    #[default]
    Idle,
    Init,
    Resize,
    Event,
    Frame,
}

impl Phase {
    fn allows_draw(self) -> bool {
        matches!(self, Phase::Frame)
    }

    fn allows_request_frame(self) -> bool {
        !matches!(self, Phase::Idle)
    }
}

const RECENT_LOG_LIMIT: usize = 16;

#[derive(Default, Debug)]
pub struct HostCtx {
    phase: Phase,
    frame: FrameOutput,
    redraw_requested: bool,
    recent_logs: VecDeque<String>,
}

impl HostCtx {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn enter_phase(&mut self, phase: Phase) {
        if matches!(phase, Phase::Frame) {
            self.frame.clear_color = None;
            self.frame.commands.clear();
        }
        self.phase = phase;
    }

    pub fn exit_phase(&mut self) {
        self.phase = Phase::Idle;
    }

    pub fn take_frame_output(&mut self) -> FrameOutput {
        let commands = self.frame.commands.drain(..).collect();
        FrameOutput {
            clear_color: self.frame.clear_color.take(),
            commands,
        }
    }

    pub fn take_redraw_request(&mut self) -> bool {
        let requested = self.redraw_requested;
        self.redraw_requested = false;
        requested
    }

    pub fn recent_logs_snapshot(&self) -> Vec<String> {
        self.recent_logs.iter().cloned().collect()
    }

    fn record_guest_log(&mut self, level: LogLevel, message: &str) {
        if self.recent_logs.len() == RECENT_LOG_LIMIT {
            self.recent_logs.pop_front();
        }
        let level_label = match level {
            LogLevel::Trace => "TRACE",
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        };
        self.recent_logs
            .push_back(format!("[{level_label}] {message}"));
    }

    fn push_command(&mut self, cmd: DrawCommand) {
        self.frame.commands.push(cmd);
    }

    fn warn_out_of_phase(&self, action: &str) {
        tracing::warn!(phase = ?self.phase, "guest attempted to {action} outside of a frame phase");
    }
}

impl GuestHost for HostCtx {
    fn clear(&mut self, color: WitColor) {
        if self.phase.allows_draw() {
            self.frame.clear_color = Some(Color::from_wit(color));
        } else {
            self.warn_out_of_phase("clear the scene");
        }
    }

    fn fill_rect(&mut self, origin: WitVec2, size: WitVec2, color: WitColor) {
        if self.phase.allows_draw() {
            self.push_command(DrawCommand::FillRect {
                origin: Vec2::from_wit(origin),
                size: Vec2::from_wit(size),
                color: Color::from_wit(color),
            });
        } else {
            self.warn_out_of_phase("issue fill-rect");
        }
    }

    fn draw_text(&mut self, text: String, origin: WitVec2, size: f32, color: WitColor) {
        if self.phase.allows_draw() {
            self.push_command(DrawCommand::DrawText {
                text,
                origin: Vec2::from_wit(origin),
                size,
                color: Color::from_wit(color),
            });
        } else {
            self.warn_out_of_phase("draw text");
        }
    }

    fn request_frame(&mut self) {
        if self.phase.allows_request_frame() {
            self.redraw_requested = true;
        } else {
            tracing::debug!(phase = ?self.phase, "guest requested frame while idle; ignoring");
        }
    }

    fn log(&mut self, level: LogLevel, message: String) {
        self.record_guest_log(level, &message);
        match level {
            LogLevel::Trace => tracing::trace!(target: "guest", "{message}"),
            LogLevel::Debug => tracing::debug!(target: "guest", "{message}"),
            LogLevel::Info => tracing::info!(target: "guest", "{message}"),
            LogLevel::Warn => tracing::warn!(target: "guest", "{message}"),
            LogLevel::Error => tracing::error!(target: "guest", "{message}"),
        }
    }
}

impl fmt::Display for DrawCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DrawCommand::FillRect { origin, size, .. } => {
                write!(
                    f,
                    "FillRect(origin=({:.1}, {:.1}), size=({:.1}, {:.1}))",
                    origin.x, origin.y, size.x, size.y
                )
            }
            DrawCommand::DrawText {
                text, origin, size, ..
            } => {
                write!(
                    f,
                    "DrawText(text='{text}', origin=({:.1}, {:.1}), size={:.1})",
                    origin.x, origin.y, size
                )
            }
        }
    }
}
