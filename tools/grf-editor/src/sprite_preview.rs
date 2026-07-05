use eframe::egui;
use ragnarok_formats::act::{MotionType, SpriteActionType, SpriteAnimationState};
use ragnarok_formats::grf::GrfArchive;
use ragnarok_game::sprite_loader;
use ragnarok_renderer::wgpu;
use ragnarok_renderer::{
    Camera, EntitySprite, SpriteRenderer, SpriteUniforms, StrEffectCache, StrEmitterInput,
    TextureCache, block_on, build_entity_sprite, build_str_effect_batches,
};

const CANVAS: u32 = 384;
const BYTES_PER_PIXEL: u32 = 4;
const STR_EFFECT_PREFIX: &str = "data/texture/effect/";

enum Content {
    None,
    Sprite,
    Str,
}

#[derive(Clone, Copy, PartialEq)]
enum Background {
    Black,
    White,
    Checkerboard,
}

impl Background {
    fn next(self) -> Self {
        match self {
            Background::Black => Background::White,
            Background::White => Background::Checkerboard,
            Background::Checkerboard => Background::Black,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Background::Black => "BG: Black",
            Background::White => "BG: White",
            Background::Checkerboard => "BG: Gray",
        }
    }

    fn clear_color(self) -> wgpu::Color {
        match self {
            Background::Black => wgpu::Color::BLACK,
            Background::White => wgpu::Color::WHITE,
            Background::Checkerboard => wgpu::Color {
                r: 0.25,
                g: 0.25,
                b: 0.25,
                a: 1.0,
            },
        }
    }
}

/// Headless wgpu-28 renderer that draws the selected `.spr`/`.act` or `.str`
/// off-screen and hands the readback pixels to egui. Kept in a separate wgpu
/// instance from eframe's (which is on wgpu 27) and bridged via a CPU pixel
/// buffer.
pub struct SpritePreview {
    device: wgpu::Device,
    queue: wgpu::Queue,
    tex_cache: TextureCache,
    sprite_renderer: SpriteRenderer,
    color_texture: wgpu::Texture,
    color_view: wgpu::TextureView,
    depth_view: wgpu::TextureView,
    readback: wgpu::Buffer,
    padded_bytes_per_row: u32,

    content: Content,
    entity: Option<EntitySprite>,
    animation: SpriteAnimationState,
    str_cache: StrEffectCache,
    str_name: Option<String>,
    str_time: f32,
    camera: Camera,
    cached_file_idx: Option<usize>,
    error: Option<String>,

    paused: bool,
    zoom: f32,
    background: Background,
    texture: Option<egui::TextureHandle>,
}

impl SpritePreview {
    pub fn new() -> Option<Self> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all().with_env(),
            flags: wgpu::InstanceFlags::from_build_config().with_env(),
            ..Default::default()
        });
        let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .ok()?;
        let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("grf-editor-sprite-preview"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::Performance,
            ..Default::default()
        }))
        .ok()?;

        let tex_cache = TextureCache::new(&device, 1.0);
        let sprite_renderer = SpriteRenderer::new(
            &device,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            &tex_cache.bind_group_layout,
            CANVAS as f32,
            CANVAS as f32,
            ragnarok_renderer::SPRITE_SHADER_SRC,
            false,
        );

        let color_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("sprite_preview_color"),
            size: wgpu::Extent3d {
                width: CANVAS,
                height: CANVAS,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let color_view = color_texture.create_view(&Default::default());

        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("sprite_preview_depth"),
            size: wgpu::Extent3d {
                width: CANVAS,
                height: CANVAS,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let depth_view = depth_texture.create_view(&Default::default());

        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let unpadded = CANVAS * BYTES_PER_PIXEL;
        let padded_bytes_per_row = unpadded.div_ceil(align) * align;
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sprite_preview_readback"),
            size: (padded_bytes_per_row * CANVAS) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        Some(Self {
            device,
            queue,
            tex_cache,
            sprite_renderer,
            color_texture,
            color_view,
            depth_view,
            readback,
            padded_bytes_per_row,
            content: Content::None,
            entity: None,
            animation: SpriteAnimationState::new(0),
            str_cache: StrEffectCache::new(),
            str_name: None,
            str_time: 0.0,
            camera: {
                let mut camera = Camera::default();
                camera.aspect = 1.0;
                camera.set_target(0.0, 0.0, 0.0);
                camera
            },
            cached_file_idx: None,
            error: None,
            paused: false,
            zoom: 2.0,
            background: Background::Black,
            texture: None,
        })
    }

    fn load(&mut self, grf: &GrfArchive, path: &str, file_idx: usize) {
        self.cached_file_idx = Some(file_idx);
        self.error = None;
        self.texture = None;
        self.content = Content::None;
        self.entity = None;
        self.str_name = None;

        if path.to_lowercase().ends_with(".str") {
            self.load_str(grf, path);
        } else {
            self.load_sprite(grf, path);
        }
    }

    fn load_sprite(&mut self, grf: &GrfArchive, spr_path: &str) {
        self.animation = SpriteAnimationState::new(0);
        self.zoom = 2.0;

        let Some(data) = sprite_loader::load_sprite_data_from_spr(grf, spr_path) else {
            self.error = Some(format!("Could not load sprite/act for {spr_path}"));
            return;
        };
        self.entity = Some(build_entity_sprite(
            &self.device,
            &self.queue,
            &self.tex_cache.bind_group_layout,
            data,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        ));
        self.content = Content::Sprite;
    }

    fn load_str(&mut self, grf: &GrfArchive, str_path: &str) {
        self.str_time = 0.0;
        self.zoom = 5.0;

        let lower = str_path.to_lowercase();
        let name = lower
            .strip_prefix(STR_EFFECT_PREFIX)
            .unwrap_or(&lower)
            .strip_suffix(".str")
            .unwrap_or(&lower)
            .to_string();
        if self
            .str_cache
            .load(&name, &[], grf, &mut self.tex_cache, &self.device, &self.queue)
        {
            self.str_name = Some(name);
            self.content = Content::Str;
        } else {
            self.error = Some(format!(
                "Could not load STR effect for {str_path} (textures must live under {STR_EFFECT_PREFIX})"
            ));
        }
    }

    fn render_to_pixels(&mut self) -> Vec<u8> {
        self.sprite_renderer.update_uniforms(
            &self.queue,
            &SpriteUniforms {
                screen_size: [CANVAS as f32, CANVAS as f32],
                zoom: 1.0,
                _pad: 0.0,
                pan: [0.0, 0.0],
                _pad2: [0.0, 0.0],
            },
        );

        let mut encoder = self.device.create_command_encoder(&Default::default());
        match self.content {
            Content::Sprite => {
                if let Some(entity) = &self.entity {
                    let anchor = [CANVAS as f32 / 2.0, CANVAS as f32 * 0.62];
                    let batches = entity.build_batches(
                        &self.animation,
                        None,
                        0,
                        anchor,
                        0.0,
                        self.zoom,
                        [0.0, 0.0],
                    );
                    self.sprite_renderer.render(
                        &mut encoder,
                        &self.color_view,
                        Some(&self.depth_view),
                        &self.device,
                        &self.queue,
                        Some(self.background.clear_color()),
                        &batches,
                    );
                }
            }
            Content::Str => {
                if let Some(name) = &self.str_name {
                    let inputs = [StrEmitterInput {
                        str_name: name,
                        position: [0.0, 0.0, 0.0],
                        anim_time: self.str_time,
                        repeat: true,
                    }];
                    let batches = build_str_effect_batches(
                        &inputs,
                        &self.str_cache,
                        &self.camera,
                        CANVAS as f32,
                        CANVAS as f32,
                        self.zoom,
                    );
                    self.sprite_renderer.render(
                        &mut encoder,
                        &self.color_view,
                        None,
                        &self.device,
                        &self.queue,
                        Some(self.background.clear_color()),
                        &batches,
                    );
                }
            }
            Content::None => {}
        }

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.color_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &self.readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.padded_bytes_per_row),
                    rows_per_image: Some(CANVAS),
                },
            },
            wgpu::Extent3d {
                width: CANVAS,
                height: CANVAS,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(std::iter::once(encoder.finish()));

        let slice = self.readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());

        let row_len = (CANVAS * BYTES_PER_PIXEL) as usize;
        let mut pixels = Vec::with_capacity(row_len * CANVAS as usize);
        {
            let data = slice.get_mapped_range();
            for row in 0..CANVAS as usize {
                let start = row * self.padded_bytes_per_row as usize;
                pixels.extend_from_slice(&data[start..start + row_len]);
            }
        }
        self.readback.unmap();

        for px in pixels.chunks_exact_mut(4) {
            px[3] = 255;
        }
        pixels
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        grf: &GrfArchive,
        spr_path: &str,
        file_idx: usize,
    ) {
        if self.cached_file_idx != Some(file_idx) {
            self.load(grf, spr_path, file_idx);
        }

        if let Some(err) = &self.error {
            ui.colored_label(egui::Color32::RED, err);
            return;
        }

        let action_count = self.entity.as_ref().map_or(0, |e| e.body_act.actions.len());
        let is_str = matches!(self.content, Content::Str);

        ui.horizontal(|ui| {
            if ui
                .button(if self.paused { "▶ Play" } else { "⏸ Pause" })
                .clicked()
            {
                self.paused = !self.paused;
            }
            ui.separator();
            if is_str {
                if ui.button("⟲ Restart").clicked() {
                    self.str_time = 0.0;
                }
            } else {
                ui.label("Dir");
                if ui.button("◀").clicked() {
                    let dir = if self.animation.direction() == 0 {
                        7
                    } else {
                        (self.animation.direction() - 1) as u8
                    };
                    self.animation.set_direction(dir);
                    self.animation.reset_motion();
                }
                if ui.button("▶").clicked() {
                    let dir = ((self.animation.direction() + 1) % 8) as u8;
                    self.animation.set_direction(dir);
                    self.animation.reset_motion();
                }
                ui.separator();
                ui.label("Action");
                if ui.button("−").clicked() {
                    self.step_action(-1, action_count);
                }
                if ui.button("+").clicked() {
                    self.step_action(1, action_count);
                }
            }
            ui.separator();
            if ui.button("Zoom −").clicked() {
                self.zoom = (self.zoom / 1.2).max(0.25);
            }
            if ui.button("Zoom +").clicked() {
                self.zoom = (self.zoom * 1.2).min(20.0);
            }
            ui.separator();
            if ui.button(self.background.label()).clicked() {
                self.background = self.background.next();
            }
        });

        let dt = ui.input(|i| i.stable_dt).min(0.1);
        if !self.paused {
            match self.content {
                Content::Sprite => {
                    if let Some(entity) = &self.entity {
                        self.animation.update_flat(dt, &entity.body_act);
                    }
                }
                Content::Str => self.str_time += dt,
                Content::None => {}
            }
        }

        let pixels = self.render_to_pixels();
        let image =
            egui::ColorImage::from_rgba_unmultiplied([CANVAS as usize, CANVAS as usize], &pixels);
        match &mut self.texture {
            Some(tex) => tex.set(image, egui::TextureOptions::NEAREST),
            None => {
                self.texture = Some(ui.ctx().load_texture(
                    "sprite_preview",
                    image,
                    egui::TextureOptions::NEAREST,
                ));
            }
        }

        let tex_id = self.texture.as_ref().map(|t| t.id());
        let status = match self.content {
            Content::Sprite => self.entity.as_ref().map(|entity| {
                let action_idx = self.animation.flat_action_index(&entity.body_act);
                let motion_count = entity.body_act.actions[action_idx].motions.len();
                let action_name = SpriteActionType::from_index(self.animation.action())
                    .map(|a| a.name())
                    .unwrap_or("?");
                format!(
                    "Act: {} ({})  Dir: {}  Frame: {}/{}",
                    self.animation.action(),
                    action_name,
                    self.animation.direction(),
                    self.animation.motion_index() + 1,
                    motion_count,
                )
            }),
            Content::Str => self.str_name.as_ref().and_then(|name| {
                let entry = self.str_cache.get(name)?;
                let str_file = &entry.str_file;
                let key = if str_file.fps > 0 {
                    (self.str_time * str_file.fps as f32) as u32 % str_file.max_key.max(1)
                } else {
                    0
                };
                Some(format!(
                    "STR  Layers: {}  Key: {}/{}  {} fps",
                    str_file.layers.len(),
                    key,
                    str_file.max_key,
                    str_file.fps,
                ))
            }),
            Content::None => None,
        };

        if let Some(tex_id) = tex_id {
            let avail = ui.available_size();
            let side = avail.x.min(avail.y).min(CANVAS as f32).max(64.0);
            let hovered = ui
                .vertical_centered(|ui| {
                    let response =
                        ui.image(egui::load::SizedTexture::new(tex_id, egui::vec2(side, side)));
                    if let Some(status) = &status {
                        ui.label(status);
                    }
                    response.hovered()
                })
                .inner;
            if hovered {
                self.handle_keyboard(ui, action_count);
            }
        }

        if !self.paused {
            ui.ctx().request_repaint();
        }
    }

    fn step_action(&mut self, delta: i32, action_count: usize) {
        let Some(entity) = &self.entity else { return };
        if action_count == 0 {
            return;
        }
        let cur = self.animation.action() as i32;
        let next = (cur + delta).rem_euclid(action_count as i32) as usize;
        self.animation
            .set_action_clamped(next, MotionType::Loop, &entity.body_act);
    }

    fn handle_keyboard(&mut self, ui: &egui::Ui, action_count: usize) {
        let (space, left, right, up, down, plus, minus) = ui.input(|i| {
            (
                i.key_pressed(egui::Key::Space),
                i.key_pressed(egui::Key::ArrowLeft),
                i.key_pressed(egui::Key::ArrowRight),
                i.key_pressed(egui::Key::ArrowUp),
                i.key_pressed(egui::Key::ArrowDown),
                i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals),
                i.key_pressed(egui::Key::Minus),
            )
        });
        if space {
            self.paused = !self.paused;
        }
        if left {
            let dir = if self.animation.direction() == 0 {
                7
            } else {
                (self.animation.direction() - 1) as u8
            };
            self.animation.set_direction(dir);
            self.animation.reset_motion();
        }
        if right {
            let dir = ((self.animation.direction() + 1) % 8) as u8;
            self.animation.set_direction(dir);
            self.animation.reset_motion();
        }
        if up {
            self.step_action(-1, action_count);
        }
        if down {
            self.step_action(1, action_count);
        }
        if plus {
            self.zoom = (self.zoom * 1.2).min(20.0);
        }
        if minus {
            self.zoom = (self.zoom / 1.2).max(0.25);
        }
    }
}
