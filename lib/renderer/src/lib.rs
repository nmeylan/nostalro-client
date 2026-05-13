pub mod camera;
pub mod damage_number;
mod device;
pub mod effect;
pub mod effect_sprite;
pub mod font_atlas;
pub mod global_uniforms;
pub mod grid_selector;
pub mod ground;
pub mod model;
pub mod sprite;
pub mod texture;
pub mod ui_renderer;
pub mod water;

pub use camera::Camera;
pub use device::{RenderDevice, block_on};
pub use global_uniforms::{FogUniform, GlobalUniforms, LightUniform, PointLightGpu};

pub use damage_number::render_damage_number_quads;
pub use effect_sprite::{
    EffectSpriteCache, EffectSpriteEntry, EmitterDraw, SpriteEffectEmitter, build_emitter_batches,
    collect_sprite_effect_draws, project_billboard,
};
pub use font_atlas::FontAtlas;
pub use grid_selector::GridSelectorRenderer;
pub use ground::GroundRenderer;
pub use model::ModelRenderer;
pub use sprite::{
    ClipQuad, CompositeClips, EntitySprite, SpriteBatch, SpriteRenderer, SpriteTextures,
    SpriteUniforms, SpriteVertex, build_clip_quad, build_composite_clips, build_entity_sprite,
    scale_clip_vertices, upload_sprite_textures,
};
pub use effect::{
    BlendKind, StrEffectCache, StrEffectEntry, StrEmitterInput, build_str_effect_batches,
    d3d_blend_to_wgpu,
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
    pub model_renderer: Option<ModelRenderer>,
    pub water_renderer: Option<WaterRenderer>,
    pub grid_selector: Option<GridSelectorRenderer>,
    pub sprite_renderer: SpriteRenderer,
    /// Dedicated sprite-pipeline instance for the effect-world pass. Owns its
    /// own vertex/index buffers so we can render effects + entities in the
    /// same encoder without an intermediate submit. Will be retired in C-2
    /// when effects move to dedicated primitive pipelines.
    pub effect_sprite_renderer: SpriteRenderer,
    pub effect_ring_renderer: effect::RingRenderer,
    pub ui_renderer: UiRenderer,
    pub font_atlas: FontAtlas,
    pub font_atlas_bind_group: wgpu::BindGroup,
    pub white_bind_group: wgpu::BindGroup,
    pub font_px_height: f32,
    pub dpi_scale: f32,
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

        let sprite_renderer = SpriteRenderer::new(
            &device.device,
            device.surface_format,
            &texture_cache.bind_group_layout,
            logical_w,
            logical_h,
            include_str!("shaders/sprite.wgsl"),
        );
        let effect_sprite_renderer = SpriteRenderer::new(
            &device.device,
            device.surface_format,
            &texture_cache.bind_group_layout,
            logical_w,
            logical_h,
            include_str!("shaders/sprite.wgsl"),
        );
        let effect_ring_renderer = effect::RingRenderer::new(
            &device.device,
            device.surface_format,
            &global_uniforms.bind_group_layout,
            &texture_cache.bind_group_layout,
        );

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
            model_renderer: None,
            water_renderer: None,
            grid_selector: None,
            sprite_renderer,
            effect_sprite_renderer,
            effect_ring_renderer,
            ui_renderer,
            font_atlas,
            font_atlas_bind_group,
            white_bind_group,
            font_px_height,
            dpi_scale,
        }
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
            self.ui_renderer
                .resize(&self.device.queue, logical_w, logical_h);
        }
    }

    pub fn render(
        &mut self,
        ui_draw_calls: &[UiDrawCall],
        effect_sprite_batches: &[SpriteBatch],
        effect_draws: &effect::EffectDrawList,
        sprite_batches: &[SpriteBatch],
        cursor_batches: &[SpriteBatch],
        inline_textures: &[&wgpu::BindGroup],
        elapsed: f32,
    ) {
        self.global_uniforms
            .update_camera(&self.device.queue, &self.camera);

        if let Some(water) = &self.water_renderer {
            water.update(&self.device.queue, elapsed);
        }

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
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.392,
                            g: 0.584,
                            b: 0.929,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.device.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });

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

        // Effect-world pass: STR + custom-fx primitives, depth-read no
        // depth-write, between the 3D pass and the entity sprite pass.
        // Uses a dedicated SpriteRenderer instance so its vertex buffer
        // doesn't collide with the entity pass below in the same encoder.
        if !effect_sprite_batches.is_empty() {
            self.effect_sprite_renderer.render(
                &mut encoder,
                &view,
                Some(&self.device.depth_view),
                &self.device.device,
                &self.device.queue,
                None,
                effect_sprite_batches,
            );
        }

        // Dispatch dedicated primitives from the effect draw list. Ring is
        // first; later primitives (Cylinder, LineStrip, …) get their own
        // dispatch block here in subsequent slices.
        let has_rings = effect_draws
            .primitives
            .iter()
            .any(|p| matches!(p, effect::EffectPrimitiveDraw::Ring { .. }));
        if has_rings {
            let texture_cache = &self.texture_cache;
            let fallback = &self.white_bind_group;
            self.effect_ring_renderer.render(
                &mut encoder,
                &view,
                &self.device.depth_view,
                &self.device.device,
                &self.device.queue,
                &self.global_uniforms.bind_group,
                &self.camera,
                effect_draws,
                fallback,
                |name| {
                    // Effect texture params store the bare filename
                    // (e.g. `magic_target.tga`); preload + cache uses the
                    // full GRF path (`data/texture/effect/<name>`).
                    if name.is_empty() {
                        return None;
                    }
                    let full = format!("data/texture/effect/{name}");
                    texture_cache.get(&full)
                },
            );
        }

        if !sprite_batches.is_empty() {
            self.sprite_renderer.render(
                &mut encoder,
                &view,
                Some(&self.device.depth_view),
                &self.device.device,
                &self.device.queue,
                None,
                sprite_batches,
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
        output.present();
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
