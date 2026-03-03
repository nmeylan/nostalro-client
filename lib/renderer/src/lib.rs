mod device;
pub mod camera;
pub mod font_atlas;
pub mod global_uniforms;
pub mod ground;
pub mod model;
pub mod sprite;
pub mod texture;
pub mod ui_renderer;
pub mod water;

pub use device::RenderDevice;
pub use camera::Camera;
pub use global_uniforms::{GlobalUniforms, LightUniform};
pub use ground::GroundRenderer;
pub use model::ModelRenderer;
pub use water::WaterRenderer;
pub use texture::TextureCache;
pub use font_atlas::FontAtlas;
pub use ui_renderer::{UiRenderer, UiVertex, UiDrawCommand};

use ragnarok_formats::gnd::GndFile;
use ragnarok_formats::grf::GrfArchive;
use ragnarok_formats::rsw::RswFile;
use std::sync::Arc;

/// Texture reference used by UI draw calls, resolved to bind groups at render time.
pub enum UiTextureRef {
    FontAtlas,
    White,
    Named(String),
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
    pub ui_renderer: UiRenderer,
    pub font_atlas: FontAtlas,
    pub font_atlas_bind_group: wgpu::BindGroup,
    pub white_bind_group: wgpu::BindGroup,
}

impl Renderer {
    pub async fn new(window: Arc<winit::window::Window>) -> Self {
        let device = RenderDevice::new(window).await;
        let camera = Camera {
            aspect: device.surface_config.width as f32 / device.surface_config.height as f32,
            ..Default::default()
        };
        let global_uniforms = GlobalUniforms::new(&device.device);
        let texture_cache = TextureCache::new(&device.device);

        let font_atlas = FontAtlas::from_embedded(16.0);
        let font_atlas_bind_group = texture::create_texture_bind_group_nearest(
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

        let ui_renderer = UiRenderer::new(
            &device.device,
            device.surface_format,
            &texture_cache.bind_group_layout,
            device.surface_config.width,
            device.surface_config.height,
        );

        Self {
            device,
            camera,
            global_uniforms,
            texture_cache,
            ground_renderer: None,
            model_renderer: None,
            water_renderer: None,
            ui_renderer,
            font_atlas,
            font_atlas_bind_group,
            white_bind_group,
        }
    }

    pub fn load_map(&mut self, gnd: &GndFile, rsw: &RswFile, grf: &GrfArchive) {
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

    pub fn resize(&mut self, width: u32, height: u32) {
        self.device.resize(width, height);
        if width > 0 && height > 0 {
            self.camera.aspect = width as f32 / height as f32;
            self.ui_renderer.resize(&self.device.queue, width, height);
        }
    }

    pub fn render(&mut self, ui_draw_calls: &[UiDrawCall], elapsed: f32) {
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
            if let Some(water) = &self.water_renderer {
                water.render(&mut pass, &self.global_uniforms, &self.texture_cache, elapsed);
            }
        }

        if !ui_draw_calls.is_empty() {
            // Resolve TextureRef -> &wgpu::BindGroup using field-level borrow splitting
            let resolved: Vec<UiDrawCommand> = ui_draw_calls
                .iter()
                .map(|call| {
                    let bind_group = match &call.texture {
                        UiTextureRef::FontAtlas => &self.font_atlas_bind_group,
                        UiTextureRef::White => &self.white_bind_group,
                        UiTextureRef::Named(name) => {
                            self.texture_cache.get(name)
                                .unwrap_or(&self.white_bind_group)
                        }
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

        self.device
            .queue
            .submit(std::iter::once(encoder.finish()));
        output.present();
    }

    pub fn try_load_grf_font(&mut self, grf: &GrfArchive) {
        let font_paths = [
            "data/Font/NanumBarunGothicBold.ttf",
            "data/Font/NanumBarunGothic.ttf",
        ];
        for path in &font_paths {
            if let Ok(data) = grf.read_file(path) {
                self.font_atlas = FontAtlas::build(&data, 16.0);
                self.font_atlas_bind_group = texture::create_texture_bind_group_nearest(
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
