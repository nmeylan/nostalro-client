pub mod camera;
pub mod damage_number;
mod device;
pub mod effect;
pub mod effect_sprite;
pub mod font_atlas;
pub mod global_uniforms;
pub mod grid_selector;
pub mod ground;
pub mod ground_proxy;
pub mod model;
pub mod sprite;
pub mod sprite_projection;
pub mod texture;
pub mod ui_renderer;
pub mod water;

pub use camera::Camera;
pub use device::{RenderDevice, block_on};
pub use global_uniforms::{FogUniform, GlobalUniforms, LightUniform, PointLightGpu};

pub use damage_number::render_damage_number_quads;
pub use effect_sprite::{
    EffectSpriteCache, EffectSpriteEntry, EmitterDraw, Smoke3DParticle, SpriteEffectEmitter,
    build_emitter_batches, collect_sprite_effect_draws, prepare_sprite_particle_records,
    project_billboard,
};
pub use font_atlas::FontAtlas;
pub use grid_selector::GridSelectorRenderer;
pub use ground::GroundRenderer;
pub use ground_proxy::GroundProxyRenderer;
pub use model::ModelRenderer;
pub use sprite::{
    ClipQuad, CompositeClips, EntitySprite, SpriteBatch, SpriteRenderer, SpriteTextures,
    SpriteUniforms, SpriteVertex, build_clip_quad, build_composite_clips, build_entity_sprite,
    scale_clip_vertices, upload_sprite_textures,
};
pub use effect::{
    BlendBucket, BlendKind, DrawRecord, EffectDispatcher, PipelineKind, StrEffectCache,
    StrEffectEntry, StrEmitterInput, build_str_effect_batches, d3d_blend_to_wgpu,
    prepare_billboard_records, prepare_frustum_records, prepare_ground_disc_records,
    prepare_quad_horn_records, prepare_sphere_records, prepare_world_quad_records,
};
pub use texture::TextureCache;
pub use ui_renderer::{UiDrawCommand, UiRenderer, UiVertex};
pub use water::WaterRenderer;
pub use wgpu;

use ragnarok_formats::fog_table::FogEntry;
use ragnarok_formats::gnd::GndFile;
use ragnarok_formats::grf::GrfArchive;
use ragnarok_formats::rsw::{RswFile, RswObject};
use std::sync::Arc;

/// Selects which floor (if any) the main pass renders. `RswMap` uses the
/// real `ground_renderer`; `GroundProxy` uses the debug checker floor;
/// `Clear` skips both so only `clear_color` shows through. Tooling like
/// the unified viewer toggles this at runtime; the game and `rsw_viewer`
/// rely on the default (`RswMap`).
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

/// Texture reference used by UI draw calls, resolved to bind groups at render time.
pub enum UiTextureRef {
    FontAtlas,
    White,
    Named(String),
    /// Index into the `inline_textures` slice passed to `Renderer::render()`.
    Inline(usize),
}

/// Owned UI draw command produced by the UI layer.
pub struct UiDrawCall {
    pub vertices: Vec<UiVertex>,
    pub indices: Vec<u32>,
    pub texture: UiTextureRef,
}

pub struct Renderer {
    pub device: RenderDevice,
    pub camera: Camera,
    pub global_uniforms: GlobalUniforms,
    pub texture_cache: TextureCache,
    pub ground_renderer: Option<GroundRenderer>,
    /// Debug/tooling stand-in for the real ground; renders when
    /// `background_mode == GroundProxy`. Provides a floor for effect
    /// primitives to depth-clip against when no real `.gnd` is loaded.
    pub ground_proxy: Option<GroundProxyRenderer>,
    pub model_renderer: Option<ModelRenderer>,
    pub water_renderer: Option<WaterRenderer>,
    pub grid_selector: Option<GridSelectorRenderer>,
    pub sprite_renderer: SpriteRenderer,
    /// Dedicated sprite-pipeline instance for the effect-world pass. Owns its
    /// own vertex/index buffers so we can render effects + entities in the
    /// same encoder without an intermediate submit. Will be retired in C-2
    /// when effects move to dedicated primitive pipelines.
    pub effect_sprite_renderer: SpriteRenderer,
    pub effect_ground_disc_renderer: effect::GroundDiscRenderer,
    pub effect_frustum_renderer: effect::FrustumRenderer,
    pub effect_quad_horn_renderer: effect::QuadHornRenderer,
    pub effect_sphere_renderer: effect::SphereRenderer,
    pub effect_world_quad_renderer: effect::WorldQuadRenderer,
    /// Owns the per-frame unified vertex / index buffer for the effect
    /// dispatch path; pipelines themselves live on the per-primitive
    /// renderer structs above.
    pub effect_dispatcher: effect::EffectDispatcher,
    pub ui_renderer: UiRenderer,
    pub font_atlas: FontAtlas,
    pub font_atlas_bind_group: wgpu::BindGroup,
    pub white_bind_group: wgpu::BindGroup,
    pub font_px_height: f32,
    pub dpi_scale: f32,
    pub clear_color: wgpu::Color,
    pub background_mode: BackgroundMode,
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

        // Entity sprites write depth so the post-sprite effect pass
        // depth-tests against the sprite — the entity sprite pass keeps
        // depth-write enabled, matching the original game's on-screen
        // occlusion. Front-facing fragments of a translucent
        // cylinder around the caster pass `LessEqual` and draw on top of
        // the sprite; back-facing fragments fail and the sprite remains
        // visible (sprite "in the middle" of the cylinder).
        let sprite_renderer = SpriteRenderer::new(
            &device.device,
            device.surface_format,
            &texture_cache.bind_group_layout,
            logical_w,
            logical_h,
            include_str!("shaders/sprite.wgsl"),
            true,
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
        let effect_ground_disc_renderer = effect::GroundDiscRenderer::new(
            &device.device,
            device.surface_format,
            &global_uniforms.bind_group_layout,
            &texture_cache.bind_group_layout,
        );
        let effect_frustum_renderer = effect::FrustumRenderer::new(
            &device.device,
            device.surface_format,
            &global_uniforms.bind_group_layout,
            &texture_cache.bind_group_layout,
        );
        let effect_quad_horn_renderer = effect::QuadHornRenderer::new(
            &device.device,
            device.surface_format,
            &global_uniforms.bind_group_layout,
            &texture_cache.bind_group_layout,
        );
        let effect_sphere_renderer = effect::SphereRenderer::new(
            &device.device,
            device.surface_format,
            &global_uniforms.bind_group_layout,
            &texture_cache.bind_group_layout,
        );
        let effect_world_quad_renderer = effect::WorldQuadRenderer::new(
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
            water_renderer: None,
            grid_selector: None,
            sprite_renderer,
            effect_sprite_renderer,
            effect_ground_disc_renderer,
            effect_frustum_renderer,
            effect_quad_horn_renderer,
            effect_sphere_renderer,
            effect_world_quad_renderer,
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
        }
    }

    pub fn set_background_mode(&mut self, mode: BackgroundMode) {
        self.background_mode = mode;
    }

    pub fn load_map(
        &mut self,
        gnd: &GndFile,
        rsw: &RswFile,
        grf: &GrfArchive,
        fog: Option<FogEntry>,
    ) {
        let scale = 240.0 * gnd.zoom;
        let fog_uniform = match fog {
            Some(entry) => FogUniform {
                color: [entry.color[0], entry.color[1], entry.color[2], 1.0],
                near: entry.near * scale,
                far: entry.far * scale,
                factor: entry.factor,
                enabled: 1.0,
            },
            None => FogUniform::default(),
        };
        self.global_uniforms
            .update_fog(&self.device.queue, &fog_uniform);

        // Set camera target to map center
        let center_x = gnd.width as f32 * gnd.zoom / 2.0;
        let center_z = gnd.height as f32 * gnd.zoom / 2.0;
        self.camera.target = glam::Vec3::new(center_x, 0.0, center_z);

        // Apply RSW light settings
        if let (Some(longitude), Some(latitude)) = (rsw.light.longitude, rsw.light.latitude) {
            let lon_rad = (longitude as f32).to_radians();
            let lat_rad = (latitude as f32).to_radians();
            let dir = glam::Vec3::new(
                -lon_rad.cos() * lat_rad.sin(),
                -lat_rad.cos(),
                -lon_rad.sin() * lat_rad.sin(),
            )
            .normalize();

            let mut light = LightUniform::default();
            light.light_dir = [dir.x, dir.y, dir.z, 0.0];
            if let Some(diffuse) = rsw.light.diffuse {
                light.diffuse_color = [diffuse[0], diffuse[1], diffuse[2], 1.0];
            }
            if let Some(ambient) = rsw.light.ambient {
                light.ambient_color = [ambient[0], ambient[1], ambient[2], 1.0];
            }
            if let Some(alpha) = rsw.light.shadow_map_alpha {
                light.shadow_strength = alpha;
            }
            self.global_uniforms
                .update_light(&self.device.queue, &light);
        }

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

        self.model_renderer = ModelRenderer::from_rsw(
            rsw,
            gnd,
            grf,
            &self.device.device,
            &self.device.queue,
            &self.global_uniforms,
            &mut self.texture_cache,
            self.device.surface_format,
        );

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

        self.background_mode = BackgroundMode::RswMap;
    }

    /// Load each `data/texture/effect/<name>` entry into the texture cache,
    /// applying the keyed-transparency conventions used by STR effect textures
    /// (magenta → transparent, pure black → transparent for additive
    /// rendering). Missing files log once via `tracing::warn` and skip.
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
        eprintln!(
            "[effect-preload] {} loaded, {} missing (of {} requested)",
            loaded.len(),
            missing.len(),
            paths.len()
        );
        for p in &loaded {
            eprintln!("[effect-preload]   ok    {p}");
        }
        for p in &missing {
            eprintln!("[effect-preload]   MISS  {p}");
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

    /// Install a debug checker floor at `y = 0`. Whether it actually
    /// renders is controlled by `background_mode`; callers usually pair
    /// this with `set_background_mode(BackgroundMode::GroundProxy)`.
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

    pub fn render(
        &mut self,
        ui_draw_calls: &[UiDrawCall],
        effect_sprite_batches: &[SpriteBatch],
        effect_draws: &effect::EffectDrawList,
        sprite_particle_records: Vec<DrawRecord>,
        sprite_batches: &[SpriteBatch],
        cursor_batches: &[SpriteBatch],
        inline_textures: &[&wgpu::BindGroup],
        elapsed: f32,
    ) {
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
        self.render_into(
            &view,
            &depth_view,
            phys_w,
            phys_h,
            clear,
            ui_draw_calls,
            effect_sprite_batches,
            effect_draws,
            sprite_particle_records,
            sprite_batches,
            cursor_batches,
            inline_textures,
            elapsed,
        );
        output.present();
    }

    /// Render a full frame to caller-provided color + depth views. Used by
    /// `render()` for the surface path and by offline capture (gif export)
    /// for an in-memory target. The render pipelines are baked against
    /// `self.device.surface_format`, so offscreen color targets must use
    /// that format too.
    pub fn render_into(
        &mut self,
        color_view: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
        physical_w: u32,
        physical_h: u32,
        clear_color: wgpu::Color,
        ui_draw_calls: &[UiDrawCall],
        effect_sprite_batches: &[SpriteBatch],
        effect_draws: &effect::EffectDrawList,
        sprite_particle_records: Vec<DrawRecord>,
        sprite_batches: &[SpriteBatch],
        cursor_batches: &[SpriteBatch],
        inline_textures: &[&wgpu::BindGroup],
        elapsed: f32,
    ) {
        if physical_w == 0 || physical_h == 0 {
            return;
        }
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

        let view = color_view;
        let mut encoder = self
            .device
            .device
            .create_command_encoder(&Default::default());

        {
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
                        ground.render(&mut pass, &self.global_uniforms, &self.texture_cache);
                    }
                    if let Some(model) = &self.model_renderer {
                        model.render(&mut pass, &self.global_uniforms, &self.texture_cache);
                    }
                    if let Some(grid) = &self.grid_selector {
                        grid.render(&mut pass, &self.global_uniforms, &self.texture_cache);
                    }
                    if let Some(water) = &self.water_renderer {
                        water.render(
                            &mut pass,
                            &self.global_uniforms,
                            &self.texture_cache,
                            elapsed,
                        );
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

        // Entity sprites render before the unified effect pass so effects
        // sit on top of the character — matches the original game's draw
        // order, where the player is drawn first and effects spawned on it
        // layer over the top.
        if !sprite_batches.is_empty() {
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

        // STR + ambient SPR / Smoke3D sprite batches go through the
        // dedicated effect sprite renderer pass. They share blend state
        // (additive vs alpha) but stay outside the unified queue: STR runs
        // its own keyframe animation system and ambient emitters aren't
        // emitted into `EffectDrawList` today, so they don't have a
        // per-primitive depth to sort by. The dedicated pass keeps them
        // off the unified vertex buffer (avoids the cost of rebuilding
        // their geometry every dispatch).
        if !effect_sprite_batches.is_empty() {
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

        // Unified effect-primitive pass: every Billboard / BillboardDisc /
        // SpriteParticle / Frustum / GroundDisc / QuadHorn / Sphere /
        // WorldQuad in `effect_draws` lands in one of [`BlendBucket`]'s
        // deferred lists, sorted back-to-front by view-space depth, then
        // flushed in alpha → additive → multiply order. Matches the
        // original game's `FlushBatch`.
        let texture_lookup = |name: &str| -> Option<&wgpu::BindGroup> {
            if name.is_empty() {
                return None;
            }
            let full = format!("data/texture/effect/{name}");
            self.texture_cache.get(&full)
        };
        let mut records: Vec<DrawRecord<'_>> = Vec::new();
        records.extend(sprite_particle_records);
        records.extend(prepare_billboard_records(
            effect_draws,
            &self.camera,
            logical_w,
            logical_h,
            &self.white_bind_group,
            texture_lookup,
        ));
        records.extend(prepare_frustum_records(
            effect_draws,
            &self.camera,
            &self.white_bind_group,
            texture_lookup,
        ));
        records.extend(prepare_ground_disc_records(
            effect_draws,
            &self.camera,
            &self.white_bind_group,
            texture_lookup,
        ));
        records.extend(prepare_quad_horn_records(
            effect_draws,
            &self.camera,
            &self.white_bind_group,
            texture_lookup,
        ));
        records.extend(prepare_sphere_records(
            effect_draws,
            &self.camera,
            &self.white_bind_group,
            texture_lookup,
        ));
        records.extend(prepare_world_quad_records(
            effect_draws,
            &self.camera,
            &self.white_bind_group,
            texture_lookup,
        ));
        // SpriteParticle records reference textures inside their respective
        // EffectSpriteCache entries (`&sprite.textures.bind_groups[i]`), which
        // the renderer doesn't own. Callers that want SpriteParticle
        // dispatched today pass the records in `extra_effect_records`;
        // future cleanup will hoist particle preparation into
        // `compose_effect_frame`.
        if !records.is_empty() {
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
                &self.effect_frustum_renderer,
                &self.effect_ground_disc_renderer,
                &self.effect_quad_horn_renderer,
                &self.effect_sphere_renderer,
                &self.effect_world_quad_renderer,
            );
        }

        // Submit 3D + effects + sprites so sprite_renderer's write_buffer is
        // flushed before cursor reuses the same buffers.
        self.device.queue.submit(std::iter::once(encoder.finish()));

        let mut encoder = self
            .device
            .device
            .create_command_encoder(&Default::default());

        if !ui_draw_calls.is_empty() {
            // Resolve TextureRef -> &wgpu::BindGroup using field-level borrow splitting
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

        self.device.queue.submit(std::iter::once(encoder.finish()));
    }

    pub fn try_load_grf_font(&mut self, grf: &GrfArchive) {
        let font_paths = [
            "data/Font/NanumBarunGothicBold.ttf",
            "data/Font/NanumBarunGothic.ttf",
        ];
        for path in &font_paths {
            if let Ok(data) = grf.read_file(path) {
                self.font_atlas = FontAtlas::build(&data, self.font_px_height, self.dpi_scale);
                self.font_atlas_bind_group = texture::create_font_atlas_bind_group(
                    &self.device.device,
                    &self.device.queue,
                    &self.font_atlas.image,
                    &self.texture_cache.bind_group_layout,
                    "font_atlas",
                );
                tracing::info!("Loaded GRF font: {path}");
                return;
            }
        }
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
        // The unified viewer's B key advances RswMap -> GroundProxy -> Clear ->
        // back. The renderer doesn't own the cycling, but the order matters
        // because clients read this enum directly.
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
