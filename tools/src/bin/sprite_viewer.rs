use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use ragnarok_formats::act::{SpriteActionType, SpriteAnimationState};
use ragnarok_formats::grf::GrfArchive;
use ragnarok_game::accessory_table::AccessoryTable;
use ragnarok_game::sprite_loader::{self as game_sprite_loader};
use ragnarok_game::sprite_path::weapon_view_id_to_type;
use ragnarok_renderer::font_atlas::FontAtlas;
use ragnarok_renderer::sprite::{
    SpriteRenderer, SpriteUniforms, EntitySprite,
    upload_sprite_textures, build_entity_sprite,
};
use ragnarok_renderer::texture::{self, TextureCache};
use ragnarok_renderer::ui_renderer::{UiDrawCommand, UiRenderer};
use ragnarok_renderer::{RenderDevice, UiTextureRef, block_on};
use ragnarok_tools::sprite_viewer::browser::{BrowserTab, SpriteBrowser};
use ragnarok_tools::sprite_viewer::controls::{self, Background, ViewerAction};
use ragnarok_tools::sprite_viewer::shader_watcher::ShaderWatcher;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowAttributes, WindowId};

struct Args {
    grf_path: Option<String>,
    list: bool,
}

fn parse_args() -> Args {
    let args: Vec<String> = std::env::args().collect();
    let mut grf_path = None;
    let mut list = false;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--grf" => {
                i += 1;
                if i < args.len() { grf_path = Some(args[i].clone()); }
            }
            "--list" => { list = true; }
            _ => {}
        }
        i += 1;
    }
    Args { grf_path, list }
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

struct App {
    window: Option<Arc<Window>>,
    device: Option<RenderDevice>,
    sprite_renderer: Option<SpriteRenderer>,
    texture_cache: Option<TextureCache>,
    entity_sprite: Option<EntitySprite>,
    animation: SpriteAnimationState,
    paused: bool,
    background: Background,
    zoom: f32,
    pan: [f32; 2],
    shader_watcher: Option<ShaderWatcher>,
    last_frame: Instant,
    initial_grf_path: Option<String>,
    grf: Option<GrfArchive>,
    font_atlas: Option<FontAtlas>,
    font_atlas_bind_group: Option<wgpu::BindGroup>,
    white_bind_group: Option<wgpu::BindGroup>,
    ui_renderer: Option<UiRenderer>,
    browser: Option<SpriteBrowser>,
    composite_job: u16,
    composite_sex: u8,
    composite_head: u16,
    weapon_view_id: u16,
    headgear_top_id: u16,
    shield_view_id: u16,
    accessory_table: AccessoryTable,
    is_composite: bool,
}

impl App {
    fn new(grf_path: Option<String>) -> Self {
        Self {
            window: None,
            device: None,
            sprite_renderer: None,
            texture_cache: None,
            entity_sprite: None,
            animation: SpriteAnimationState::new(0),
            paused: false,
            background: Background::Black,
            zoom: 2.0,
            pan: [0.0, 0.0],
            shader_watcher: None,
            last_frame: Instant::now(),
            initial_grf_path: grf_path,
            grf: None,
            font_atlas: None,
            font_atlas_bind_group: None,
            white_bind_group: None,
            ui_renderer: None,
            browser: None,
            composite_job: 0,
            composite_sex: 1,
            composite_head: 1,
            weapon_view_id: 0,
            headgear_top_id: 0,
            shield_view_id: 0,
            accessory_table: AccessoryTable::empty(),
            is_composite: false,
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
        self.accessory_table = AccessoryTable::load_from_grf(&grf);
        self.grf = Some(grf);

        let mut browser = SpriteBrowser::new_with_tabs(sprites, &self.accessory_table);
        if let Some(device) = &self.device {
            browser.update_visible_rows(device.surface_config.height as f32);
        }
        self.browser = Some(browser);

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

        let sprite_data = match game_sprite_loader::load_sprite_data_from_spr(grf, path) {
            Some(d) => d,
            None => return,
        };

        self.animation = SpriteAnimationState::new(0);
        self.is_composite = false;
        self.entity_sprite = Some(build_entity_sprite(
            &device.device, &device.queue, &tex_cache.bind_group_layout,
            sprite_data, None, None, None, None, None, None, None,
        ));

        if let Some(window) = &self.window {
            window.set_title(&format!("Sprite Viewer — {path}"));
        }
    }

    fn load_composite(&mut self, job: u16, sex: u8, head_id: u16, weapon_view_id: u16, headgear_top: u16, shield: u16) {
        let (Some(device), Some(tex_cache), Some(grf)) = (
            &self.device, &self.texture_cache, &self.grf,
        ) else {
            return;
        };

        let weapon_type = weapon_view_id_to_type(weapon_view_id);
        let data = match game_sprite_loader::load_player_sprite_data(grf, &self.accessory_table, job, sex, head_id, 0, 0, weapon_type, headgear_top, 0, 0, shield) {
            Some(d) => d,
            None => {
                eprintln!("Failed to load body sprite for job={job} sex={sex}");
                return;
            }
        };
        self.animation = SpriteAnimationState::new(0);
        self.is_composite = true;
        self.entity_sprite = Some(build_entity_sprite(
            &device.device, &device.queue, &tex_cache.bind_group_layout,
            data.body, data.head, data.weapon, data.headgear_top, data.headgear_mid, data.headgear_bottom, data.shield, None,
        ));
        self.composite_job = job;
        self.composite_sex = sex;
        self.composite_head = head_id;
        self.weapon_view_id = weapon_view_id;
        self.headgear_top_id = headgear_top;
        self.shield_view_id = shield;

        self.update_composite_title();
    }

    fn update_composite_title(&self) {
        if let Some(window) = &self.window {
            let weapon_str = weapon_view_id_to_type(self.weapon_view_id)
                .map(|w| format!("{w:?}"))
                .unwrap_or_else(|| "None".into());
            let hg_str = if self.headgear_top_id > 0 {
                self.accessory_table.get_suffix(self.headgear_top_id)
                    .unwrap_or("?").to_string()
            } else { "None".into() };
            let shield_str = if self.shield_view_id > 0 { format!("{}", self.shield_view_id) } else { "None".into() };
            window.set_title(&format!(
                "Sprite Viewer — job:{} sex:{} head:{} weapon:{weapon_str} headgear:{hg_str} shield:{shield_str}",
                self.composite_job, self.composite_sex, self.composite_head,
            ));
        }
    }

    fn reload_weapon(&mut self) {
        let (Some(device), Some(tex_cache), Some(grf)) = (
            &self.device, &self.texture_cache, &self.grf,
        ) else {
            return;
        };
        let Some(entity) = &mut self.entity_sprite else { return };

        let (weapon_textures, weapon_act) = if let Some(wt) = weapon_view_id_to_type(self.weapon_view_id) {
            if let Some(wd) = game_sprite_loader::load_weapon_sprite(grf, self.composite_job, self.composite_sex, wt) {
                let wtex = upload_sprite_textures(
                    &wd.images, wd.indexed_count,
                    &device.device, &device.queue, &tex_cache.bind_group_layout,
                );
                (Some(wtex), Some(wd.act))
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

        entity.weapon_textures = weapon_textures;
        entity.weapon_act = weapon_act;
        self.update_composite_title();
    }

    fn reload_headgear(&mut self) {
        let (Some(device), Some(tex_cache), Some(grf)) = (
            &self.device, &self.texture_cache, &self.grf,
        ) else {
            return;
        };
        let Some(entity) = &mut self.entity_sprite else { return };

        let (hg_textures, hg_act) = if self.headgear_top_id > 0 {
            if let Some(data) = game_sprite_loader::load_headgear_sprite(
                grf,
                self.accessory_table.get_suffix(self.headgear_top_id).unwrap_or(""),
                self.composite_sex,
            ) {
                let tex = upload_sprite_textures(
                    &data.images, data.indexed_count,
                    &device.device, &device.queue, &tex_cache.bind_group_layout,
                );
                (Some(tex), Some(data.act))
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

        entity.headgear_top_textures = hg_textures;
        entity.headgear_top_act = hg_act;
        self.update_composite_title();
    }

    fn reload_shield(&mut self) {
        let (Some(device), Some(tex_cache), Some(grf)) = (
            &self.device, &self.texture_cache, &self.grf,
        ) else {
            return;
        };
        let Some(entity) = &mut self.entity_sprite else { return };

        let (shield_textures, shield_act) = if self.shield_view_id > 0 {
            if let Some(data) = game_sprite_loader::load_shield_sprite(grf, self.shield_view_id, self.composite_job, self.composite_sex) {
                let tex = upload_sprite_textures(
                    &data.images, data.indexed_count,
                    &device.device, &device.queue, &tex_cache.bind_group_layout,
                );
                (Some(tex), Some(data.act))
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

        entity.shield_textures = shield_textures;
        entity.shield_act = shield_act;
        self.update_composite_title();
    }

    fn handle_browser_select(&mut self) {
        let active_tab = self.browser.as_ref().and_then(|b| b.active_tab());
        match active_tab {
            Some(BrowserTab::Character) => {
                let job_id = self.browser.as_ref().and_then(|b| b.selected_job_id());
                if let Some(job_id) = job_id {
                    if let Some(browser) = &mut self.browser {
                        browser.open = false;
                    }
                    self.composite_job = job_id;
                    let (job, sex, head, weapon) = (self.composite_job, self.composite_sex, self.composite_head, self.weapon_view_id);
                    self.load_composite(job, sex, head, weapon, self.headgear_top_id, self.shield_view_id);
                }
            }
            Some(BrowserTab::Headgear) => {
                let hg_id = self.browser.as_ref().and_then(|b| b.selected_headgear_id());
                if let Some(hg_id) = hg_id {
                    if let Some(browser) = &mut self.browser {
                        browser.open = false;
                    }
                    self.headgear_top_id = hg_id;
                    self.reload_headgear();
                }
            }
            Some(BrowserTab::Npc) | Some(BrowserTab::Monster) | Some(BrowserTab::Other) => {
                let selected = self.browser.as_ref()
                    .and_then(|b| b.selected_item().map(|s| s.to_string()));
                if let Some(path) = selected {
                    if let Some(browser) = &mut self.browser {
                        browser.open = false;
                    }
                    self.entity_sprite = None;
                    self.load_sprite(&path);
                }
            }
            None => {}
        }
    }

    fn handle_action(&mut self, action: ViewerAction) {
        match action {
            ViewerAction::ToggleBrowser => {
                if let Some(browser) = &mut self.browser {
                    if browser.has_tabs() {
                        browser.open = !browser.open;
                    }
                }
            }
            ViewerAction::NextDirection => {
                let dir = ((self.animation.direction() + 1) % 8) as u8;
                self.animation.set_direction(dir);
                self.animation.reset_motion();
            }
            ViewerAction::PrevDirection => {
                let dir = if self.animation.direction() == 0 { 7u8 } else { (self.animation.direction() - 1) as u8 };
                self.animation.set_direction(dir);
                self.animation.reset_motion();
            }
            ViewerAction::NextAction => {
                if let Some(entity) = &self.entity_sprite {
                    let next = self.animation.action() + 1;
                    self.animation.set_action_clamped(next, &entity.body_act);
                }
            }
            ViewerAction::PrevAction => {
                if let Some(entity) = &self.entity_sprite {
                    let prev = if self.animation.action() == 0 {
                        entity.body_act.actions.len() - 1
                    } else {
                        self.animation.action() - 1
                    };
                    self.animation.set_action_clamped(prev, &entity.body_act);
                }
            }
            ViewerAction::TogglePause => {
                self.paused = !self.paused;
            }
            ViewerAction::StepForward => {
                if self.paused {
                    if let Some(entity) = &self.entity_sprite {
                        let action_idx = self.animation.flat_action_index(&entity.body_act);
                        let motion_count = entity.body_act.actions[action_idx].motions.len();
                        self.animation.step_forward(motion_count);
                    }
                }
            }
            ViewerAction::StepBackward => {
                if self.paused {
                    if let Some(entity) = &self.entity_sprite {
                        let action_idx = self.animation.flat_action_index(&entity.body_act);
                        let motion_count = entity.body_act.actions[action_idx].motions.len();
                        self.animation.step_backward(motion_count);
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
            ViewerAction::NextWeapon => {
                if self.entity_sprite.is_some() {
                    self.weapon_view_id = if self.weapon_view_id >= 17 { 0 } else { self.weapon_view_id + 1 };
                    self.reload_weapon();
                }
            }
            ViewerAction::PrevWeapon => {
                if self.entity_sprite.is_some() {
                    self.weapon_view_id = if self.weapon_view_id == 0 { 17 } else { self.weapon_view_id - 1 };
                    self.reload_weapon();
                }
            }
            ViewerAction::ToggleSex => {
                if self.entity_sprite.is_some() {
                    self.composite_sex = if self.composite_sex == 0 { 1 } else { 0 };
                    let (job, sex, head, weapon) = (self.composite_job, self.composite_sex, self.composite_head, self.weapon_view_id);
                    self.load_composite(job, sex, head, weapon, self.headgear_top_id, self.shield_view_id);
                }
            }
            ViewerAction::NextHead => {
                if self.entity_sprite.is_some() {
                    self.composite_head = if self.composite_head >= 30 { 1 } else { self.composite_head + 1 };
                    let (job, sex, head, weapon) = (self.composite_job, self.composite_sex, self.composite_head, self.weapon_view_id);
                    self.load_composite(job, sex, head, weapon, self.headgear_top_id, self.shield_view_id);
                }
            }
            ViewerAction::PrevHead => {
                if self.entity_sprite.is_some() {
                    self.composite_head = if self.composite_head <= 1 { 30 } else { self.composite_head - 1 };
                    let (job, sex, head, weapon) = (self.composite_job, self.composite_sex, self.composite_head, self.weapon_view_id);
                    self.load_composite(job, sex, head, weapon, self.headgear_top_id, self.shield_view_id);
                }
            }
            ViewerAction::NextHeadgear => {
                if self.entity_sprite.is_some() {
                    self.headgear_top_id = self.accessory_table.next_id(self.headgear_top_id);
                    self.reload_headgear();
                }
            }
            ViewerAction::PrevHeadgear => {
                if self.entity_sprite.is_some() {
                    self.headgear_top_id = self.accessory_table.prev_id(self.headgear_top_id);
                    self.reload_headgear();
                }
            }
            ViewerAction::NextShield => {
                if self.entity_sprite.is_some() {
                    self.shield_view_id = if self.shield_view_id >= 4 { 0 } else { self.shield_view_id + 1 };
                    self.reload_shield();
                }
            }
            ViewerAction::PrevShield => {
                if self.entity_sprite.is_some() {
                    self.shield_view_id = if self.shield_view_id == 0 { 4 } else { self.shield_view_id - 1 };
                    self.reload_shield();
                }
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

        if let Some(renderer) = &mut self.sprite_renderer {
            if let Some(entity) = &self.entity_sprite {
                renderer.update_uniforms(&device.queue, &SpriteUniforms {
                    screen_size: [width, height],
                    zoom: 1.0,
                    _pad: 0.0,
                    pan: [0.0, 0.0],
                    _pad2: [0.0, 0.0],
                });

                let screen_center = [width / 2.0 + self.pan[0] * self.zoom, height / 2.0 + self.pan[1] * self.zoom];
                let batches = entity.build_batches(&self.animation, None, 0, screen_center, 0.0, self.zoom, 0.0);

                renderer.render(
                    &mut encoder,
                    &view,
                    Some(&device.depth_view),
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
        let mut ui_draw_calls: Option<Vec<_>> = if browser_open {
            self.browser.as_ref()
                .zip(self.font_atlas.as_ref())
                .map(|(browser, atlas)| browser.build_draw_calls(atlas, width, height))
        } else {
            self.font_atlas.as_ref()
                .map(|atlas| controls::build_legend_draw_calls(atlas, height))
        };

        if let (Some(entity), Some(atlas)) = (&self.entity_sprite, &self.font_atlas) {
            let action_idx = self.animation.flat_action_index(&entity.body_act);
            let motion_count = entity.body_act.actions[action_idx].motions.len();
            let status = controls::build_status_draw_calls(
                atlas, width,
                self.animation.action(),
                self.animation.direction(),
                self.animation.motion_index(),
                motion_count,
                self.paused,
            );
            ui_draw_calls.get_or_insert_with(Vec::new).extend(status);
        }

        if let (Some(draw_calls), Some(ui_renderer), Some(font_bg), Some(white_bg)) = (
            ui_draw_calls, &mut self.ui_renderer,
            &self.font_atlas_bind_group, &self.white_bind_group,
        ) {
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
                    let has_tabs = self.browser.as_ref().is_some_and(|b| b.has_tabs());
                    match &event.logical_key {
                        Key::Named(NamedKey::Escape) | Key::Named(NamedKey::Tab) => {
                            if has_tabs {
                                if let Some(browser) = &mut self.browser {
                                    browser.open = false;
                                }
                            }
                        }
                        Key::Named(NamedKey::Enter) => {
                            if has_tabs {
                                self.handle_browser_select();
                            } else {
                                let selected = self.browser.as_ref()
                                    .and_then(|b| b.selected_item().map(|s| s.to_string()));
                                if let Some(path) = selected {
                                    self.open_grf(&path);
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
                                        if has_tabs {
                                            match ch.as_str() {
                                                "1" => browser.switch_tab(BrowserTab::Npc),
                                                "2" => browser.switch_tab(BrowserTab::Monster),
                                                "3" => browser.switch_tab(BrowserTab::Character),
                                                "4" => browser.switch_tab(BrowserTab::Headgear),
                                                "5" => browser.switch_tab(BrowserTab::Other),
                                                _ => {
                                                    for c in ch.chars() {
                                                        if !c.is_control() {
                                                            browser.handle_char(c);
                                                        }
                                                    }
                                                }
                                            }
                                        } else {
                                            for c in ch.chars() {
                                                if !c.is_control() {
                                                    browser.handle_char(c);
                                                }
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

                if !self.paused {
                    if let Some(entity) = &self.entity_sprite {
                        let animated = !self.is_composite || SpriteActionType::from_index(self.animation.action())
                            .is_none_or(|a| a.is_animated());
                        if animated {
                            self.animation.update_flat(dt, &entity.body_act);
                        }
                    }
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
    let mut app = App::new(args.grf_path);
    event_loop.run_app(&mut app).unwrap();
}
