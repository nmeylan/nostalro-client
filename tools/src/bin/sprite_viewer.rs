use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use ragnarok_formats::grf::GrfArchive;
use ragnarok_formats::act::ActFile;
use ragnarok_formats::spr::SprFile;
use ragnarok_renderer::font_atlas::FontAtlas;
use ragnarok_renderer::sprite::{
    SpriteRenderer, SpriteTextures, SpriteUniforms, SpriteBatch,
    build_clip_quad, upload_sprite_textures,
};
use ragnarok_renderer::texture::{self, TextureCache};
use ragnarok_renderer::ui_renderer::{UiDrawCommand, UiRenderer};
use ragnarok_renderer::{RenderDevice, UiTextureRef};
use ragnarok_tools::sprite_viewer::animation::AnimationState;
use ragnarok_tools::sprite_viewer::browser::SpriteBrowser;
use ragnarok_tools::sprite_viewer::controls::{self, Background, ViewerAction};
use ragnarok_tools::sprite_viewer::shader_watcher::ShaderWatcher;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowAttributes, WindowId};

struct Args {
    grf_path: Option<String>,
    sprite_path: Option<String>,
    list: bool,
}

fn parse_args() -> Args {
    let args: Vec<String> = std::env::args().collect();
    let mut grf_path = None;
    let mut sprite_path = None;
    let mut list = false;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--grf" => {
                i += 1;
                if i < args.len() { grf_path = Some(args[i].clone()); }
            }
            "--sprite" => {
                i += 1;
                if i < args.len() { sprite_path = Some(args[i].clone()); }
            }
            "--list" => { list = true; }
            _ => {}
        }
        i += 1;
    }
    Args { grf_path, sprite_path, list }
}

fn scan_grf_files() -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(".") else { return Vec::new() };
    let mut files = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("grf")) {
            if let Some(name) = path.to_str() {
                files.push(name.to_string());
            }
        }
    }
    files
}

struct SpriteData {
    spr: SprFile,
    act: ActFile,
}

fn load_sprite_data(grf: &GrfArchive, sprite_path: &str) -> Option<SpriteData> {
    let spr_data = match grf.read_file(sprite_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Failed to read SPR {sprite_path}: {e}");
            return None;
        }
    };
    let spr = match SprFile::parse(&spr_data) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to parse SPR: {e}");
            return None;
        }
    };

    let act_path = sprite_path
        .strip_suffix(".spr")
        .map(|p| format!("{p}.act"))
        .unwrap_or_else(|| sprite_path.replace(".spr", ".act"));

    let act_data = match grf.read_file(&act_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Failed to read ACT {act_path}: {e}");
            return None;
        }
    };
    let act = match ActFile::parse(&act_data) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Failed to parse ACT: {e}");
            return None;
        }
    };

    println!(
        "Loaded: {} indexed + {} rgba sprites, {} actions",
        spr.indexed_sprites.len(),
        spr.rgba_sprites.len(),
        act.actions.len(),
    );

    Some(SpriteData { spr, act })
}

#[derive(Clone, Copy, PartialEq)]
enum BrowserMode {
    Grf,
    Sprite,
}

struct App {
    window: Option<Arc<Window>>,
    device: Option<RenderDevice>,
    sprite_renderer: Option<SpriteRenderer>,
    sprite_textures: Option<SpriteTextures>,
    texture_cache: Option<TextureCache>,
    sprite_data: Option<SpriteData>,
    animation: AnimationState,
    background: Background,
    zoom: f32,
    pan: [f32; 2],
    shader_watcher: Option<ShaderWatcher>,
    last_frame: Instant,
    initial_grf_path: Option<String>,
    initial_sprite_path: Option<String>,
    grf: Option<GrfArchive>,
    font_atlas: Option<FontAtlas>,
    font_atlas_bind_group: Option<wgpu::BindGroup>,
    white_bind_group: Option<wgpu::BindGroup>,
    ui_renderer: Option<UiRenderer>,
    browser: Option<SpriteBrowser>,
    browser_mode: BrowserMode,
}

impl App {
    fn new(grf_path: Option<String>, sprite_path: Option<String>) -> Self {
        Self {
            window: None,
            device: None,
            sprite_renderer: None,
            sprite_textures: None,
            texture_cache: None,
            sprite_data: None,
            animation: AnimationState::new(),
            background: Background::Black,
            zoom: 2.0,
            pan: [0.0, 0.0],
            shader_watcher: None,
            last_frame: Instant::now(),
            initial_grf_path: grf_path,
            initial_sprite_path: sprite_path,
            grf: None,
            font_atlas: None,
            font_atlas_bind_group: None,
            white_bind_group: None,
            ui_renderer: None,
            browser: None,
            browser_mode: BrowserMode::Grf,
        }
    }

    fn open_grf(&mut self, path: &str) {
        let grf = match GrfArchive::open(Path::new(path)) {
            Ok(g) => g,
            Err(e) => {
                eprintln!("Failed to open GRF: {e}");
                return;
            }
        };

        let sprites: Vec<String> = grf.files_with_extension(".spr")
            .into_iter().map(|s| s.to_string()).collect();
        self.grf = Some(grf);

        if let Some(browser) = &mut self.browser {
            browser.set_items(sprites, "sprites");
        } else {
            self.browser = Some(SpriteBrowser::new(sprites, "sprites"));
        }
        if let Some(browser) = &mut self.browser {
            if let Some(device) = &self.device {
                browser.update_visible_rows(device.surface_config.height as f32);
            }
        }
        self.browser_mode = BrowserMode::Sprite;

        if let Some(window) = &self.window {
            window.set_title(&format!("Sprite Viewer — {path}"));
        }
    }

    fn load_sprite(&mut self, path: &str) {
        let (Some(device), Some(tex_cache), Some(grf)) = (
            &self.device, &self.texture_cache, &self.grf,
        ) else {
            return;
        };

        let sprite_data = match load_sprite_data(grf, path) {
            Some(d) => d,
            None => return,
        };

        let textures = upload_sprite_textures(
            &sprite_data.spr,
            &device.device,
            &device.queue,
            &tex_cache.bind_group_layout,
        );

        self.sprite_textures = Some(textures);
        self.sprite_data = Some(sprite_data);
        self.animation = AnimationState::new();

        if let Some(window) = &self.window {
            window.set_title(&format!("Sprite Viewer — {path}"));
        }
    }

    fn handle_action(&mut self, action: ViewerAction) {
        match action {
            ViewerAction::ToggleBrowser => {
                if self.browser_mode == BrowserMode::Sprite {
                    if let Some(browser) = &mut self.browser {
                        browser.open = !browser.open;
                    }
                }
            }
            ViewerAction::NextDirection => {
                let dir = (self.animation.direction + 1) % 8;
                self.animation.set_direction(dir);
            }
            ViewerAction::PrevDirection => {
                let dir = if self.animation.direction == 0 { 7 } else { self.animation.direction - 1 };
                self.animation.set_direction(dir);
            }
            ViewerAction::NextAction => {
                if let Some(data) = &self.sprite_data {
                    let next = self.animation.action + 1;
                    self.animation.set_action(next, &data.act);
                }
            }
            ViewerAction::PrevAction => {
                if let Some(data) = &self.sprite_data {
                    let prev = if self.animation.action == 0 {
                        data.act.actions.len() / 8
                    } else {
                        self.animation.action - 1
                    };
                    self.animation.set_action(prev, &data.act);
                }
            }
            ViewerAction::TogglePause => {
                self.animation.paused = !self.animation.paused;
            }
            ViewerAction::StepForward => {
                if self.animation.paused {
                    if let Some(data) = &self.sprite_data {
                        self.animation.step_forward(&data.act);
                    }
                }
            }
            ViewerAction::StepBackward => {
                if self.animation.paused {
                    if let Some(data) = &self.sprite_data {
                        self.animation.step_backward(&data.act);
                    }
                }
            }
            ViewerAction::ZoomIn => {
                self.zoom = (self.zoom * 1.2).min(20.0);
            }
            ViewerAction::ZoomOut => {
                self.zoom = (self.zoom / 1.2).max(0.25);
            }
            ViewerAction::CycleBackground => {
                self.background = self.background.next();
            }
        }
    }

    fn render_frame(&mut self) {
        let Some(device) = &self.device else { return };

        let width = device.surface_config.width as f32;
        let height = device.surface_config.height as f32;

        let output = match device.surface.get_current_texture() {
            Ok(tex) => tex,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                device.reconfigure_surface();
                return;
            }
            Err(wgpu::SurfaceError::Timeout) => return,
            Err(e) => {
                tracing::error!("Surface error: {e}");
                return;
            }
        };

        let view = output.texture.create_view(&Default::default());
        let mut encoder = device.device.create_command_encoder(&Default::default());

        if let (Some(renderer), Some(textures), Some(data)) = (
            &mut self.sprite_renderer, &self.sprite_textures, &self.sprite_data,
        ) {
            renderer.update_uniforms(&device.queue, &SpriteUniforms {
                screen_size: [width, height],
                zoom: self.zoom,
                _pad: 0.0,
                pan: self.pan,
                _pad2: [0.0, 0.0],
            });

            let action_idx = self.animation.action_index(&data.act);
            let motion = &data.act.actions[action_idx].motions[self.animation.motion_index % data.act.actions[action_idx].motions.len().max(1)];

            let screen_center = [width / (2.0 * self.zoom), height / (2.0 * self.zoom)];
            let mut batches: Vec<SpriteBatch> = Vec::new();

            for clip in &motion.clips {
                if let Some((vertices, indices, tex_idx)) = build_clip_quad(clip, textures, screen_center) {
                    if tex_idx < textures.bind_groups.len() {
                        batches.push(SpriteBatch {
                            vertices,
                            indices,
                            texture: &textures.bind_groups[tex_idx],
                        });
                    }
                }
            }

            renderer.render(
                &mut encoder,
                &view,
                &device.device,
                &device.queue,
                Some(self.background.clear_color()),
                &batches,
            );
        } else {
            encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("clear"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.background.clear_color()),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });
        }

        let browser_open = self.browser.as_ref().is_some_and(|b| b.open);
        if browser_open {
            if let (Some(browser), Some(atlas), Some(ui_renderer), Some(font_bg), Some(white_bg)) = (
                &self.browser, &self.font_atlas, &mut self.ui_renderer,
                &self.font_atlas_bind_group, &self.white_bind_group,
            ) {
                let draw_calls = browser.build_draw_calls(atlas, width, height);
                let resolved: Vec<UiDrawCommand> = draw_calls.iter()
                    .map(|call| {
                        let bind_group = match &call.texture {
                            UiTextureRef::FontAtlas => font_bg,
                            UiTextureRef::White => white_bg,
                            UiTextureRef::Named(_) => white_bg,
                        };
                        UiDrawCommand {
                            vertices: &call.vertices,
                            indices: &call.indices,
                            texture: bind_group,
                        }
                    })
                    .collect();

                ui_renderer.render(
                    &mut encoder,
                    &view,
                    &device.device,
                    &device.queue,
                    &resolved,
                );
            }
        }

        device.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    }

    fn browser_is_open(&self) -> bool {
        self.browser.as_ref().is_some_and(|b| b.open)
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attrs = WindowAttributes::default()
            .with_title("Sprite Viewer")
            .with_inner_size(winit::dpi::PhysicalSize::new(800u32, 600u32));

        let window = Arc::new(event_loop.create_window(attrs).unwrap());
        let device = block_on(RenderDevice::new(window.clone()));

        let tex_cache = TextureCache::new(&device.device);

        let shader_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../lib/renderer/src/shaders");
        let shader_source = std::fs::read_to_string(shader_dir.join("sprite.wgsl"))
            .expect("Failed to read sprite.wgsl");

        let sprite_renderer = SpriteRenderer::new(
            &device.device,
            device.surface_format,
            &tex_cache.bind_group_layout,
            device.surface_config.width,
            device.surface_config.height,
            &shader_source,
        );

        let watcher = ShaderWatcher::new(&shader_dir, "sprite.wgsl")
            .map_err(|e| tracing::warn!("Shader watcher unavailable: {e}"))
            .ok();

        let font_atlas = FontAtlas::from_embedded(16.0);
        let font_atlas_bind_group = texture::create_texture_bind_group_nearest(
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
        let ui_renderer = UiRenderer::new(
            &device.device,
            device.surface_format,
            &tex_cache.bind_group_layout,
            device.surface_config.width,
            device.surface_config.height,
        );

        let screen_h = device.surface_config.height as f32;

        self.texture_cache = Some(tex_cache);
        self.sprite_renderer = Some(sprite_renderer);
        self.shader_watcher = watcher;
        self.font_atlas = Some(font_atlas);
        self.font_atlas_bind_group = Some(font_atlas_bind_group);
        self.white_bind_group = Some(white_bind_group);
        self.ui_renderer = Some(ui_renderer);
        #[cfg(feature = "hot-reload")]
        {
            let win = window.clone();
            subsecond::register_handler(std::sync::Arc::new(move || win.request_redraw()));
        }

        self.device = Some(device);
        self.window = Some(window);

        if let Some(grf_path) = self.initial_grf_path.clone() {
            self.open_grf(&grf_path);
            if let Some(sprite_path) = self.initial_sprite_path.clone() {
                self.load_sprite(&sprite_path);
                if let Some(browser) = &mut self.browser {
                    browser.open = false;
                }
            }
        } else {
            let grf_files = scan_grf_files();
            if grf_files.is_empty() {
                eprintln!("No .grf files found in current directory. Use --grf <path> to specify.");
                event_loop.exit();
                return;
            }
            let mut browser = SpriteBrowser::new(grf_files, "GRF files");
            browser.update_visible_rows(screen_h);
            self.browser = Some(browser);
            self.browser_mode = BrowserMode::Grf;
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(device) = &mut self.device {
                    device.resize(size.width, size.height);
                }
                if let Some(ui_renderer) = &self.ui_renderer {
                    if let Some(device) = &self.device {
                        ui_renderer.resize(&device.queue, size.width, size.height);
                    }
                }
                if let Some(browser) = &mut self.browser {
                    browser.update_visible_rows(size.height as f32);
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if self.browser_is_open() {
                    if event.state != winit::event::ElementState::Pressed {
                        return;
                    }
                    match &event.logical_key {
                        Key::Named(NamedKey::Escape) | Key::Named(NamedKey::Tab) => {
                            // Can only dismiss browser when viewing sprites
                            if self.browser_mode == BrowserMode::Sprite {
                                if let Some(browser) = &mut self.browser {
                                    browser.open = false;
                                }
                            }
                        }
                        Key::Named(NamedKey::Enter) => {
                            let selected = self.browser.as_ref()
                                .and_then(|b| b.selected_item().map(|s| s.to_string()));
                            if let Some(path) = selected {
                                match self.browser_mode {
                                    BrowserMode::Grf => {
                                        self.open_grf(&path);
                                    }
                                    BrowserMode::Sprite => {
                                        if let Some(browser) = &mut self.browser {
                                            browser.open = false;
                                        }
                                        self.load_sprite(&path);
                                    }
                                }
                            }
                        }
                        key => {
                            if let Some(browser) = &mut self.browser {
                                match key {
                                    Key::Named(NamedKey::ArrowUp) => browser.handle_up(),
                                    Key::Named(NamedKey::ArrowDown) => browser.handle_down(),
                                    Key::Named(NamedKey::PageUp) => browser.handle_page_up(),
                                    Key::Named(NamedKey::PageDown) => browser.handle_page_down(),
                                    Key::Named(NamedKey::Backspace) => browser.handle_backspace(),
                                    Key::Character(ch) => {
                                        for c in ch.chars() {
                                            if !c.is_control() {
                                                browser.handle_char(c);
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                } else {
                    if let Some(action) = controls::map_key_press(
                        &event.logical_key,
                        event.state,
                    ) {
                        self.handle_action(action);
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                if !self.browser_is_open() {
                    if let Some(action) = controls::map_scroll(delta) {
                        self.handle_action(action);
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = (now - self.last_frame).as_secs_f32();
                self.last_frame = now;

                if let Some(data) = &self.sprite_data {
                    let act = &data.act;
                    self.animation.update(dt, act);
                }

                if let (Some(watcher), Some(renderer), Some(device), Some(tc)) = (
                    &self.shader_watcher,
                    &mut self.sprite_renderer,
                    &self.device,
                    &self.texture_cache,
                ) {
                    if let Some(new_source) = watcher.check_and_reload() {
                        renderer.recreate_pipeline(
                            &device.device,
                            device.surface_format,
                            &tc.bind_group_layout,
                            &new_source,
                        );
                    }
                }

                #[cfg(feature = "hot-reload")]
                subsecond::call(|| self.render_frame());
                #[cfg(not(feature = "hot-reload"))]
                self.render_frame();

                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }
}

fn block_on<F: std::future::Future>(future: F) -> F::Output {
    let mut future = std::pin::pin!(future);
    let waker = std::task::Waker::noop();
    let mut cx = std::task::Context::from_waker(&waker);
    loop {
        match future.as_mut().poll(&mut cx) {
            std::task::Poll::Ready(val) => return val,
            std::task::Poll::Pending => std::hint::spin_loop(),
        }
    }
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let args = parse_args();

    if args.list {
        let grf_path = args.grf_path.as_deref().unwrap_or_else(|| {
            eprintln!("--list requires --grf <path>");
            std::process::exit(1);
        });
        let grf = GrfArchive::open(Path::new(grf_path)).expect("Failed to open GRF");
        let mut sprites: Vec<&str> = grf.files_with_extension(".spr");
        sprites.sort();
        for name in &sprites {
            println!("{name}");
        }
        println!("\n{} sprite files found", sprites.len());
        return;
    }

    #[cfg(feature = "hot-reload")]
    dioxus_devtools::connect_subsecond();

    let event_loop = EventLoop::new().unwrap();
    let mut app = App::new(args.grf_path, args.sprite_path);
    event_loop.run_app(&mut app).unwrap();
}
