use eframe::egui;
use ragnarok_formats::act::{MotionType, SpriteActionType, SpriteAnimationState};
use ragnarok_formats::gr2::{Gr2Container, Gr2File};
use ragnarok_formats::grf::GrfArchive;
use ragnarok_formats::rsm::RsmFile;
use ragnarok_game::gr2_model::{
    AnimationClip, Gr2Action, SkeletonPose, animation_file_path, bone_type_from_name,
};
use ragnarok_game::sprite_loader;
use ragnarok_renderer::wgpu;
use ragnarok_renderer::{
    Camera, EntitySprite, GlobalUniforms, Gr2ModelRenderer, LightUniform, ModelRenderer,
    SpriteRenderer, SpriteUniforms, StrEffectCache, StrEmitterInput, TextureCache, block_on,
    build_entity_sprite, build_str_effect_batches,
};

const CANVAS: u32 = 384;
const BYTES_PER_PIXEL: u32 = 4;
const STR_EFFECT_PREFIX: &str = "data/texture/effect/";
const GR2_ACTION_NAMES: [&str; 5] = ["Stand", "Move", "Attack", "Dead", "Damage"];

enum Content {
    None,
    Sprite,
    Str,
    Model,
    Gr2,
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
    global_uniforms: GlobalUniforms,
    model: Option<ModelRenderer>,
    gr2: Option<Gr2ModelRenderer>,
    gr2_pose: Option<SkeletonPose>,
    gr2_clips: [Option<AnimationClip>; 5],
    gr2_action: usize,
    gr2_time: f32,
    model_center: [f32; 3],
    model_size: [f32; 3],
    model_yaw: f32,
    model_pitch: f32,
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
        let global_uniforms = GlobalUniforms::new(&device);
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
            global_uniforms,
            model: None,
            gr2: None,
            gr2_pose: None,
            gr2_clips: Default::default(),
            gr2_action: 0,
            gr2_time: 0.0,
            model_center: [0.0; 3],
            model_size: [1.0; 3],
            model_yaw: 0.7,
            model_pitch: 0.5,
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
        self.model = None;
        self.gr2 = None;
        self.gr2_pose = None;
        self.gr2_clips = Default::default();
        self.gr2_action = 0;

        let lower = path.to_lowercase();
        if lower.ends_with(".str") {
            self.load_str(grf, path);
        } else if lower.ends_with(".rsm") {
            self.load_model(grf, path);
        } else if lower.ends_with(".gr2") {
            self.load_gr2(grf, path);
        } else if lower.ends_with(".act") {
            self.load_sprite(grf, &spr_sibling(path));
        } else {
            self.load_sprite(grf, path);
        }
    }

    fn load_gr2(&mut self, grf: &GrfArchive, path: &str) {
        self.zoom = 1.0;
        self.model_yaw = 0.7;
        self.model_pitch = 0.35;
        self.gr2_time = 0.0;

        let data = match grf.read_file(path) {
            Ok(d) => d,
            Err(e) => {
                self.error = Some(format!("Could not read {path}: {e}"));
                return;
            }
        };
        let file = match Gr2Container::parse(&data).and_then(|c| Gr2File::parse(&c)) {
            Ok(f) => f,
            Err(e) => {
                self.error = Some(format!("Could not parse {path}: {e:?}"));
                return;
            }
        };
        let Some(pose) = SkeletonPose::from_model(&file, 0) else {
            self.error = Some(format!("Model has no skeleton: {path}"));
            return;
        };
        let Some(renderer) = Gr2ModelRenderer::from_gr2(
            &file,
            0,
            &self.device,
            &self.queue,
            &self.global_uniforms,
            &self.tex_cache,
            wgpu::TextureFormat::Rgba8UnormSrgb,
        ) else {
            self.error = Some(format!("Model has no drawable geometry: {path}"));
            return;
        };
        renderer.set_transform(&self.queue, z_up_transform());
        self.model_center = renderer.center;
        self.model_size = renderer.size;

        let bone_type = bone_type_from_name(path);
        self.gr2_clips = std::array::from_fn(|i| match Gr2Action::ALL[i] {
            Gr2Action::Stand => AnimationClip::from_gr2(&file, 0),
            action => {
                let anim_path = animation_file_path(bone_type?, action)?;
                let bytes = grf.read_file(&anim_path).ok()?;
                let anim = Gr2Container::parse(&bytes)
                    .and_then(|c| Gr2File::parse(&c))
                    .ok()?;
                AnimationClip::from_gr2(&anim, 0)
            }
        });
        self.gr2_action = 0;
        self.gr2 = Some(renderer);
        self.gr2_pose = Some(pose);
        self.content = Content::Gr2;
    }

    fn load_model(&mut self, grf: &GrfArchive, rsm_path: &str) {
        self.zoom = 1.0;
        self.model_yaw = 0.7;
        self.model_pitch = 0.5;

        let data = match grf.read_file(rsm_path) {
            Ok(d) => d,
            Err(e) => {
                self.error = Some(format!("Could not read {rsm_path}: {e}"));
                return;
            }
        };
        let rsm = match RsmFile::parse(&data) {
            Ok(r) => r,
            Err(e) => {
                self.error = Some(format!("Could not parse {rsm_path}: {e}"));
                return;
            }
        };
        match ModelRenderer::from_rsm(
            &rsm,
            grf,
            &self.device,
            &self.queue,
            &self.global_uniforms,
            &mut self.tex_cache,
            wgpu::TextureFormat::Rgba8UnormSrgb,
        ) {
            Some((model, center, size)) => {
                self.model = Some(model);
                self.model_center = center;
                self.model_size = size;
                self.content = Content::Model;
            }
            None => {
                self.error = Some(format!("Model has no drawable geometry: {rsm_path}"));
            }
        }
    }

    fn frame_model_camera(&mut self, z_up: bool) {
        let [sx, sy, sz] = self.model_size;
        let radius = 0.5 * (sx * sx + sy * sy + sz * sz).sqrt().max(1.0);
        let fov_y = 40_f32.to_radians();
        self.camera.fov_y = fov_y;
        self.camera.aspect = 1.0;
        let [cx, cy, cz] = self.model_center;
        let (tx, ty, tz) = if z_up { (cx, -cz, cy) } else { (cx, cy, cz) };
        self.camera.set_target(tx, ty, tz);
        self.camera.yaw = self.model_yaw;
        self.camera.pitch = self.model_pitch;
        self.camera.distance = radius / (fov_y * 0.5).tan() / self.zoom.max(0.05);
        self.camera.near = (radius * 0.02).max(0.5);
        self.camera.far = radius * 20.0 + 2000.0;
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
        if self.str_cache.load(
            &name,
            &[],
            grf,
            &mut self.tex_cache,
            &self.device,
            &self.queue,
        ) {
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
                world_light: [1.0; 4],
            },
        );

        if matches!(self.content, Content::Model | Content::Gr2) {
            self.frame_model_camera(matches!(self.content, Content::Gr2));
            self.global_uniforms
                .update_camera(&self.queue, &self.camera);
            self.global_uniforms
                .update_light(&self.queue, &model_light());
        }
        if matches!(self.content, Content::Gr2) {
            if let (Some(gr2), Some(pose)) = (&self.gr2, &self.gr2_pose) {
                let clip = &self.gr2_clips[self.gr2_action];
                gr2.set_palette(&self.queue, &gr2_palette(clip, pose, self.gr2_time));
            }
        }

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
            Content::Model => {
                if let Some(model) = &self.model {
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("model_preview"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &self.color_view,
                            depth_slice: None,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(self.background.clear_color()),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                            view: &self.depth_view,
                            depth_ops: Some(wgpu::Operations {
                                load: wgpu::LoadOp::Clear(1.0),
                                store: wgpu::StoreOp::Store,
                            }),
                            stencil_ops: None,
                        }),
                        ..Default::default()
                    });
                    model.render(&mut pass, &self.global_uniforms, &self.tex_cache);
                }
            }
            Content::Gr2 => {
                if let Some(gr2) = &self.gr2 {
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("gr2_preview"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &self.color_view,
                            depth_slice: None,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(self.background.clear_color()),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                            view: &self.depth_view,
                            depth_ops: Some(wgpu::Operations {
                                load: wgpu::LoadOp::Clear(1.0),
                                store: wgpu::StoreOp::Store,
                            }),
                            stencil_ops: None,
                        }),
                        ..Default::default()
                    });
                    gr2.render(&mut pass, &self.global_uniforms);
                }
            }
            Content::None => {}
        }

        self.finish_read(encoder)
    }

    /// Copy the off-screen colour texture into the readback buffer, map it, and
    /// return tightly-packed opaque RGBA (`CANVAS`×`CANVAS`).
    fn finish_read(&self, mut encoder: wgpu::CommandEncoder) -> Vec<u8> {
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

    /// Render a single still thumbnail (`size`×`size`) for a sprite or STR
    /// effect without disturbing the live single-file preview state. Used by the
    /// gallery grid. Returns `None` if the asset fails to load.
    pub fn thumbnail(
        &mut self,
        grf: &GrfArchive,
        path: &str,
        size: u32,
    ) -> Option<egui::ColorImage> {
        self.sprite_renderer.update_uniforms(
            &self.queue,
            &SpriteUniforms {
                screen_size: [CANVAS as f32, CANVAS as f32],
                zoom: 1.0,
                _pad: 0.0,
                pan: [0.0, 0.0],
                _pad2: [0.0, 0.0],
                world_light: [1.0; 4],
            },
        );

        let lower = path.to_lowercase();
        let pixels = if lower.ends_with(".str") {
            self.render_str_thumbnail(grf, path)?
        } else if lower.ends_with(".rsm") {
            self.render_model_thumbnail(grf, path)?
        } else if lower.ends_with(".gr2") {
            self.render_gr2_thumbnail(grf, path)?
        } else if lower.ends_with(".act") {
            self.render_sprite_thumbnail(grf, &spr_sibling(path))?
        } else {
            self.render_sprite_thumbnail(grf, path)?
        };

        let src = image::RgbaImage::from_raw(CANVAS, CANVAS, pixels)?;
        let scaled =
            image::imageops::resize(&src, size, size, image::imageops::FilterType::Triangle);
        Some(egui::ColorImage::from_rgba_unmultiplied(
            [size as usize, size as usize],
            scaled.as_raw(),
        ))
    }

    fn render_sprite_thumbnail(&mut self, grf: &GrfArchive, spr_path: &str) -> Option<Vec<u8>> {
        let data = sprite_loader::load_sprite_data_from_spr(grf, spr_path)?;
        let entity = build_entity_sprite(
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
        );
        let anim = SpriteAnimationState::new(0);
        let anchor = [CANVAS as f32 / 2.0, CANVAS as f32 * 0.62];
        let batches = entity.build_batches(&anim, None, 0, anchor, 0.0, 2.0, [0.0, 0.0]);
        let mut encoder = self.device.create_command_encoder(&Default::default());
        self.sprite_renderer.render(
            &mut encoder,
            &self.color_view,
            Some(&self.depth_view),
            &self.device,
            &self.queue,
            Some(Background::Checkerboard.clear_color()),
            &batches,
        );
        Some(self.finish_read(encoder))
    }

    fn render_str_thumbnail(&mut self, grf: &GrfArchive, str_path: &str) -> Option<Vec<u8>> {
        let lower = str_path.to_lowercase();
        let name = lower
            .strip_prefix(STR_EFFECT_PREFIX)
            .unwrap_or(&lower)
            .strip_suffix(".str")
            .unwrap_or(&lower)
            .to_string();
        if !self.str_cache.load(
            &name,
            &[],
            grf,
            &mut self.tex_cache,
            &self.device,
            &self.queue,
        ) {
            return None;
        }
        // Sample mid-animation — STR effects are usually empty at t=0.
        let anim_time = self
            .str_cache
            .get(&name)
            .filter(|e| e.str_file.fps > 0)
            .map(|e| e.str_file.max_key as f32 / e.str_file.fps as f32 * 0.5)
            .unwrap_or(0.5);
        let inputs = [StrEmitterInput {
            str_name: &name,
            position: [0.0, 0.0, 0.0],
            anim_time,
            repeat: false,
        }];
        let batches = build_str_effect_batches(
            &inputs,
            &self.str_cache,
            &self.camera,
            CANVAS as f32,
            CANVAS as f32,
            5.0,
        );
        let mut encoder = self.device.create_command_encoder(&Default::default());
        self.sprite_renderer.render(
            &mut encoder,
            &self.color_view,
            None,
            &self.device,
            &self.queue,
            Some(wgpu::Color::BLACK),
            &batches,
        );
        Some(self.finish_read(encoder))
    }

    fn render_model_thumbnail(&mut self, grf: &GrfArchive, rsm_path: &str) -> Option<Vec<u8>> {
        let data = grf.read_file(rsm_path).ok()?;
        let rsm = RsmFile::parse(&data).ok()?;
        let (model, center, size) = ModelRenderer::from_rsm(
            &rsm,
            grf,
            &self.device,
            &self.queue,
            &self.global_uniforms,
            &mut self.tex_cache,
            wgpu::TextureFormat::Rgba8UnormSrgb,
        )?;

        self.model_center = center;
        self.model_size = size;
        self.model_yaw = 0.7;
        self.model_pitch = 0.5;
        let saved_zoom = self.zoom;
        self.zoom = 1.0;
        self.frame_model_camera(false);
        self.zoom = saved_zoom;
        self.global_uniforms
            .update_camera(&self.queue, &self.camera);
        self.global_uniforms
            .update_light(&self.queue, &model_light());

        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("model_thumb"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.color_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(Background::Checkerboard.clear_color()),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            model.render(&mut pass, &self.global_uniforms, &self.tex_cache);
        }
        Some(self.finish_read(encoder))
    }

    fn render_gr2_thumbnail(&mut self, grf: &GrfArchive, path: &str) -> Option<Vec<u8>> {
        let data = grf.read_file(path).ok()?;
        let file = Gr2Container::parse(&data)
            .and_then(|c| Gr2File::parse(&c))
            .ok()?;
        let pose = SkeletonPose::from_model(&file, 0)?;
        let clip = AnimationClip::from_gr2(&file, 0);
        let renderer = Gr2ModelRenderer::from_gr2(
            &file,
            0,
            &self.device,
            &self.queue,
            &self.global_uniforms,
            &self.tex_cache,
            wgpu::TextureFormat::Rgba8UnormSrgb,
        )?;
        renderer.set_transform(&self.queue, z_up_transform());
        renderer.set_palette(&self.queue, &gr2_palette(&clip, &pose, 0.0));

        self.model_center = renderer.center;
        self.model_size = renderer.size;
        self.model_yaw = 0.7;
        self.model_pitch = 0.35;
        let saved_zoom = self.zoom;
        self.zoom = 1.0;
        self.frame_model_camera(true);
        self.zoom = saved_zoom;
        self.global_uniforms
            .update_camera(&self.queue, &self.camera);
        self.global_uniforms
            .update_light(&self.queue, &model_light());

        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("gr2_thumb"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.color_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(Background::Checkerboard.clear_color()),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            renderer.render(&mut pass, &self.global_uniforms);
        }
        Some(self.finish_read(encoder))
    }

    pub fn show(&mut self, ui: &mut egui::Ui, grf: &GrfArchive, spr_path: &str, file_idx: usize) {
        if self.cached_file_idx != Some(file_idx) {
            self.load(grf, spr_path, file_idx);
        }

        if let Some(err) = &self.error {
            ui.colored_label(egui::Color32::RED, err);
            return;
        }

        let action_count = self.entity.as_ref().map_or(0, |e| e.body_act.actions.len());

        ui.horizontal(|ui| {
            match self.content {
                Content::Str => {
                    if ui
                        .button(if self.paused { "▶ Play" } else { "⏸ Pause" })
                        .clicked()
                    {
                        self.paused = !self.paused;
                    }
                    ui.separator();
                    if ui.button("⟲ Restart").clicked() {
                        self.str_time = 0.0;
                    }
                }
                Content::Model => {
                    ui.label("Rotate");
                    if ui.button("◀").clicked() {
                        self.model_yaw -= std::f32::consts::FRAC_PI_8;
                    }
                    if ui.button("▶").clicked() {
                        self.model_yaw += std::f32::consts::FRAC_PI_8;
                    }
                    ui.separator();
                    ui.label("Tilt");
                    if ui.button("▲").clicked() {
                        self.model_pitch = (self.model_pitch + 0.15).min(1.5);
                    }
                    if ui.button("▼").clicked() {
                        self.model_pitch = (self.model_pitch - 0.15).max(-1.5);
                    }
                }
                Content::Gr2 => {
                    if ui
                        .button(if self.paused { "▶ Play" } else { "⏸ Pause" })
                        .clicked()
                    {
                        self.paused = !self.paused;
                    }
                    ui.separator();
                    ui.label("Action");
                    if ui.button("◀").clicked() {
                        self.step_gr2_action(-1);
                    }
                    ui.label(GR2_ACTION_NAMES[self.gr2_action]);
                    if ui.button("▶").clicked() {
                        self.step_gr2_action(1);
                    }
                    ui.separator();
                    ui.label("Rotate");
                    if ui.button("◀").clicked() {
                        self.model_yaw -= std::f32::consts::FRAC_PI_8;
                    }
                    if ui.button("▶").clicked() {
                        self.model_yaw += std::f32::consts::FRAC_PI_8;
                    }
                    ui.separator();
                    ui.label("Tilt");
                    if ui.button("▲").clicked() {
                        self.model_pitch = (self.model_pitch + 0.15).min(1.5);
                    }
                    if ui.button("▼").clicked() {
                        self.model_pitch = (self.model_pitch - 0.15).max(-1.5);
                    }
                }
                _ => {
                    if ui
                        .button(if self.paused { "▶ Play" } else { "⏸ Pause" })
                        .clicked()
                    {
                        self.paused = !self.paused;
                    }
                    ui.separator();
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
                Content::Gr2 => self.gr2_time += dt,
                Content::Model | Content::None => {}
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
            Content::Model => {
                let [sx, sy, sz] = self.model_size;
                Some(format!("RSM  Size: {sx:.0} × {sy:.0} × {sz:.0}"))
            }
            Content::Gr2 => {
                let bones = self.gr2_pose.as_ref().map_or(0, |p| p.bone_count());
                let actions = self.gr2_clips.iter().filter(|c| c.is_some()).count();
                Some(format!(
                    "GR2  {}  Bones: {bones}  Actions: {actions}",
                    GR2_ACTION_NAMES[self.gr2_action],
                ))
            }
            Content::None => None,
        };

        if let Some(tex_id) = tex_id {
            let avail = ui.available_size();
            let side = avail.x.min(avail.y).min(CANVAS as f32).max(64.0);
            let hovered = ui
                .vertical_centered(|ui| {
                    let response = ui.image(egui::load::SizedTexture::new(
                        tex_id,
                        egui::vec2(side, side),
                    ));
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

    /// Cycle to the next/previous GR2 action that actually has a clip loaded.
    fn step_gr2_action(&mut self, delta: i32) {
        let avail: Vec<usize> = (0..self.gr2_clips.len())
            .filter(|&i| self.gr2_clips[i].is_some())
            .collect();
        if avail.is_empty() {
            return;
        }
        let cur = avail
            .iter()
            .position(|&i| i == self.gr2_action)
            .unwrap_or(0) as i32;
        let next = (cur + delta).rem_euclid(avail.len() as i32) as usize;
        self.gr2_action = avail[next];
        self.gr2_time = 0.0;
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

/// Sibling `.spr` path for a selected `.act` file — the sprite loader keys on the
/// `.spr` and derives the `.act` back from it.
fn spr_sibling(act_path: &str) -> String {
    let lower = act_path.to_lowercase();
    match lower.strip_suffix(".act") {
        Some(base) => format!("{base}.spr"),
        None => lower,
    }
}

/// GR2 models are authored Z-up; a +90° X-rotation maps model +Z to world -Y
/// (up in RO coordinates) without flipping handedness.
fn z_up_transform() -> glam::Mat4 {
    glam::Mat4::from_rotation_x(std::f32::consts::FRAC_PI_2)
}

/// Skinning palette for the stand clip at time `t`, falling back to the bind
/// pose when the model carries no animation.
fn gr2_palette(clip: &Option<AnimationClip>, pose: &SkeletonPose, t: f32) -> Vec<glam::Mat4> {
    match clip {
        Some(clip) if clip.duration > 0.0 => clip.skinning_palette(pose, t % clip.duration),
        Some(clip) => clip.skinning_palette(pose, 0.0),
        None => pose.bind_palette(),
    }
}

/// Angled directional light so standalone models read as 3D rather than flat.
fn model_light() -> LightUniform {
    LightUniform {
        light_dir: [-0.5, -1.0, -0.4, 0.0],
        ..LightUniform::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn gr2_thumbnail_renders_geometry() {
        let Some(grf_path) = [
            "data/data.grf",
            "../../data/data.grf",
            "../../../../data/data.grf",
        ]
        .iter()
        .find(|p| Path::new(p).exists()) else {
            eprintln!("skipping: data.grf not found");
            return;
        };
        let grf = GrfArchive::open(Path::new(grf_path)).expect("open grf");
        let Some(mut preview) = SpritePreview::new() else {
            eprintln!("skipping: no GPU adapter");
            return;
        };

        let image = preview
            .thumbnail(&grf, "data/model/3dmob/empelium90_0.gr2", 96)
            .expect("gr2 thumbnail renders");

        let first = image.pixels[0];
        let varied = image.pixels.iter().any(|p| *p != first);
        assert!(
            varied,
            "gr2 thumbnail is a flat frame — model did not render"
        );

        preview.load_gr2(&grf, "data/model/3dmob/kguardian90_7.gr2");
        assert!(preview.error.is_none(), "{:?}", preview.error);
        let actions = preview.gr2_clips.iter().filter(|c| c.is_some()).count();
        assert!(
            actions > 1,
            "guardian should load external action clips, got {actions}"
        );
    }
}
