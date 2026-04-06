use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use tracing::warn;
use ragnarok_formats::grf::GrfArchive;
use ragnarok_game::item_resource_table::ItemResourceTable;
use ragnarok_renderer::font_atlas::FontAtlas;
use ragnarok_renderer::texture::{self, TextureCache};
use ragnarok_renderer::ui_renderer::{UiDrawCommand, UiRenderer};
use ragnarok_renderer::{RenderDevice, UiTextureRef, block_on};
use ragnarok_ui::context::UiContext;
use ragnarok_ui::frame::UiFrame;
use ragnarok_ui::state::StateCache;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

struct Gpu {
    device: RenderDevice,
    texture_cache: TextureCache,
    font_atlas: FontAtlas,
    font_atlas_bind_group: wgpu::BindGroup,
    white_bind_group: wgpu::BindGroup,
    ui_renderer: UiRenderer,
}

/// Passed to the build closure so components can lazily call `set_texture_sizes()`.
pub struct ExampleCtx<'a> {
    pub ui: UiFrame<'a>,
    pub texture_size: &'a dyn Fn(&str) -> Option<(u32, u32)>,
    pub item_resource_table: Option<&'a ItemResourceTable>,
}

pub struct UiExampleApp<F> {
    title: &'static str,
    width: u32,
    height: u32,
    build_fn: F,
    window: Option<Arc<Window>>,
    gpu: Option<Gpu>,
    grf: Option<GrfArchive>,
    item_resource_table: Option<ItemResourceTable>,
    has_grf_textures: bool,
    ui_ctx: UiContext,
    state_cache: StateCache,
    start_time: Instant,
    grf_path: Option<String>,
    texture_paths: Vec<&'static str>,
}

impl<F: FnMut(&mut ExampleCtx)> UiExampleApp<F> {
    pub fn new(title: &'static str, width: u32, height: u32, build_fn: F) -> Self {
        Self {
            title,
            width,
            height,
            build_fn,
            window: None,
            gpu: None,
            grf: None,
            item_resource_table: None,
            has_grf_textures: false,
            ui_ctx: UiContext::new(width as f32, height as f32),
            state_cache: StateCache::new(),
            start_time: Instant::now(),
            grf_path: None,
            texture_paths: Vec::new(),
        }
    }

    pub fn with_grf_textures(mut self, texture_paths: Vec<&'static str>) -> Self {
        self.texture_paths = texture_paths;
        self
    }

    pub fn run(self) {
        tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .init();

        let event_loop = EventLoop::new().expect("Failed to create event loop");
        let mut app = self;
        // Parse --grf <path> from command line
        let args: Vec<String> = std::env::args().collect();
        let mut i = 1;
        while i < args.len() {
            if args[i] == "--grf" {
                i += 1;
                if i < args.len() {
                    app.grf_path = Some(args[i].clone());
                }
            }
            i += 1;
        }
        if app.grf_path.is_none() {
            warn!("No grf specified try loading at data/data.grf");
            app.grf_path = Some(String::from("data/data.grf"));
        }
        event_loop.run_app(&mut app).unwrap();
    }

    fn render_frame(&mut self) {
        let Some(gpu) = &mut self.gpu else { return };

        let output = match gpu.device.surface.get_current_texture() {
            Ok(tex) => tex,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                gpu.device.reconfigure_surface();
                return;
            }
            Err(wgpu::SurfaceError::Timeout) => return,
            Err(e) => {
                tracing::error!("Surface error: {e}");
                return;
            }
        };

        let elapsed = self.start_time.elapsed().as_secs_f32();
        let ui = UiFrame::new(
            &self.ui_ctx,
            &gpu.font_atlas,
            &mut self.state_cache,
            elapsed,
            self.has_grf_textures,
            None,
        );

        let texture_size = |name: &str| -> Option<(u32, u32)> {
            gpu.texture_cache.texture_size(name)
        };
        let mut ctx = ExampleCtx { ui, texture_size: &texture_size, item_resource_table: self.item_resource_table.as_ref() };
        (self.build_fn)(&mut ctx);

        let draw_calls = std::mem::take(&mut ctx.ui.draw_calls);
        drop(ctx);

        let view = output.texture.create_view(&Default::default());
        let mut encoder = gpu.device.device.create_command_encoder(&Default::default());

        // Clear pass
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("clear"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.1,
                        g: 0.1,
                        b: 0.15,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            ..Default::default()
        });

        // Lazily load any Named textures not yet in cache from GRF
        if let Some(grf) = &self.grf {
            for call in &draw_calls {
                if let UiTextureRef::Named(name) = &call.texture {
                    if gpu.texture_cache.get(name).is_none() {
                        gpu.texture_cache.get_or_load(
                            name, grf, &gpu.device.device, &gpu.device.queue, true,
                        );
                    }
                }
            }
        }

        let resolved: Vec<UiDrawCommand> = draw_calls
            .iter()
            .map(|call| {
                let bind_group = match &call.texture {
                    UiTextureRef::FontAtlas => &gpu.font_atlas_bind_group,
                    UiTextureRef::White => &gpu.white_bind_group,
                    UiTextureRef::Named(name) => {
                        gpu.texture_cache.get(name)
                            .unwrap_or(&gpu.white_bind_group)
                    }
                };
                UiDrawCommand {
                    vertices: &call.vertices,
                    indices: &call.indices,
                    texture: bind_group,
                }
            })
            .collect();

        gpu.ui_renderer.render(
            &mut encoder,
            &view,
            &gpu.device.device,
            &gpu.device.queue,
            &resolved,
        );

        gpu.device.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    }
}

impl<F: FnMut(&mut ExampleCtx)> ApplicationHandler for UiExampleApp<F> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attrs = WindowAttributes::default()
            .with_title(self.title)
            .with_inner_size(winit::dpi::PhysicalSize::new(self.width, self.height));

        let window = Arc::new(event_loop.create_window(attrs).unwrap());
        let device = block_on(RenderDevice::new(window.clone()));
        let dpi_scale = 1.5_f32;
        let mut tex_cache = TextureCache::new(&device.device, dpi_scale);

        let font_atlas = FontAtlas::from_embedded(14.0, dpi_scale);
        let font_atlas_bind_group = texture::create_font_atlas_bind_group(
            &device.device,
            &device.queue,
            &font_atlas.image,
            &tex_cache.bind_group_layout,
            "font_atlas",
        );

        let white_img = image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 255, 255, 255]));
        let white_bind_group = texture::create_texture_bind_group(
            &device.device,
            &device.queue,
            &white_img,
            &tex_cache.bind_group_layout,
            "ui_white",
        );

        let w = device.surface_config.width as f32 / dpi_scale;
        let h = device.surface_config.height as f32 / dpi_scale;
        let ui_renderer = UiRenderer::new(
            &device.device,
            device.surface_format,
            &tex_cache.bind_group_layout,
            w,
            h,
        );

        // Load GRF and preload textures if --grf was provided
        if let Some(grf_path) = &self.grf_path {
            match GrfArchive::open(Path::new(grf_path)) {
                Ok(grf) => {
                    let mut all_loaded = true;
                    for path in &self.texture_paths {
                        if tex_cache.get_or_load(
                            path, &grf, &device.device, &device.queue, true,
                        ).is_none() {
                            all_loaded = false;
                        }
                    }
                    self.has_grf_textures = all_loaded;
                    self.item_resource_table = Some(ItemResourceTable::load(&grf));
                    self.grf = Some(grf);
                }
                Err(e) => {
                    eprintln!("Failed to open GRF {grf_path}: {e}");
                }
            }
        }

        self.ui_ctx.dpi_scale = dpi_scale;
        self.ui_ctx.screen_width = device.surface_config.width as f32 / dpi_scale;
        self.ui_ctx.screen_height = device.surface_config.height as f32 / dpi_scale;

        self.gpu = Some(Gpu {
            device,
            texture_cache: tex_cache,
            font_atlas,
            font_atlas_bind_group,
            white_bind_group,
            ui_renderer,
        });
        self.window = Some(window);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        self.ui_ctx.handle_event(&event);

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(gpu) = &mut self.gpu {
                    let dpi = self.ui_ctx.dpi_scale;
                    gpu.device.resize(size.width, size.height);
                    gpu.ui_renderer.resize(
                        &gpu.device.queue,
                        size.width as f32 / dpi,
                        size.height as f32 / dpi,
                    );
                    self.ui_ctx.screen_width = size.width as f32 / dpi;
                    self.ui_ctx.screen_height = size.height as f32 / dpi;
                }
            }
            WindowEvent::RedrawRequested => {
                self.render_frame();
                self.ui_ctx.begin_frame();
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }
}
