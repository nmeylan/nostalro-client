use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use ragnarok_formats::grf::GrfArchive;
use ragnarok_game::damage_number::DamageNumberQuad;
use ragnarok_game::sprite_loader;
use ragnarok_renderer::font_atlas::FontAtlas;
use ragnarok_renderer::sprite::{SpriteTextures, upload_sprite_textures};
use ragnarok_renderer::texture::{self, TextureCache};
use ragnarok_renderer::ui_renderer::{UiDrawCommand, UiRenderer};
use ragnarok_renderer::{RenderDevice, UiDrawCall, UiTextureRef, block_on, render_damage_number_quads};
use ragnarok_tools::rendering_viewer::controls::{
    self, Background, Scenario, ViewerAction,
};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

use std::path::PathBuf;
use std::time::SystemTime;

// --- Hot-reload dylib loading ---

type HotCreateFn = extern "C" fn() -> *mut ();
type HotDestroyFn = unsafe extern "C" fn(*mut ());
type HotTriggerFn = unsafe extern "C" fn(*mut (), u8, i32, u8);
type HotUpdateFn = unsafe extern "C" fn(*mut (), f32);
type HotBuildFn = unsafe extern "C" fn(*mut (), *mut Vec<DamageNumberQuad>);
type HotInitSpritesFn = unsafe extern "C" fn(*mut (), *const u8, usize, *const (u32, u32), usize, usize, *const (u32, u32), usize);

struct HotLib {
    _lib: libloading::Library,
    state: *mut (),
    update_fn: HotUpdateFn,
    trigger_fn: HotTriggerFn,
    build_fn: HotBuildFn,
    destroy_fn: HotDestroyFn,
    init_sprites_fn: HotInitSpritesFn,
}

impl HotLib {
    fn load(dylib_path: &Path) -> Option<Self> {
        let lib = match unsafe { libloading::Library::new(dylib_path) } {
            Ok(lib) => lib,
            Err(e) => {
                eprintln!("Failed to load dylib: {e}");
                return None;
            }
        };

        let (create_fn, update_fn, trigger_fn, build_fn, destroy_fn, init_sprites_fn) = unsafe {
            let create: libloading::Symbol<HotCreateFn> = lib.get(b"hot_create").ok()?;
            let update: libloading::Symbol<HotUpdateFn> = lib.get(b"hot_update").ok()?;
            let trigger: libloading::Symbol<HotTriggerFn> = lib.get(b"hot_trigger").ok()?;
            let build: libloading::Symbol<HotBuildFn> = lib.get(b"hot_build").ok()?;
            let destroy: libloading::Symbol<HotDestroyFn> = lib.get(b"hot_destroy").ok()?;
            let init_sprites: libloading::Symbol<HotInitSpritesFn> = lib.get(b"hot_init_sprites").ok()?;
            (*create, *update, *trigger, *build, *destroy, *init_sprites)
        };

        let state = (create_fn)();
        Some(Self { _lib: lib, state, update_fn, trigger_fn, build_fn, destroy_fn, init_sprites_fn })
    }

    fn unload(mut self) {
        if !self.state.is_null() {
            unsafe { (self.destroy_fn)(self.state) };
            self.state = std::ptr::null_mut();
        }
    }

    fn update(&self, dt: f32) {
        unsafe { (self.update_fn)(self.state, dt) };
    }

    fn trigger(&self, scenario: u8, damage_value: i32, direction: u8) {
        unsafe { (self.trigger_fn)(self.state, scenario, damage_value, direction) };
    }

    fn build(&self, out: &mut Vec<DamageNumberQuad>) {
        unsafe { (self.build_fn)(self.state, out as *mut Vec<DamageNumberQuad>) };
    }

    fn init_sprites(
        &self,
        act_data: &[u8],
        num_sizes: &[(u32, u32)],
        num_indexed_count: usize,
        msg_sizes: Option<&[(u32, u32)]>,
    ) {
        let (msg_ptr, msg_len) = match msg_sizes {
            Some(s) => (s.as_ptr(), s.len()),
            None => (std::ptr::null(), 0),
        };
        unsafe {
            (self.init_sprites_fn)(
                self.state,
                act_data.as_ptr(),
                act_data.len(),
                num_sizes.as_ptr(),
                num_sizes.len(),
                num_indexed_count,
                msg_ptr,
                msg_len,
            )
        };
    }
}

fn find_dylib() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let target_dir = manifest_dir
        .parent().unwrap()
        .join("target")
        .join("debug");

    #[cfg(target_os = "linux")]
    let name = "librendering_viewer_hot.so";
    #[cfg(target_os = "macos")]
    let name = "librendering_viewer_hot.dylib";
    #[cfg(target_os = "windows")]
    let name = "rendering_viewer_hot.dll";

    target_dir.join(name)
}

fn dylib_mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).ok()?.modified().ok()
}

struct App {
    window: Option<Arc<Window>>,
    device: Option<RenderDevice>,
    texture_cache: Option<TextureCache>,
    ui_renderer: Option<UiRenderer>,
    font_atlas: Option<FontAtlas>,
    font_atlas_bind_group: Option<wgpu::BindGroup>,
    white_bind_group: Option<wgpu::BindGroup>,

    num_textures: Option<SpriteTextures>,
    num_act_data: Option<Vec<u8>>,
    msg_textures: Option<SpriteTextures>,

    paused: bool,
    speed: f32,
    last_frame: Instant,

    damage_value: i32,
    direction: u8,
    background: Background,
    grf_path: Option<String>,

    hot_lib: Option<HotLib>,
    dylib_path: PathBuf,
    last_dylib_mtime: SystemTime,
    reload_counter: u64,
}

// Fixed screen positions for each scenario entity_id (1-9)
const GRID_COLS: usize = 5;
const GRID_CELL_W: f32 = 160.0;
const GRID_CELL_H: f32 = 120.0;
const GRID_OFFSET_X: f32 = 230.0;
const GRID_OFFSET_Y: f32 = 80.0;

fn entity_screen_pos(entity_id: u32) -> (f32, f32) {
    let idx = (entity_id.saturating_sub(1)) as usize;
    let col = idx % GRID_COLS;
    let row = idx / GRID_COLS;
    let x = GRID_OFFSET_X + col as f32 * GRID_CELL_W + GRID_CELL_W / 2.0;
    let y = GRID_OFFSET_Y + row as f32 * GRID_CELL_H + GRID_CELL_H / 2.0;
    (x, y)
}

const SCENARIO_LABELS: &[(u32, &str)] = &[
    (1, "Normal"),
    (2, "Skill"),
    (3, "Critical"),
    (4, "Damage to Self"),
    (5, "Skill Multi"),
    (6, "Normal Multi"),
    (7, "Heal"),
    (8, "Miss"),
    (9, "Lucky"),
];

fn scenario_to_u8(s: Scenario) -> u8 {
    match s {
        Scenario::NormalAttack => 1,
        Scenario::SkillAttack => 2,
        Scenario::CriticalHit => 3,
        Scenario::PlayerDamage => 4,
        Scenario::SkillMultiHit => 5,
        Scenario::NormalMultiHit => 6,
        Scenario::Heal => 7,
        Scenario::Miss => 8,
        Scenario::LuckyDodge => 9,
        Scenario::All => 0,
    }
}

impl App {
    fn new(grf_path: Option<String>) -> Self {
        let dylib_path = find_dylib();
        let last_dylib_mtime = dylib_mtime(&dylib_path).unwrap_or(SystemTime::UNIX_EPOCH);
        let hot_lib = HotLib::load(&dylib_path);

        Self {
            window: None,
            device: None,
            texture_cache: None,
            ui_renderer: None,
            font_atlas: None,
            font_atlas_bind_group: None,
            white_bind_group: None,
            num_textures: None,
            num_act_data: None,
            msg_textures: None,
            paused: false,
            speed: 1.0,
            last_frame: Instant::now(),
            damage_value: 1234,
            direction: 0,
            background: Background::Black,
            grf_path,
            hot_lib,
            dylib_path,
            last_dylib_mtime,
            reload_counter: 0,
        }
    }

    fn load_damage_sprites(&mut self, grf: &GrfArchive) {
        let (device, tex_cache) = match (&self.device, &self.texture_cache) {
            (Some(d), Some(tc)) => (d, tc),
            _ => return,
        };

        if let Some(sprite_data) = sprite_loader::load_damage_number_sprite(grf) {
            self.num_textures = Some(upload_sprite_textures(
                &sprite_data.images,
                sprite_data.indexed_count,
                &device.device,
                &device.queue,
                &tex_cache.bind_group_layout,
            ));
        }
        if let Some(sprite_data) = sprite_loader::load_damage_miss_msg_sprite(grf) {
            self.msg_textures = Some(upload_sprite_textures(
                &sprite_data.images,
                sprite_data.indexed_count,
                &device.device,
                &device.queue,
                &tex_cache.bind_group_layout,
            ));
        }
    }

    fn handle_action(&mut self, action: ViewerAction) {
        match action {
            ViewerAction::TriggerScenario(s) => {
                if let Some(hot) = &self.hot_lib {
                    hot.trigger(scenario_to_u8(s), self.damage_value, self.direction);
                }
            }
            ViewerAction::TogglePause => self.paused = !self.paused,
            ViewerAction::Restart => {
                if let Some(old) = self.hot_lib.take() {
                    old.unload();
                }
                self.hot_lib = HotLib::load(&self.dylib_path);
                self.hot_init_sprites();
                if let Some(hot) = &self.hot_lib {
                    hot.trigger(0, self.damage_value, self.direction);
                }
            }
            ViewerAction::IncreaseValue => {
                self.damage_value = (self.damage_value + 100).min(999999);
            }
            ViewerAction::DecreaseValue => {
                self.damage_value = (self.damage_value - 100).max(1);
            }
            ViewerAction::NextDirection => {
                self.direction = (self.direction + 1) % 8;
            }
            ViewerAction::PrevDirection => {
                self.direction = if self.direction == 0 { 7 } else { self.direction - 1 };
            }
            ViewerAction::SpeedUp => {
                self.speed = (self.speed + 0.25).min(5.0);
            }
            ViewerAction::SpeedDown => {
                self.speed = (self.speed - 0.25).max(0.25);
            }
            ViewerAction::CycleBackground => {
                self.background = self.background.next();
            }
        }
    }

    fn check_hot_reload(&mut self) {
        if let Some(mtime) = dylib_mtime(&self.dylib_path) {
            if mtime > self.last_dylib_mtime {
                self.last_dylib_mtime = mtime;
                self.reload_counter += 1;
                let ext = format!("hot{}.so", self.reload_counter);
                let tmp_path = self.dylib_path.with_extension(&ext);
                if std::fs::copy(&self.dylib_path, &tmp_path).is_err() {
                    eprintln!("Failed to copy dylib to temp file");
                    return;
                }

                eprintln!("Reloading dylib...");

                if let Some(old) = self.hot_lib.take() {
                    old.unload();
                }

                match HotLib::load(&tmp_path) {
                    Some(new_lib) => {
                        self.hot_lib = Some(new_lib);
                        self.hot_init_sprites();
                        if let Some(hot) = &self.hot_lib {
                            hot.trigger(0, self.damage_value, self.direction);
                        }
                        eprintln!("Reload complete.");
                    }
                    None => {
                        eprintln!("Failed to load new dylib, falling back to original");
                        self.hot_lib = HotLib::load(&self.dylib_path);
                        self.hot_init_sprites();
                    }
                }

                if self.reload_counter > 1 {
                    let prev_ext = format!("hot{}.so", self.reload_counter - 1);
                    let prev = self.dylib_path.with_extension(prev_ext);
                    let _ = std::fs::remove_file(prev);
                }
            }
        }
    }

    fn hot_init_sprites(&self) {
        if let (Some(hot), Some(num_tex), Some(act_data)) = (&self.hot_lib, &self.num_textures, &self.num_act_data) {
            hot.init_sprites(
                act_data,
                &num_tex.sizes,
                num_tex.indexed_count,
                self.msg_textures.as_ref().map(|t| t.sizes.as_slice()),
            );
        }
    }

    fn render_frame(&mut self) {
        if self.device.is_none() { return; }

        self.check_hot_reload();

        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f32();
        self.last_frame = now;

        if !self.paused {
            let scaled_dt = dt * self.speed;
            if let Some(hot) = &self.hot_lib {
                hot.update(scaled_dt);
            }
        }

        let device = self.device.as_ref().unwrap();
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

        // Clear pass
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

        let mut draw_calls: Vec<UiDrawCall> = Vec::new();
        let mut inline_textures: Vec<&wgpu::BindGroup> = Vec::new();

        // Build damage number quads
        let quads: Vec<DamageNumberQuad> = {
            let mut q = Vec::new();
            if let Some(hot) = &self.hot_lib {
                hot.build(&mut q);
            }
            q
        };

        if let Some(num_tex) = &self.num_textures {
            render_damage_number_quads(
                &quads,
                num_tex,
                self.msg_textures.as_ref(),
                &mut draw_calls,
                &mut inline_textures,
            );
        }

        // Labels for each entity position
        if let Some(atlas) = &self.font_atlas {
            for &(entity_id, label) in SCENARIO_LABELS {
                let (sx, sy) = entity_screen_pos(entity_id);
                let label_w = atlas.measure_text(label);
                let label_x = sx - label_w / 2.0;
                let label_y = sy + 30.0;
                let (tv, ti) = ragnarok_ui::draw::text_vertices(
                    label, label_x, label_y, [0.7, 0.7, 0.7, 0.6], atlas,
                );
                draw_calls.push(UiDrawCall {
                    vertices: tv, indices: ti, texture: UiTextureRef::FontAtlas,
                });
            }

            let mut legend = controls::build_legend_draw_calls(atlas, height);
            draw_calls.append(&mut legend);

            let mut status = controls::build_status_draw_calls(
                atlas, width, self.damage_value, self.direction, self.speed, self.paused,
            );
            draw_calls.append(&mut status);
        }

        // Resolve textures and render
        if let (Some(ui_renderer), Some(font_bg), Some(white_bg)) = (
            &mut self.ui_renderer, &self.font_atlas_bind_group, &self.white_bind_group,
        ) {
            let resolved: Vec<UiDrawCommand> = draw_calls.iter()
                .map(|call| {
                    let bind_group = match &call.texture {
                        UiTextureRef::FontAtlas => font_bg,
                        UiTextureRef::White => white_bg,
                        UiTextureRef::Named(_) => white_bg,
                        UiTextureRef::Inline(idx) => {
                            inline_textures.get(*idx).copied().unwrap_or(white_bg)
                        }
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

        device.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attrs = WindowAttributes::default()
            .with_title("Rendering Viewer")
            .with_inner_size(winit::dpi::PhysicalSize::new(1024u32, 600u32));

        let window = Arc::new(event_loop.create_window(attrs).unwrap());
        let device = block_on(RenderDevice::new(window.clone()));
        let tex_cache = TextureCache::new(&device.device, 1.0);

        let font_atlas = FontAtlas::from_embedded(14.0, 1.0);
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
        let ui_renderer = UiRenderer::new(
            &device.device,
            device.surface_format,
            &tex_cache.bind_group_layout,
            device.surface_config.width as f32,
            device.surface_config.height as f32,
        );

        self.font_atlas = Some(font_atlas);
        self.font_atlas_bind_group = Some(font_atlas_bind_group);
        self.white_bind_group = Some(white_bind_group);
        self.ui_renderer = Some(ui_renderer);
        self.texture_cache = Some(tex_cache);


        self.last_frame = Instant::now();
        self.device = Some(device);
        self.window = Some(window);

        // Load GRF and damage sprites
        if let Some(grf_path) = &self.grf_path.clone() {
            match GrfArchive::open(Path::new(grf_path)) {
                Ok(grf) => {
                    self.load_damage_sprites(&grf);
                    if let Ok(act_data) = grf.read_file("data/sprite/이팩트/숫자.act") {
                        self.num_act_data = Some(act_data);
                    }
                    self.hot_init_sprites();
                    if let Some(hot) = &self.hot_lib {
                        hot.trigger(0, self.damage_value, self.direction);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to open GRF {grf_path}: {e}");
                    event_loop.exit();
                }
            }
        } else {
            eprintln!("No GRF specified. Use --grf <path>");
            event_loop.exit();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(device) = &mut self.device {
                    device.resize(size.width, size.height);
                    if let Some(ui_renderer) = &mut self.ui_renderer {
                        ui_renderer.resize(
                            &device.queue,
                            size.width as f32,
                            size.height as f32,
                        );
                    }
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let Some(action) = controls::map_key_press(&event.logical_key, event.state) {
                    self.handle_action(action);
                }
            }
            WindowEvent::RedrawRequested => {
                self.render_frame();

                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => {}
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

    let args: Vec<String> = std::env::args().collect();
    let mut grf_path = None;
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--grf" {
            i += 1;
            if i < args.len() {
                grf_path = Some(args[i].clone());
            }
        }
        i += 1;
    }


    let event_loop = EventLoop::new().unwrap();
    let mut app = App::new(grf_path);
    event_loop.run_app(&mut app).unwrap();
}
