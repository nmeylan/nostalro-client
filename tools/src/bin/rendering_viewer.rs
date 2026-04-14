use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use ragnarok_formats::act::ActFile;
use ragnarok_formats::grf::GrfArchive;
use ragnarok_game::damage_number::{DamageEvent, DamageNumberManager};
use ragnarok_game::sprite_loader;
use ragnarok_renderer::damage_number::DamageNumberEntry;
use ragnarok_renderer::font_atlas::FontAtlas;
use ragnarok_renderer::sprite::{SpriteTextures, upload_sprite_textures};
use ragnarok_renderer::texture::{self, TextureCache};
use ragnarok_renderer::ui_renderer::{UiDrawCommand, UiRenderer};
use ragnarok_renderer::{RenderDevice, UiDrawCall, UiTextureRef, block_on, render_damage_numbers};
use ragnarok_tools::rendering_viewer::controls::{
    self, Background, Scenario, ViewerAction,
};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

struct ScheduledHit {
    delay: f32,
    entity_id: u32,
    event: DamageEvent,
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
    num_act: Option<ActFile>,
    msg_textures: Option<SpriteTextures>,

    damage_numbers: DamageNumberManager,
    scheduled_hits: Vec<ScheduledHit>,
    paused: bool,
    speed: f32,
    last_frame: Instant,

    damage_value: i32,
    direction: u8,
    background: Background,
    grf_path: Option<String>,
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

impl App {
    fn new(grf_path: Option<String>) -> Self {
        Self {
            window: None,
            device: None,
            texture_cache: None,
            ui_renderer: None,
            font_atlas: None,
            font_atlas_bind_group: None,
            white_bind_group: None,
            num_textures: None,
            num_act: None,
            msg_textures: None,
            damage_numbers: DamageNumberManager::new(),
            scheduled_hits: Vec::new(),
            paused: false,
            speed: 1.0,
            last_frame: Instant::now(),
            damage_value: 1234,
            direction: 0,
            background: Background::Black,
            grf_path,
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
            self.num_act = Some(sprite_data.act);
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

    fn trigger_scenario(&mut self, scenario: Scenario) {
        match scenario {
            Scenario::NormalAttack => {
                self.damage_numbers.emit(1, self.direction, &DamageEvent {
                    damage: self.damage_value,
                    is_critical: false,
                    is_skill: false,
                    is_multi_hit: false,
                    is_player_target: false,
                    hit_index: 0,
                    is_last_hit: true,
                });
            }
            Scenario::SkillAttack => {
                self.damage_numbers.emit(2, self.direction, &DamageEvent {
                    damage: self.damage_value,
                    is_critical: false,
                    is_skill: true,
                    is_multi_hit: false,
                    is_player_target: false,
                    hit_index: 0,
                    is_last_hit: true,
                });
            }
            Scenario::CriticalHit => {
                self.damage_numbers.emit(3, self.direction, &DamageEvent {
                    damage: self.damage_value,
                    is_critical: true,
                    is_skill: false,
                    is_multi_hit: false,
                    is_player_target: false,
                    hit_index: 0,
                    is_last_hit: true,
                });
            }
            Scenario::PlayerDamage => {
                self.damage_numbers.emit(4, self.direction, &DamageEvent {
                    damage: self.damage_value,
                    is_critical: false,
                    is_skill: false,
                    is_multi_hit: false,
                    is_player_target: true,
                    hit_index: 0,
                    is_last_hit: true,
                });
            }
            Scenario::SkillMultiHit => {
                let per_hit = self.damage_value / 3;
                let delay = 0.2;
                for i in 0..3u16 {
                    self.scheduled_hits.push(ScheduledHit {
                        delay: delay * i as f32,
                        entity_id: 5,
                        event: DamageEvent {
                            damage: per_hit,
                            is_critical: false,
                            is_skill: true,
                            is_multi_hit: true,
                            is_player_target: false,
                            hit_index: i,
                            is_last_hit: i == 2,
                        },
                    });
                }
            }
            Scenario::NormalMultiHit => {
                let per_hit = self.damage_value / 3;
                let delay = 0.2;
                for i in 0..3u16 {
                    self.scheduled_hits.push(ScheduledHit {
                        delay: delay * i as f32,
                        entity_id: 6,
                        event: DamageEvent {
                            damage: per_hit,
                            is_critical: false,
                            is_skill: false,
                            is_multi_hit: true,
                            is_player_target: false,
                            hit_index: i,
                            is_last_hit: i == 2,
                        },
                    });
                }
            }
            Scenario::Heal => {
                self.damage_numbers.emit(7, self.direction, &DamageEvent {
                    damage: -(self.damage_value),
                    is_critical: false,
                    is_skill: false,
                    is_multi_hit: false,
                    is_player_target: false,
                    hit_index: 0,
                    is_last_hit: true,
                });
            }
            Scenario::Miss => {
                self.damage_numbers.emit(8, self.direction, &DamageEvent {
                    damage: 0,
                    is_critical: false,
                    is_skill: false,
                    is_multi_hit: false,
                    is_player_target: false,
                    hit_index: 0,
                    is_last_hit: true,
                });
            }
            Scenario::LuckyDodge => {
                self.damage_numbers.add(ragnarok_game::damage_number::DamageNumber::new(
                    9, 0, ragnarok_game::damage_number::DamageNumberType::Lucky, self.direction,
                ));
            }
            Scenario::All => {
                self.trigger_scenario(Scenario::NormalAttack);
                self.trigger_scenario(Scenario::SkillAttack);
                self.trigger_scenario(Scenario::CriticalHit);
                self.trigger_scenario(Scenario::PlayerDamage);
                self.trigger_scenario(Scenario::SkillMultiHit);
                self.trigger_scenario(Scenario::NormalMultiHit);
                self.trigger_scenario(Scenario::Heal);
                self.trigger_scenario(Scenario::Miss);
                self.trigger_scenario(Scenario::LuckyDodge);
            }
        }
    }

    fn handle_action(&mut self, action: ViewerAction) {
        match action {
            ViewerAction::TriggerScenario(s) => self.trigger_scenario(s),
            ViewerAction::TogglePause => self.paused = !self.paused,
            ViewerAction::Restart => {
                self.damage_numbers = DamageNumberManager::new();
                self.scheduled_hits.clear();
                self.trigger_scenario(Scenario::All);
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

    fn process_scheduled_hits(&mut self, dt: f32) {
        let mut ready = Vec::new();
        self.scheduled_hits.retain_mut(|hit| {
            hit.delay -= dt;
            if hit.delay <= 0.0 {
                ready.push((hit.entity_id, std::mem::replace(&mut hit.event, DamageEvent {
                    damage: 0, is_critical: false, is_skill: false,
                    is_multi_hit: false, is_player_target: false,
                    hit_index: 0, is_last_hit: false,
                })));
                false
            } else {
                true
            }
        });
        for (entity_id, event) in ready {
            self.damage_numbers.emit(entity_id, self.direction, &event);
        }
    }

    fn render_frame(&mut self) {
        if self.device.is_none() { return; }

        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f32();
        self.last_frame = now;

        if !self.paused {
            let scaled_dt = dt * self.speed;
            self.damage_numbers.update(scaled_dt);
            self.process_scheduled_hits(scaled_dt);
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

        // Build damage number entries
        let entries: Vec<DamageNumberEntry> = self.damage_numbers.numbers.iter()
            .filter_map(|dmg| {
                let (sx, sy) = entity_screen_pos(dmg.entity_id);
                let data = dmg.render_data()?;
                Some(DamageNumberEntry {
                    screen_x: sx,
                    screen_y: sy,
                    digits: data.digits,
                    digit_x_offsets: data.digit_x_offsets,
                    sprite_action: data.sprite_action,
                    color: data.color,
                    zoom: data.zoom,
                    y_offset: data.y_offset,
                    x_offset: data.x_offset,
                    uses_msg_sprite: data.uses_msg_sprite,
                    msg_frames: data.msg_frames,
                    is_critical: data.is_critical,
                })
            })
            .collect();

        if let (Some(num_tex), Some(num_act)) = (&self.num_textures, &self.num_act) {
            render_damage_numbers(
                &entries,
                num_tex,
                num_act,
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

        #[cfg(feature = "hot-reload")]
        {
            let win = window.clone();
            subsecond::register_handler(Arc::new(move || win.request_redraw()));
        }

        self.last_frame = Instant::now();
        self.device = Some(device);
        self.window = Some(window);

        // Load GRF and damage sprites
        if let Some(grf_path) = &self.grf_path.clone() {
            match GrfArchive::open(Path::new(grf_path)) {
                Ok(grf) => {
                    self.load_damage_sprites(&grf);
                    // Trigger all scenarios on startup
                    self.trigger_scenario(Scenario::All);
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

    #[cfg(feature = "hot-reload")]
    dioxus_devtools::connect_subsecond();

    let event_loop = EventLoop::new().unwrap();
    let mut app = App::new(grf_path);
    event_loop.run_app(&mut app).unwrap();
}
