mod device;
pub mod camera;
pub mod global_uniforms;
pub mod ground;
pub mod texture;

pub use device::RenderDevice;
pub use camera::Camera;
pub use global_uniforms::{GlobalUniforms, LightUniform};
pub use ground::GroundRenderer;
pub use texture::TextureCache;

use ragnarok_formats::gnd::GndFile;
use ragnarok_formats::grf::GrfArchive;
use ragnarok_formats::rsw::RswFile;
use std::sync::Arc;

pub struct Renderer {
    pub device: RenderDevice,
    pub camera: Camera,
    pub global_uniforms: GlobalUniforms,
    pub texture_cache: TextureCache,
    pub ground_renderer: Option<GroundRenderer>,
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
        Self {
            device,
            camera,
            global_uniforms,
            texture_cache,
            ground_renderer: None,
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
                lat_rad.cos() * lon_rad.sin(),
                lat_rad.sin(),
                lat_rad.cos() * lon_rad.cos(),
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
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.device.resize(width, height);
        if width > 0 && height > 0 {
            self.camera.aspect = width as f32 / height as f32;
        }
    }

    pub fn render(&mut self) {
        self.global_uniforms
            .update_camera(&self.device.queue, &self.camera);

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
        }

        self.device
            .queue
            .submit(std::iter::once(encoder.finish()));
        output.present();
    }
}
