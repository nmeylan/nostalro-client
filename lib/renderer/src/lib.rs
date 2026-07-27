pub mod camera;
pub mod cell_light;
pub mod damage_number;
mod device;
pub mod effect;
pub mod effect_sprite;
pub mod font_atlas;
pub mod fps;
pub mod global_uniforms;
pub mod gr2_model;
pub mod graffiti;
pub mod grid_selector;
pub mod ground;
pub mod ground_proxy;
pub mod model;
pub mod rsm_anim;
pub mod sprite;
pub mod sprite_projection;
pub mod texture;
pub mod ui_renderer;
pub mod water;

pub use camera::Camera;
pub use device::{RenderDevice, block_on};
pub use fps::Fps;
pub use global_uniforms::{FogUniform, GlobalUniforms, LightUniform, PointLightGpu};

pub use damage_number::render_damage_number_quads;
pub use effect::{
    BlendBucket, BlendKind, DrawRecord, EffectDispatcher, PipelineKind, StrEffectCache,
    StrEffectEntry, StrEmitterInput, build_str_effect_batches, d3d_blend_to_wgpu,
    prepare_billboard_records, prepare_cylinder_records, prepare_frustum_records,
    prepare_ground_disc_records, prepare_line_strip_records, prepare_quad_horn_records,
    prepare_radial_ring_records, prepare_screen_quad_records, prepare_sphere_records,
    prepare_texture3d_records, prepare_world_quad_records,
};
pub use effect_sprite::{
    BurstParticle, EffectSpriteCache, EffectSpriteEntry, EmitterDraw, SpriteEffectEmitter,
    build_emitter_batches, collect_sprite_effect_draws, prepare_sprite_particle_records,
    project_billboard,
};
pub use font_atlas::FontAtlas;
pub use gr2_model::{Gr2ModelRenderer, Gr2ModelVertex, build_gr2_geometry};
pub use grid_selector::GridSelectorRenderer;
pub use ground::GroundRenderer;
pub use ground_proxy::GroundProxyRenderer;
pub use model::{AnimatedModelRenderer, ModelRenderer};
pub use sprite::{
    BodyChannels, ClipQuad, CompositeClips, EntitySprite, SpriteBatch, SpriteRenderer,
    SpriteTextures, SpriteUniforms, SpriteVertex, build_clip_quad, build_clip_quad_scaled,
    build_composite_clips, build_entity_sprite, compose_actor_batches, scale_clip_vertices,
    transform_batch_vertices, upload_sprite_textures,
};
pub use texture::TextureCache;
pub use ui_renderer::{UiDrawCommand, UiRenderer, UiVertex};
pub use water::WaterRenderer;
pub use wgpu;

pub const SPRITE_SHADER_SRC: &str = include_str!("shaders/sprite.wgsl");

use ragnarok_formats::fog_table::FogEntry;
use ragnarok_formats::gnd::GndFile;
use ragnarok_formats::grf::GrfArchive;
use ragnarok_formats::rsm::RsmFile;
use ragnarok_formats::rsw::{RswFile, RswObject};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackgroundMode {
    Clear,
    GroundProxy,
    RswMap,
}

impl Default for BackgroundMode {
    fn default() -> Self {
        BackgroundMode::RswMap
    }
}

pub enum UiTextureRef {
    FontAtlas,
    White,
    Named(String),
    Inline(usize),
}

pub struct UiDrawCall {
    pub vertices: Vec<UiVertex>,
    pub indices: Vec<u32>,
    pub texture: UiTextureRef,
}

pub struct FrameInputs<'a> {
    pub ui_draw_calls: &'a [UiDrawCall],
    pub effect_sprite_batches: &'a [SpriteBatch<'a>],
    pub effect_draws: &'a effect::EffectDrawList,
    pub sprite_particle_records: Vec<DrawRecord<'a>>,
    pub sprite_batches: &'a [SpriteBatch<'a>],
    pub silhouette_batches: &'a [SpriteBatch<'a>],
    pub cursor_batches: &'a [SpriteBatch<'a>],
    pub inline_textures: &'a [&'a wgpu::BindGroup],
    pub elapsed: f32,
    /// Seconds since the previous frame. Callers that render more than once per
    /// frame (offscreen capture) must pass 0.0 for the extra passes so
    /// time-stepped state is not advanced twice.
    pub delta: f32,
}

pub struct Renderer {
    pub device: RenderDevice,
    pub camera: Camera,
    pub global_uniforms: GlobalUniforms,
    pub texture_cache: TextureCache,
    pub ground_renderer: Option<GroundRenderer>,
    pub ground_proxy: Option<GroundProxyRenderer>,
    pub model_renderer: Option<ModelRenderer>,
    pub animated_model_renderer: Option<AnimatedModelRenderer>,
    pub skill_unit_models: std::collections::HashMap<u32, ModelRenderer>,
    /// Animated GR2 entity models keyed by entity gid (emperium, guardians…).
    pub gr2_models: std::collections::HashMap<u32, Gr2ModelRenderer>,
    pub water_renderer: Option<WaterRenderer>,
    pub grid_selector: Option<GridSelectorRenderer>,
    pub sprite_renderer: SpriteRenderer,
    pub effect_sprite_renderer: SpriteRenderer,
    pub effect_primitives: effect::EffectPrimitiveRegistry,
    pub effect_dispatcher: effect::EffectDispatcher,
    pub ui_renderer: UiRenderer,
    pub font_atlas: FontAtlas,
    pub font_atlas_bind_group: wgpu::BindGroup,
    pub white_bind_group: wgpu::BindGroup,
    pub font_px_height: f32,
    pub dpi_scale: f32,
    pub clear_color: wgpu::Color,
    pub background_mode: BackgroundMode,
    /// The map's day lighting, captured in `load_map`. `set_day_night` patches the
    /// diffuse rgb over this so the day/night fade never loses the map's light dir,
    /// ambient or shadow strength.
    pub base_light: LightUniform,
    /// World-unit scale of the loaded map (240 * gnd zoom), needed to convert
    /// fog-table near/far into world distances when fog is toggled at runtime.
    fog_scale: f32,
    lightmap_enabled: bool,
}

#[allow(clippy::too_many_arguments)]
fn build_effect_records<'tex>(
    effect_draws: &effect::EffectDrawList,
    camera: &Camera,
    texture_cache: &'tex TextureCache,
    white_bind_group: &'tex wgpu::BindGroup,
    logical_w: f32,
    logical_h: f32,
    primitives: &effect::EffectPrimitiveRegistry,
) -> Vec<DrawRecord<'tex>> {
    let texture_lookup = |name: &str| -> Option<&'tex wgpu::BindGroup> {
        if name.is_empty() {
            return None;
        }
        // Runtime-composed textures are registered under their own key, which must not
        // be rewritten into a GRF path.
        name.split('|').find_map(|candidate| {
            texture_cache
                .get(candidate)
                .or_else(|| texture_cache.get(&effect::effect_texture_path(candidate)))
        })
    };
    let mut records: Vec<DrawRecord<'tex>> = Vec::new();
    records.extend(prepare_billboard_records(
        effect_draws,
        camera,
        logical_w,
        logical_h,
        white_bind_group,
        &texture_lookup,
    ));
    for renderer in primitives.iter() {
        records.extend(renderer.prepare(effect_draws, camera, white_bind_group, &texture_lookup));
    }
    records
}

impl Renderer {
    pub async fn new(
        window: Arc<winit::window::Window>,
        font_px_height: f32,
        dpi_scale: f32,
    ) -> Self {
        let device = RenderDevice::new(window).await;
        let camera = Camera {
            aspect: device.surface_config.width as f32 / device.surface_config.height as f32,
            ..Default::default()
        };
        let global_uniforms = GlobalUniforms::new(&device.device);
        let texture_cache = TextureCache::new(&device.device, dpi_scale);

        let font_atlas = FontAtlas::from_embedded(font_px_height, dpi_scale);
        let font_atlas_bind_group = texture::create_font_atlas_bind_group(
            &device.device,
            &device.queue,
            &font_atlas.image,
            &texture_cache.bind_group_layout,
            "font_atlas",
        );

        let white_img = image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 255, 255, 255]));
        let white_bind_group = texture::create_texture_bind_group(
            &device.device,
            &device.queue,
            &white_img,
            &texture_cache.bind_group_layout,
            "ui_white",
        );

        let logical_w = device.surface_config.width as f32 / dpi_scale;
        let logical_h = device.surface_config.height as f32 / dpi_scale;

        // Entity sprites: the colour pass tests against world geometry per-pixel
        // (gradient) but writes no depth, so coplanar body layers/copies blend by
        // paint order (no seams). A flat feet-depth body silhouette is stamped
        // afterward (render_silhouette) purely so effects occlude against the body
        // the way the game did before per-pixel gradient depth — effects above the
        // feet on top, ground effects at the feet occluded.
        let sprite_renderer = SpriteRenderer::new(
            &device.device,
            device.surface_format,
            &texture_cache.bind_group_layout,
            logical_w,
            logical_h,
            include_str!("shaders/sprite.wgsl"),
            false,
        );
        let effect_sprite_renderer = SpriteRenderer::new(
            &device.device,
            device.surface_format,
            &texture_cache.bind_group_layout,
            logical_w,
            logical_h,
            include_str!("shaders/sprite.wgsl"),
            false,
        );
        let effect_primitives = effect::EffectPrimitiveRegistry::new(
            &device.device,
            device.surface_format,
            &global_uniforms.bind_group_layout,
            &texture_cache.bind_group_layout,
        );
        let effect_dispatcher = effect::EffectDispatcher::new(&device.device);

        let ui_renderer = UiRenderer::new(
            &device.device,
            device.surface_format,
            &texture_cache.bind_group_layout,
            logical_w,
            logical_h,
        );

        Self {
            device,
            camera,
            global_uniforms,
            texture_cache,
            ground_renderer: None,
            ground_proxy: None,
            model_renderer: None,
            animated_model_renderer: None,
            skill_unit_models: std::collections::HashMap::new(),
            gr2_models: std::collections::HashMap::new(),
            water_renderer: None,
            grid_selector: None,
            sprite_renderer,
            effect_sprite_renderer,
            effect_primitives,
            effect_dispatcher,
            ui_renderer,
            font_atlas,
            font_atlas_bind_group,
            white_bind_group,
            font_px_height,
            dpi_scale,
            clear_color: wgpu::Color {
                r: 0.392,
                g: 0.584,
                b: 0.929,
                a: 1.0,
            },
            background_mode: BackgroundMode::default(),
            base_light: LightUniform::default(),
            fog_scale: 240.0,
            lightmap_enabled: true,
        }
    }

    pub fn set_background_mode(&mut self, mode: BackgroundMode) {
        self.background_mode = mode;
    }

    pub fn set_day_night(&mut self, diffuse: [f32; 3], sprite_light: [f32; 3]) {
        let mut light = self.base_light;
        light.diffuse_color = [diffuse[0], diffuse[1], diffuse[2], light.diffuse_color[3]];
        self.global_uniforms
            .update_light(&self.device.queue, &light);
        self.sprite_renderer
            .set_world_light(&self.device.queue, sprite_light);
    }

    pub fn toggle_lightmap(&mut self) -> bool {
        self.set_lightmap_enabled(!self.lightmap_enabled);
        self.lightmap_enabled
    }

    pub fn lightmap_enabled(&self) -> bool {
        self.lightmap_enabled
    }

    pub fn set_lightmap_enabled(&mut self, enabled: bool) {
        self.lightmap_enabled = enabled;
        if let Some(ground) = &mut self.ground_renderer {
            ground.set_lightmap_enabled(enabled);
        }
        self.global_uniforms
            .set_cell_light_enabled(&self.device.queue, enabled);
        let mut light = self.base_light;
        if !enabled {
            for c in light.ambient_color.iter_mut().take(3) {
                *c = (*c * 1.5).min(1.0);
            }
        }
        self.global_uniforms
            .update_light(&self.device.queue, &light);
    }

    pub fn set_fog(&mut self, fog: Option<FogEntry>) {
        let fog_uniform = match fog {
            Some(entry) => FogUniform {
                color: [entry.color[0], entry.color[1], entry.color[2], 1.0],
                near: entry.near * self.fog_scale,
                far: entry.far * self.fog_scale,
                factor: entry.factor,
                enabled: 1.0,
            },
            None => FogUniform::default(),
        };
        self.global_uniforms
            .update_fog(&self.device.queue, &fog_uniform);
    }

    pub fn load_map(
        &mut self,
        gnd: &GndFile,
        rsw: &RswFile,
        grf: &GrfArchive,
        fog: Option<FogEntry>,
    ) {
        self.fog_scale = 240.0 * gnd.zoom;
        self.set_fog(fog);

        let center_x = gnd.width as f32 * gnd.zoom / 2.0;
        let center_z = gnd.height as f32 * gnd.zoom / 2.0;
        self.camera.target = glam::Vec3::new(center_x, 0.0, center_z);

        let mut light = LightUniform::default();
        if let (Some(longitude), Some(latitude)) = (rsw.light.longitude, rsw.light.latitude) {
            let lon_rad = (longitude as f32).to_radians();
            let lat_rad = (latitude as f32).to_radians();
            let dir = glam::Vec3::new(
                -lon_rad.cos() * lat_rad.sin(),
                -lat_rad.cos(),
                -lon_rad.sin() * lat_rad.sin(),
            )
            .normalize();
            light.light_dir = [dir.x, dir.y, dir.z, 0.0];
        }
        if let Some(diffuse) = rsw.light.diffuse {
            light.diffuse_color = [diffuse[0], diffuse[1], diffuse[2], 1.0];
        }
        if let Some(ambient) = rsw.light.ambient {
            light.ambient_color = [ambient[0], ambient[1], ambient[2], 1.0];
        }
        if let Some(alpha) = rsw.light.shadow_map_alpha {
            light.shadow_strength = alpha;
        }
        self.base_light = light;
        self.global_uniforms
            .update_light(&self.device.queue, &light);

        let scale_factor = gnd.zoom / 10.0;
        let center_x = gnd.width as f32 * gnd.zoom / 2.0;
        let center_z = gnd.height as f32 * gnd.zoom / 2.0;
        let point_lights: Vec<PointLightGpu> = rsw
            .objects
            .iter()
            .filter_map(|obj| {
                if let RswObject::Light(l) = obj {
                    Some(PointLightGpu {
                        position: [
                            l.position[0] * scale_factor + center_x,
                            l.position[1] * scale_factor,
                            l.position[2] * scale_factor + center_z,
                            0.0,
                        ],
                        color_range: [l.color[0], l.color[1], l.color[2], l.range * scale_factor],
                    })
                } else {
                    None
                }
            })
            .collect();
        tracing::info!("Loaded {} RSW point lights", point_lights.len());
        self.global_uniforms.update_point_lights(
            &self.device.device,
            &self.device.queue,
            &point_lights,
        );

        let ground_renderer = GroundRenderer::from_gnd(
            gnd,
            grf,
            &self.device.device,
            &self.device.queue,
            &self.global_uniforms,
            &mut self.texture_cache,
            self.device.surface_format,
        );
        self.ground_renderer = Some(ground_renderer);
        self.global_uniforms.update_cell_light(
            &self.device.device,
            &self.device.queue,
            cell_light::CellLightMap::from_gnd(gnd).as_ref(),
            gnd.zoom,
        );
        self.set_lightmap_enabled(self.lightmap_enabled);

        let props = ModelRenderer::from_rsw(
            rsw,
            gnd,
            grf,
            &self.device.device,
            &self.device.queue,
            &self.global_uniforms,
            &mut self.texture_cache,
            self.device.surface_format,
        );
        self.model_renderer = props.static_models;
        self.animated_model_renderer = props.animated_models;

        self.water_renderer = WaterRenderer::from_water_settings(
            &rsw.water,
            gnd,
            grf,
            &self.device.device,
            &self.device.queue,
            &self.global_uniforms,
            &mut self.texture_cache,
            self.device.surface_format,
        );

        self.skill_unit_models.clear();
        self.gr2_models.clear();
        self.background_mode = BackgroundMode::RswMap;
    }

    pub fn has_skill_unit_model(&self, key: u32) -> bool {
        self.skill_unit_models.contains_key(&key)
    }

    pub fn add_skill_unit_model(
        &mut self,
        key: u32,
        rsm: &RsmFile,
        grf: &GrfArchive,
        world_pos: [f32; 3],
        scale_factor: f32,
    ) {
        if let Some(model) = ModelRenderer::from_rsm_at(
            rsm,
            grf,
            &self.device.device,
            &self.device.queue,
            &self.global_uniforms,
            &mut self.texture_cache,
            self.device.surface_format,
            world_pos,
            scale_factor,
        ) {
            self.skill_unit_models.insert(key, model);
        }
    }

    pub fn retain_skill_unit_models(&mut self, keep: &std::collections::HashSet<u32>) {
        self.skill_unit_models.retain(|k, _| keep.contains(k));
    }

    pub fn preload_effect_textures(&mut self, paths: &[String], grf: &GrfArchive) {
        let mut loaded: Vec<&str> = Vec::new();
        let mut missing: Vec<&str> = Vec::new();
        for path in paths {
            if self.texture_cache.get(path).is_some() {
                loaded.push(path);
                continue;
            }
            match texture::load_keyed_texture(
                path,
                grf,
                &self.device.device,
                &self.device.queue,
                &self.texture_cache.bind_group_layout,
            ) {
                Some((bind_group, w, h)) => {
                    self.texture_cache.insert(path, bind_group, w, h);
                    loaded.push(path);
                }
                None => missing.push(path),
            }
        }
        if ragnarok_profiling::debug::trace_texture_load() {
            tracing::info!(
                "[effect-preload] {} loaded, {} missing (of {} requested)",
                loaded.len(),
                missing.len(),
                paths.len()
            );
            for p in &loaded {
                tracing::info!("[effect-preload]   ok    {p}");
            }
            for p in &missing {
                tracing::info!("[effect-preload]   MISS  {p}");
            }
        }
    }

    pub fn preload_textures(&mut self, paths: &[&str], grf: &GrfArchive) -> bool {
        let mut all_loaded = true;
        for path in paths {
            if self
                .texture_cache
                .get_or_load(path, grf, &self.device.device, &self.device.queue, true)
                .is_none()
            {
                all_loaded = false;
            }
        }
        all_loaded
    }

    /// Builds a Graffiti message decal and registers it under `key`. Returns false
    /// when the alphabet atlas is missing from the GRF.
    pub fn build_graffiti_texture(&mut self, key: &str, message: &str, grf: &GrfArchive) -> bool {
        let Ok(bytes) = grf.read_file(graffiti::ALPHABET_TEXTURE) else {
            return false;
        };
        let Ok(atlas) = image::load_from_memory_with_format(&bytes, image::ImageFormat::Bmp) else {
            return false;
        };
        let mut atlas = atlas.to_rgba8();
        ragnarok_formats::apply_magenta_transparency(atlas.as_mut());
        let composed = graffiti::compose(&atlas, message);
        let (w, h) = (composed.width(), composed.height());
        let bind_group = texture::create_texture_bind_group_from_rgba(
            &self.device.device,
            &self.device.queue,
            composed.as_raw(),
            w,
            h,
            &self.texture_cache.bind_group_layout,
            key,
            wgpu::FilterMode::Linear,
            wgpu::TextureFormat::Rgba8Unorm,
            wgpu::AddressMode::ClampToEdge,
        );
        self.texture_cache.insert(key, bind_group, w, h);
        true
    }

    /// Changes the UI scale at runtime. The font atlas is re-rasterized at the
    /// new scale so text stays crisp; per-frame layout picks up `dpi_scale`.
    pub fn set_dpi_scale(&mut self, dpi_scale: f32) {
        if dpi_scale <= 0.0 || (dpi_scale - self.dpi_scale).abs() < f32::EPSILON {
            return;
        }
        self.dpi_scale = dpi_scale;
        self.font_atlas = FontAtlas::from_embedded(self.font_px_height, dpi_scale);
        self.font_atlas_bind_group = texture::create_font_atlas_bind_group(
            &self.device.device,
            &self.device.queue,
            &self.font_atlas.image,
            &self.texture_cache.bind_group_layout,
            "font_atlas",
        );
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.device.resize(width, height);
        if width > 0 && height > 0 {
            self.camera.aspect = width as f32 / height as f32;
            let logical_w = width as f32 / self.dpi_scale;
            let logical_h = height as f32 / self.dpi_scale;
            self.sprite_renderer
                .resize(&self.device.queue, logical_w, logical_h);
            // `effect_sprite_renderer` carries its own `SpriteUniforms`; if we
            // skip it here its screen_size diverges from the camera's
            // projection screen_w/h and billboards shift off-center after the
            // first window resize.
            self.effect_sprite_renderer
                .resize(&self.device.queue, logical_w, logical_h);
            self.ui_renderer
                .resize(&self.device.queue, logical_w, logical_h);
        }
    }

    pub fn enable_ground_proxy(&mut self) {
        if self.ground_proxy.is_some() {
            return;
        }
        let proxy = GroundProxyRenderer::new(
            &self.device.device,
            self.device.surface_format,
            &self.global_uniforms.bind_group_layout,
        );
        proxy.initialise(&self.device.queue);
        self.ground_proxy = Some(proxy);
    }

    pub fn render(&mut self, frame: FrameInputs) {
        ragnarok_profiling::profile_function!();
        let output = match self.device.surface.get_current_texture() {
            Ok(tex) => tex,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.device.reconfigure_surface();
                return;
            }
            Err(wgpu::SurfaceError::Timeout) => return,
            Err(e) => {
                tracing::error!("Surface error: {e}");
                return;
            }
        };

        let view = output.texture.create_view(&Default::default());
        let depth_view = self.device.depth_view.clone();
        let phys_w = self.device.surface_config.width;
        let phys_h = self.device.surface_config.height;
        let clear = self.clear_color;
        self.render_into(&view, &depth_view, phys_w, phys_h, clear, frame);
        output.present();
    }

    pub fn render_into(
        &mut self,
        color_view: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
        physical_w: u32,
        physical_h: u32,
        clear_color: wgpu::Color,
        frame: FrameInputs,
    ) {
        ragnarok_profiling::profile_function!();
        if physical_w == 0 || physical_h == 0 {
            return;
        }
        let FrameInputs {
            ui_draw_calls,
            effect_sprite_batches,
            effect_draws,
            sprite_particle_records,
            sprite_batches,
            silhouette_batches,
            cursor_batches,
            inline_textures,
            elapsed,
            delta,
        } = frame;
        let logical_w = physical_w as f32 / self.dpi_scale;
        let logical_h = physical_h as f32 / self.dpi_scale;
        self.camera.aspect = physical_w as f32 / physical_h as f32;
        self.sprite_renderer
            .resize(&self.device.queue, logical_w, logical_h);
        self.effect_sprite_renderer
            .resize(&self.device.queue, logical_w, logical_h);
        self.ui_renderer
            .resize(&self.device.queue, logical_w, logical_h);

        self.global_uniforms
            .update_camera(&self.device.queue, &self.camera);

        if let Some(water) = &self.water_renderer {
            water.update(&self.device.queue, elapsed);
        }

        if let Some(animated) = &mut self.animated_model_renderer {
            // A long stall must not fling props through their whole animation
            // in one step.
            animated.update(&self.device.queue, delta.clamp(0.0, 0.25));
        }

        let view = color_view;
        let mut encoder = self
            .device
            .device
            .create_command_encoder(&Default::default());

        {
            ragnarok_profiling::profile_scope!("scene-opaque");
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });

            match self.background_mode {
                BackgroundMode::RswMap => {
                    if let Some(ground) = &self.ground_renderer {
                        ragnarok_profiling::profile_scope!("ground");
                        ground.render(&mut pass, &self.global_uniforms, &self.texture_cache);
                    }
                    if let Some(model) = &self.model_renderer {
                        ragnarok_profiling::profile_scope!("model");
                        model.render(&mut pass, &self.global_uniforms, &self.texture_cache);
                    }
                    if let Some(animated) = &self.animated_model_renderer {
                        ragnarok_profiling::profile_scope!("animated-models");
                        animated.render(&mut pass, &self.global_uniforms, &self.texture_cache);
                    }
                    if !self.skill_unit_models.is_empty() {
                        ragnarok_profiling::profile_scope!("skill-unit-models");
                        for model in self.skill_unit_models.values() {
                            model.render(&mut pass, &self.global_uniforms, &self.texture_cache);
                        }
                    }
                    if !self.gr2_models.is_empty() {
                        ragnarok_profiling::profile_scope!("gr2-models");
                        for model in self.gr2_models.values() {
                            model.render(&mut pass, &self.global_uniforms);
                        }
                    }
                    if let Some(grid) = &self.grid_selector {
                        grid.render(&mut pass, &self.global_uniforms, &self.texture_cache);
                    }
                }
                BackgroundMode::GroundProxy => {
                    if let Some(proxy) = &self.ground_proxy {
                        proxy.render(&mut pass, &self.global_uniforms, &self.camera);
                    }
                }
                BackgroundMode::Clear => {}
            }
        }

        if !effect_draws.behind.is_empty() {
            ragnarok_profiling::profile_scope!("effect-behind");
            let behind_list = effect_draws.behind_as_list();
            let behind_records = build_effect_records(
                &behind_list,
                &self.camera,
                &self.texture_cache,
                &self.white_bind_group,
                logical_w,
                logical_h,
                &self.effect_primitives,
            );
            if !behind_records.is_empty() {
                self.effect_dispatcher.dispatch(
                    behind_records,
                    &mut encoder,
                    &view,
                    depth_view,
                    &self.device.device,
                    &self.device.queue,
                    &self.global_uniforms.bind_group,
                    &self.effect_sprite_renderer.uniform_bind_group,
                    &self.effect_sprite_renderer,
                    &self.effect_primitives,
                );
            }
        }

        if !sprite_batches.is_empty() {
            ragnarok_profiling::profile_scope!("sprite");
            self.sprite_renderer.render(
                &mut encoder,
                &view,
                Some(depth_view),
                &self.device.device,
                &self.device.queue,
                None,
                sprite_batches,
            );
        }

        // Stamp the flat-depth body silhouette after the colour pass so the
        // effect passes below occlude against the body (effects above the feet
        // draw on top; ground effects at the feet are hidden).
        if !silhouette_batches.is_empty() {
            ragnarok_profiling::profile_scope!("silhouette");
            self.sprite_renderer.render_silhouette(
                &mut encoder,
                &view,
                depth_view,
                &self.device.device,
                &self.device.queue,
                silhouette_batches,
            );
        }

        // Water draws after the silhouette, never after the colour pass: the colour
        // pass writes no depth, so at that point the body's pixels still hold the
        // depth of the ground behind it and the surface would swallow the whole
        // sprite. Against the silhouette's flat feet depth the surface cuts the body
        // at the waterline instead, so a character wading in deep water is submerged
        // further than one in the shallows.
        if let (BackgroundMode::RswMap, Some(water)) = (self.background_mode, &self.water_renderer)
        {
            ragnarok_profiling::profile_scope!("water");
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("water"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            water.render(
                &mut pass,
                &self.global_uniforms,
                &self.texture_cache,
                elapsed,
            );
        }

        if !effect_sprite_batches.is_empty() {
            ragnarok_profiling::profile_scope!("effect-sprite");
            self.effect_sprite_renderer.render(
                &mut encoder,
                &view,
                Some(depth_view),
                &self.device.device,
                &self.device.queue,
                None,
                effect_sprite_batches,
            );
        }

        let mut records: Vec<DrawRecord<'_>> = {
            ragnarok_profiling::profile_scope!("effect-build");
            build_effect_records(
                effect_draws,
                &self.camera,
                &self.texture_cache,
                &self.white_bind_group,
                logical_w,
                logical_h,
                &self.effect_primitives,
            )
        };
        records.extend(sprite_particle_records);
        if !records.is_empty() {
            ragnarok_profiling::profile_scope!("effect-dispatch");
            self.effect_dispatcher.dispatch(
                records,
                &mut encoder,
                &view,
                depth_view,
                &self.device.device,
                &self.device.queue,
                &self.global_uniforms.bind_group,
                &self.effect_sprite_renderer.uniform_bind_group,
                &self.effect_sprite_renderer,
                &self.effect_primitives,
            );
        }

        {
            ragnarok_profiling::profile_scope!("submit-scene");
            self.device.queue.submit(std::iter::once(encoder.finish()));
        }

        let mut encoder = self
            .device
            .device
            .create_command_encoder(&Default::default());

        if !ui_draw_calls.is_empty() {
            ragnarok_profiling::profile_scope!("ui");
            let resolved: Vec<UiDrawCommand> = ui_draw_calls
                .iter()
                .map(|call| {
                    let bind_group = match &call.texture {
                        UiTextureRef::FontAtlas => &self.font_atlas_bind_group,
                        UiTextureRef::White => &self.white_bind_group,
                        UiTextureRef::Named(name) => self
                            .texture_cache
                            .get(name)
                            .unwrap_or(&self.white_bind_group),
                        UiTextureRef::Inline(idx) => inline_textures[*idx],
                    };
                    UiDrawCommand {
                        vertices: &call.vertices,
                        indices: &call.indices,
                        texture: bind_group,
                    }
                })
                .collect();

            self.ui_renderer.render(
                &mut encoder,
                &view,
                &self.device.device,
                &self.device.queue,
                &resolved,
            );
        }

        if !cursor_batches.is_empty() {
            ragnarok_profiling::profile_scope!("cursor");
            self.sprite_renderer.render(
                &mut encoder,
                &view,
                None,
                &self.device.device,
                &self.device.queue,
                None,
                cursor_batches,
            );
        }

        {
            ragnarok_profiling::profile_scope!("submit-ui");
            self.device.queue.submit(std::iter::once(encoder.finish()));
        }
    }

    pub fn try_load_grf_font(&mut self, grf: &GrfArchive) {
        let extra_chars = font_atlas::euc_kr_charset();
        let font_paths = [
            "data/Font/NanumBarunGothicBold.ttf",
            "data/Font/NanumBarunGothic.ttf",
        ];
        for path in &font_paths {
            if let Ok(data) = grf.read_file(path) {
                self.set_font_atlas(FontAtlas::build_with_extra_chars(
                    &data,
                    self.font_px_height,
                    self.dpi_scale,
                    &extra_chars,
                ));
                tracing::info!("Loaded GRF font: {path}");
                return;
            }
        }
        self.set_font_atlas(FontAtlas::from_embedded_cjk(
            self.font_px_height,
            self.dpi_scale,
            &extra_chars,
        ));
        tracing::info!("No GRF font found, using embedded CJK font for Korean text");
    }

    fn set_font_atlas(&mut self, atlas: FontAtlas) {
        self.font_atlas = atlas;
        self.font_atlas_bind_group = texture::create_font_atlas_bind_group(
            &self.device.device,
            &self.device.queue,
            &self.font_atlas.image,
            &self.texture_cache.bind_group_layout,
            "font_atlas",
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn background_mode_default_is_rsw_map() {
        assert_eq!(BackgroundMode::default(), BackgroundMode::RswMap);
    }

    #[test]
    fn background_mode_cycle_round_trip() {
        let mut mode = BackgroundMode::default();
        let cycle = [
            BackgroundMode::GroundProxy,
            BackgroundMode::Clear,
            BackgroundMode::RswMap,
        ];
        for expected in cycle {
            mode = match mode {
                BackgroundMode::RswMap => BackgroundMode::GroundProxy,
                BackgroundMode::GroundProxy => BackgroundMode::Clear,
                BackgroundMode::Clear => BackgroundMode::RswMap,
            };
            assert_eq!(mode, expected);
        }
    }
}
