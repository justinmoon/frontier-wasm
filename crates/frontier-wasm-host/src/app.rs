use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;
use tracing::error;
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::keyboard::{Key, PhysicalKey};
use winit::window::{Window, WindowAttributes};

use crate::graphics::{GraphicsState, OverlayContent};
use crate::model::{
    KeyEvent as GuestKeyEvent, LogicalSize, Modifiers, PointerButtons, PointerEvent, PointerKind,
};
use crate::runtime::{CallResult, ComponentRuntime, FrameResult};

pub struct App {
    component_path: PathBuf,
    window: Option<Arc<Window>>,
    runtime: Option<ComponentRuntime>,
    graphics: Option<GraphicsState>,
    logical_size: LogicalSize,
    scale_factor: f32,
    pointer_buttons: PointerButtons,
    modifiers: Modifiers,
    last_frame_instant: Option<Instant>,
    needs_redraw: bool,
    overlay: Option<OverlayState>,
    cursor_position: PhysicalPosition<f64>,
}

#[derive(Clone, Debug)]
struct OverlayState {
    title: String,
    body: String,
    footer: String,
}

impl OverlayState {
    fn to_content(&self) -> OverlayContent {
        OverlayContent {
            title: self.title.clone(),
            body: self.body.lines().map(|s| s.to_string()).collect(),
            footer: self.footer.clone(),
        }
    }
}

impl App {
    pub fn new(component_path: PathBuf) -> Self {
        Self {
            component_path,
            window: None,
            runtime: None,
            graphics: None,
            logical_size: LogicalSize::default(),
            scale_factor: 1.0,
            pointer_buttons: PointerButtons::default(),
            modifiers: Modifiers::default(),
            last_frame_instant: None,
            needs_redraw: false,
            overlay: None,
            cursor_position: PhysicalPosition::new(0.0, 0.0),
        }
    }

    fn request_redraw(&mut self) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn ensure_runtime(&mut self) -> Result<()> {
        if self.runtime.is_some() {
            return Ok(());
        }
        let runtime = ComponentRuntime::new(self.component_path.clone())?;
        self.runtime = Some(runtime);
        Ok(())
    }

    fn ensure_graphics(&mut self, window: Arc<Window>) -> Result<()> {
        if self.graphics.is_some() {
            return Ok(());
        }
        let graphics = GraphicsState::new(window.clone(), self.scale_factor, self.logical_size)?;
        self.graphics = Some(graphics);
        self.window = Some(window);
        Ok(())
    }

    fn handle_call_result(&mut self, result: CallResult) {
        if result.requested_redraw {
            self.request_redraw();
        }
    }

    fn handle_frame_result(&mut self, frame: FrameResult) -> Result<()> {
        if frame.requested_redraw {
            self.request_redraw();
        }
        let overlay_content = self.overlay.as_ref().map(|state| state.to_content());
        if let Some(graphics) = self.graphics.as_mut() {
            graphics.render(Some(&frame.frame), overlay_content.as_ref())?;
        }
        Ok(())
    }

    fn render_overlay_only(&mut self) -> Result<()> {
        if let Some(graphics) = self.graphics.as_mut() {
            let overlay_content = self.overlay.as_ref().map(|state| state.to_content());
            graphics.render(None, overlay_content.as_ref())?;
        }
        Ok(())
    }

    fn schedule_restart(&mut self) {
        if self.runtime.is_none() {
            match ComponentRuntime::new(self.component_path.clone()) {
                Ok(runtime) => self.runtime = Some(runtime),
                Err(err) => {
                    self.set_overlay_error("Failed to restart component", &err);
                    return;
                }
            }
        }

        if let Some(runtime) = self.runtime.as_mut() {
            if let Err(err) = runtime.reload() {
                self.set_overlay_error("Failed to restart component", &err);
                return;
            }
            if let Err(err) = runtime.call_init(self.logical_size) {
                self.set_overlay_error("Component init failed", &err);
            } else {
                self.overlay = None;
                self.request_redraw();
            }
        }
    }

    fn set_overlay_error(&mut self, title: &str, err: &anyhow::Error) {
        error!(error = %err, "guest runtime error");
        self.overlay = Some(OverlayState {
            title: title.to_string(),
            body: format!("{err:#}"),
            footer: "Press R to restart the component".to_string(),
        });
        self.request_redraw();
    }

    fn logical_from_physical(&self, size: PhysicalSize<u32>) -> LogicalSize {
        let scale = self.scale_factor.max(0.0001) as f32;
        LogicalSize {
            width: size.width as f32 / scale,
            height: size.height as f32 / scale,
            scale_factor: self.scale_factor,
        }
    }

    fn pointer_event(&self, position: PhysicalPosition<f64>) -> PointerEvent {
        let logical = position.to_logical::<f64>(self.scale_factor as f64);
        PointerEvent {
            kind: PointerKind::Mouse,
            position: [logical.x as f32, logical.y as f32],
            buttons: self.pointer_buttons,
            modifiers: self.modifiers,
            pointer_id: 0,
        }
    }

    fn key_event_from_winit(&self, event: &KeyEvent) -> GuestKeyEvent {
        let key = match &event.logical_key {
            Key::Character(ch) => ch.to_string(),
            Key::Named(named) => format!("{named:?}"),
            Key::Dead(dead) => format!("Dead({dead:?})"),
            other => format!("{other:?}"),
        };
        let code = match &event.physical_key {
            PhysicalKey::Code(code) => format!("{code:?}"),
            PhysicalKey::Unidentified(id) => format!("Unidentified({id:?})"),
        };
        GuestKeyEvent {
            key,
            code,
            modifiers: self.modifiers,
            is_repeat: event.repeat,
        }
    }

    fn tick_frame_time(&mut self) -> f32 {
        let now = Instant::now();
        let dt = if let Some(last) = self.last_frame_instant.replace(now) {
            (now - last).as_secs_f32() * 1000.0
        } else {
            0.0
        };
        dt
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window = event_loop
            .create_window(
                WindowAttributes::default()
                    .with_title("Frontier Canvas Prototype")
                    .with_inner_size(PhysicalSize::new(900, 600)),
            )
            .expect("failed to create window");
        let window = Arc::new(window);
        self.scale_factor = window.scale_factor() as f32;
        let physical = window.inner_size();
        self.logical_size = self.logical_from_physical(physical);

        if let Err(err) = self.ensure_graphics(window.clone()) {
            self.set_overlay_error("Graphics initialisation failed", &err);
            return;
        }

        if let Err(err) = self.ensure_runtime() {
            self.set_overlay_error("Runtime initialisation failed", &err);
            return;
        }

        if let Some(graphics) = self.graphics.as_mut() {
            graphics.set_logical_size(self.logical_size);
            graphics.set_scale_factor(self.scale_factor);
        }

        if let Some(runtime) = self.runtime.as_mut() {
            match runtime.call_init(self.logical_size) {
                Ok(result) => {
                    self.handle_call_result(result);
                }
                Err(err) => {
                    self.set_overlay_error("Component init failed", &err);
                }
            }
        }

        self.request_redraw();
        event_loop.set_control_flow(ControlFlow::Wait);
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if self.needs_redraw {
            self.request_redraw();
            self.needs_redraw = false;
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let Some(window) = &self.window else {
            return;
        };
        if window.id() != window_id {
            return;
        }

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(graphics) = self.graphics.as_mut() {
                    graphics.resize(size);
                }
                let logical = self.logical_from_physical(size);
                self.logical_size = logical;
                if let Some(graphics) = self.graphics.as_mut() {
                    graphics.set_logical_size(logical);
                }
                if let Some(runtime) = self.runtime.as_mut() {
                    match runtime.call_resize(logical) {
                        Ok(result) => self.handle_call_result(result),
                        Err(err) => self.set_overlay_error("Component resize failed", &err),
                    }
                }
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale_factor = scale_factor as f32;
                let size = window.inner_size();
                let logical = self.logical_from_physical(size);
                self.logical_size = logical;
                if let Some(graphics) = self.graphics.as_mut() {
                    graphics.set_scale_factor(self.scale_factor);
                    graphics.set_logical_size(logical);
                    graphics.resize(size);
                }
                if let Some(runtime) = self.runtime.as_mut() {
                    match runtime.call_resize(logical) {
                        Ok(result) => self.handle_call_result(result),
                        Err(err) => self.set_overlay_error("Component resize failed", &err),
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                if self.overlay.is_some() {
                    if let Err(err) = self.render_overlay_only() {
                        self.set_overlay_error("Overlay render failed", &err);
                    }
                    return;
                }

                let dt_ms = self.tick_frame_time();
                if let Some(runtime) = self.runtime.as_mut() {
                    match runtime.call_frame(dt_ms) {
                        Ok(frame) => {
                            if let Err(err) = self.handle_frame_result(frame) {
                                self.set_overlay_error("Render failed", &err);
                            }
                        }
                        Err(err) => self.set_overlay_error("Component frame failed", &err),
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_position = position;
                if self.overlay.is_some() {
                    return;
                }
                let event = self.pointer_event(position);
                if let Some(runtime) = self.runtime.as_mut() {
                    match runtime.call_pointer_move(&event) {
                        Ok(result) => self.handle_call_result(result),
                        Err(err) => self.set_overlay_error("Pointer move failed", &err),
                    }
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if self.overlay.is_some() {
                    return;
                }
                if button == MouseButton::Left {
                    self.pointer_buttons.primary = state == ElementState::Pressed;
                } else if button == MouseButton::Right {
                    self.pointer_buttons.secondary = state == ElementState::Pressed;
                }
                let event = self.pointer_event(self.cursor_position);
                if let Some(runtime) = self.runtime.as_mut() {
                    let result = match state {
                        ElementState::Pressed => runtime.call_pointer_down(&event),
                        ElementState::Released => runtime.call_pointer_up(&event),
                    };
                    match result {
                        Ok(res) => self.handle_call_result(res),
                        Err(err) => self.set_overlay_error("Pointer button failed", &err),
                    }
                }
            }
            WindowEvent::ModifiersChanged(state) => {
                let state = state.state();
                self.modifiers = Modifiers {
                    shift: state.shift_key(),
                    ctrl: state.control_key(),
                    alt: state.alt_key(),
                    meta: state.super_key(),
                };
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let ElementState::Pressed = event.state {
                    if let Key::Character(ch) = &event.logical_key {
                        if self.overlay.is_some() && ch.eq_ignore_ascii_case("r") {
                            self.schedule_restart();
                            return;
                        }
                    }
                }

                if self.overlay.is_some() {
                    return;
                }

                let key_event = self.key_event_from_winit(&event);
                if let Some(runtime) = self.runtime.as_mut() {
                    let result = match event.state {
                        ElementState::Pressed => runtime.call_key_down(&key_event),
                        ElementState::Released => runtime.call_key_up(&key_event),
                    };
                    match result {
                        Ok(res) => self.handle_call_result(res),
                        Err(err) => self.set_overlay_error("Key event failed", &err),
                    }
                }
            }
            _ => {}
        }
    }
}
