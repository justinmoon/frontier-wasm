use std::sync::Arc;

use anyhow::{bail, Context, Result};
use vello::kurbo::{Affine, Rect};
use vello::peniko::{Brush, Fill};
use vello::util::{RenderContext, RenderSurface};
use vello::{AaConfig, Glyph, Renderer, RendererOptions, Scene};
use wgpu::SurfaceError;
use winit::dpi::PhysicalSize;
use winit::window::Window;

use crate::host::{Color, DrawCommand, FrameOutput};

const FONT_BYTES: &[u8] = include_bytes!("../../../assets/Cantarell-Regular.ttf");

pub struct OverlayContent {
    pub title: String,
    pub body: Vec<String>,
    pub footer: String,
}

pub struct GraphicsState {
    render_cx: RenderContext,
    surface: RenderSurface<'static>,
    renderer: Renderer,
    scene: Scene,
    font: FontAssets,
    scale_factor: f32,
    logical_size: crate::model::LogicalSize,
    default_clear: Color,
}

struct FontAssets {
    font_data: vello::peniko::FontData,
    font_arc: ab_glyph::FontArc,
}

impl GraphicsState {
    pub fn new(
        window: Arc<Window>,
        scale_factor: f32,
        logical_size: crate::model::LogicalSize,
    ) -> Result<Self> {
        let mut render_cx = RenderContext::new();
        let physical = window.inner_size();
        let surface = pollster::block_on(render_cx.create_surface(
            window.clone(),
            physical.width.max(1),
            physical.height.max(1),
            wgpu::PresentMode::Fifo,
        ))
        .context("failed to create wgpu surface")?;

        let device = &render_cx.devices[surface.dev_id].device;
        let renderer = Renderer::new(
            device,
            RendererOptions {
                use_cpu: false,
                antialiasing_support: vello::AaSupport::area_only(),
                num_init_threads: if cfg!(target_os = "macos") {
                    Some(std::num::NonZeroUsize::new(1).unwrap())
                } else {
                    None
                },
                pipeline_cache: None,
            },
        )
        .context("failed to initialise vello renderer")?;

        let font = FontAssets::new().context("failed to prepare font assets")?;

        Ok(Self {
            render_cx,
            surface,
            renderer,
            scene: Scene::new(),
            font,
            scale_factor,
            logical_size,
            default_clear: Color {
                r: 0.06,
                g: 0.07,
                b: 0.09,
                a: 1.0,
            },
        })
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 {
            return;
        }
        self.render_cx
            .resize_surface(&mut self.surface, new_size.width, new_size.height);
    }

    pub fn set_logical_size(&mut self, logical_size: crate::model::LogicalSize) {
        self.logical_size = logical_size;
    }

    pub fn set_scale_factor(&mut self, scale: f32) {
        self.scale_factor = scale;
    }

    pub fn render(
        &mut self,
        frame: Option<&FrameOutput>,
        overlay: Option<&OverlayContent>,
    ) -> Result<()> {
        self.scene.reset();

        let mut base_color = self.default_clear;

        if let Some(frame) = frame {
            if let Some(clear) = frame.clear_color {
                base_color = clear;
            }
            for command in &frame.commands {
                self.apply_command(command);
            }
        }

        if let Some(overlay) = overlay {
            self.draw_overlay(overlay);
        }

        let device_handle = &self.render_cx.devices[self.surface.dev_id];
        let device = &device_handle.device;
        let queue = &device_handle.queue;

        let render_params = vello::RenderParams {
            base_color: base_color.to_peniko(),
            width: self.surface.config.width,
            height: self.surface.config.height,
            antialiasing_method: AaConfig::Area,
        };

        self.renderer
            .render_to_texture(
                device,
                queue,
                &self.scene,
                &self.surface.target_view,
                &render_params,
            )
            .context("vello render failed")?;

        let frame = match self.surface.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(SurfaceError::Lost) => {
                tracing::warn!("surface lost, reconfiguring");
                self.resize(PhysicalSize::new(
                    self.surface.config.width,
                    self.surface.config.height,
                ));
                return Ok(());
            }
            Err(SurfaceError::OutOfMemory) => {
                bail!("surface out of memory");
            }
            Err(err) => {
                tracing::warn!(?err, "failed to acquire surface texture");
                return Ok(());
            }
        };

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("frontier.present"),
        });

        self.surface
            .blitter
            .copy(device, &mut encoder, &self.surface.target_view, &view);

        queue.submit(std::iter::once(encoder.finish()));
        frame.present();
        Ok(())
    }

    fn apply_command(&mut self, command: &DrawCommand) {
        match command {
            DrawCommand::FillRect {
                origin,
                size,
                color,
            } => {
                self.draw_rect([origin.x, origin.y], [size.x, size.y], *color);
            }
            DrawCommand::DrawText {
                text,
                origin,
                size,
                color,
            } => {
                self.draw_text(text.as_str(), [origin.x, origin.y], *size, *color);
            }
        }
    }

    fn draw_rect(&mut self, origin: [f32; 2], size: [f32; 2], color: Color) {
        let x0 = (origin[0] * self.scale_factor) as f64;
        let y0 = (origin[1] * self.scale_factor) as f64;
        let rect = Rect::new(
            x0,
            y0,
            x0 + (size[0] * self.scale_factor) as f64,
            y0 + (size[1] * self.scale_factor) as f64,
        );
        self.scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            Brush::Solid(color.to_peniko()),
            None,
            &rect,
        );
    }

    fn draw_text(&mut self, text: &str, origin: [f32; 2], size: f32, color: Color) {
        if text.is_empty() {
            return;
        }
        let physical_origin = [origin[0] * self.scale_factor, origin[1] * self.scale_factor];
        let font_size = size * self.scale_factor;
        let glyphs = layout_text(&self.font.font_arc, text, font_size);
        if glyphs.is_empty() {
            return;
        }
        self.scene
            .draw_glyphs(&self.font.font_data)
            .font_size(font_size)
            .brush(Brush::Solid(color.to_peniko()))
            .transform(Affine::translate((
                physical_origin[0] as f64,
                physical_origin[1] as f64,
            )))
            .draw(Fill::NonZero, glyphs.into_iter());
    }

    fn draw_overlay(&mut self, overlay: &OverlayContent) {
        let width = self.logical_size.width;
        let height = self.logical_size.height;
        self.draw_rect(
            [0.0, 0.0],
            [width, height],
            Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.7,
            },
        );

        let mut cursor_y = height * 0.2;
        let title_color = Color {
            r: 1.0,
            g: 0.78,
            b: 0.2,
            a: 1.0,
        };
        self.draw_text(&overlay.title, [width * 0.1, cursor_y], 28.0, title_color);
        cursor_y += 36.0;

        let body_color = Color {
            r: 0.9,
            g: 0.9,
            b: 0.9,
            a: 1.0,
        };
        for line in &overlay.body {
            self.draw_text(line, [width * 0.1, cursor_y], 20.0, body_color);
            cursor_y += 26.0;
        }

        cursor_y += 16.0;
        let footer_color = Color {
            r: 0.7,
            g: 0.7,
            b: 0.7,
            a: 1.0,
        };
        self.draw_text(&overlay.footer, [width * 0.1, cursor_y], 18.0, footer_color);
    }
}

impl FontAssets {
    fn new() -> Result<Self> {
        let font_arc = ab_glyph::FontArc::try_from_slice(FONT_BYTES)
            .context("embedded font corrupted or unsupported")?;
        let blob: vello::peniko::Blob<u8> = FONT_BYTES.to_vec().into();
        let font_data = vello::peniko::FontData::new(blob, 0);
        Ok(Self {
            font_data,
            font_arc,
        })
    }
}

fn layout_text(font: &ab_glyph::FontArc, text: &str, font_size: f32) -> Vec<Glyph> {
    use ab_glyph::{Font, ScaleFont};

    let mut glyphs = Vec::with_capacity(text.len());
    let scaled = font.as_scaled(font_size);
    let mut caret_x = 0.0f32;
    let mut caret_y = 0.0f32;
    let line_height = font_size * 1.2;
    for ch in text.chars() {
        if ch == '\n' {
            caret_x = 0.0;
            caret_y += line_height;
            continue;
        }
        let glyph_id = scaled.glyph_id(ch);
        glyphs.push(Glyph {
            id: glyph_id.0 as u32,
            x: caret_x,
            y: caret_y,
        });
        caret_x += scaled.h_advance(glyph_id);
    }
    glyphs
}
