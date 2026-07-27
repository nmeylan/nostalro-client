use crate::camera::{Camera, CameraUniform};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightUniform {
    pub light_dir: [f32; 4],
    pub diffuse_color: [f32; 4],
    pub ambient_color: [f32; 4],
    pub shadow_strength: f32,
    pub _pad: [f32; 3],
}

impl Default for LightUniform {
    fn default() -> Self {
        Self {
            light_dir: [0.0, -1.0, 0.0, 0.0],
            diffuse_color: [1.0, 1.0, 1.0, 1.0],
            ambient_color: [0.3, 0.3, 0.3, 1.0],
            shadow_strength: 1.0,
            _pad: [0.0; 3],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PointLightGpu {
    pub position: [f32; 4],
    pub color_range: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct FogUniform {
    pub color: [f32; 4],
    pub near: f32,
    pub far: f32,
    pub factor: f32,
    pub enabled: f32,
}

impl Default for FogUniform {
    fn default() -> Self {
        Self {
            color: [0.0; 4],
            near: 0.0,
            far: 1.0,
            factor: 0.0,
            enabled: 0.0,
        }
    }
}

/// Maps a world XZ to a texel of the per-cell light texture. `enabled` is 0 when
/// the map has no lightmap or the player turned lightmaps off.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CellLightUniform {
    pub cell_size: f32,
    pub width: f32,
    pub height: f32,
    pub enabled: f32,
}

impl Default for CellLightUniform {
    fn default() -> Self {
        Self {
            cell_size: 1.0,
            width: 1.0,
            height: 1.0,
            enabled: 0.0,
        }
    }
}

pub struct GlobalUniforms {
    pub camera_buffer: wgpu::Buffer,
    pub light_buffer: wgpu::Buffer,
    pub point_light_buffer: wgpu::Buffer,
    pub fog_buffer: wgpu::Buffer,
    pub point_light_capacity: usize,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
    cell_light_buffer: wgpu::Buffer,
    cell_light_view: wgpu::TextureView,
    cell_light_sampler: wgpu::Sampler,
    cell_light: CellLightUniform,
    cell_light_available: bool,
}

impl GlobalUniforms {
    pub fn new(device: &wgpu::Device) -> Self {
        use wgpu::util::DeviceExt;

        let camera_uniform = CameraUniform::from_camera(&Camera::default());
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("camera_uniform"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let light_uniform = LightUniform::default();
        let light_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("light_uniform"),
            contents: bytemuck::cast_slice(&[light_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let initial_lights = [PointLightGpu {
            position: [0.0; 4],
            color_range: [0.0; 4],
        }];
        let point_light_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("point_lights"),
            contents: bytemuck::cast_slice(&initial_lights),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        let point_light_capacity = initial_lights.len();

        let fog_uniform = FogUniform::default();
        let fog_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("fog_uniform"),
            contents: bytemuck::cast_slice(&[fog_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let cell_light = CellLightUniform::default();
        let cell_light_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("cell_light_uniform"),
            contents: bytemuck::cast_slice(&[cell_light]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let cell_light_view =
            create_cell_light_texture(device, 1, 1).create_view(&Default::default());
        let cell_light_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("cell_light_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("global_uniforms"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 6,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let bind_group = build_bind_group(
            device,
            &bind_group_layout,
            &camera_buffer,
            &light_buffer,
            &point_light_buffer,
            &fog_buffer,
            &cell_light_view,
            &cell_light_sampler,
            &cell_light_buffer,
        );

        Self {
            camera_buffer,
            light_buffer,
            point_light_buffer,
            fog_buffer,
            point_light_capacity,
            bind_group_layout,
            bind_group,
            cell_light_buffer,
            cell_light_view,
            cell_light_sampler,
            cell_light,
            cell_light_available: false,
        }
    }

    fn rebuild_bind_group(&mut self, device: &wgpu::Device) {
        self.bind_group = build_bind_group(
            device,
            &self.bind_group_layout,
            &self.camera_buffer,
            &self.light_buffer,
            &self.point_light_buffer,
            &self.fog_buffer,
            &self.cell_light_view,
            &self.cell_light_sampler,
            &self.cell_light_buffer,
        );
    }

    /// Uploads a map's per-cell light texture, or clears it when the map has no
    /// lightmap to sample.
    pub fn update_cell_light(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        map: Option<&crate::cell_light::CellLightMap>,
        cell_size: f32,
    ) {
        let (width, height, pixels) = match map {
            Some(map) => (map.width, map.height, map.pixels.as_slice()),
            None => (1, 1, &[0u8, 0, 0, 255][..]),
        };

        let texture = create_cell_light_texture(device, width, height);
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        self.cell_light_view = texture.create_view(&Default::default());
        self.cell_light_available = map.is_some();
        self.cell_light = CellLightUniform {
            cell_size,
            width: width as f32,
            height: height as f32,
            enabled: self.cell_light.enabled,
        };
        self.write_cell_light(queue);
        self.rebuild_bind_group(device);
    }

    pub fn set_cell_light_enabled(&mut self, queue: &wgpu::Queue, enabled: bool) {
        self.cell_light.enabled = if enabled { 1.0 } else { 0.0 };
        self.write_cell_light(queue);
    }

    fn write_cell_light(&self, queue: &wgpu::Queue) {
        let mut uniform = self.cell_light;
        if !self.cell_light_available {
            uniform.enabled = 0.0;
        }
        queue.write_buffer(&self.cell_light_buffer, 0, bytemuck::cast_slice(&[uniform]));
    }

    pub fn update_camera(&self, queue: &wgpu::Queue, camera: &Camera) {
        let uniform = CameraUniform::from_camera(camera);
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[uniform]));
    }

    pub fn update_light(&self, queue: &wgpu::Queue, light: &LightUniform) {
        queue.write_buffer(&self.light_buffer, 0, bytemuck::cast_slice(&[*light]));
    }

    pub fn update_fog(&self, queue: &wgpu::Queue, fog: &FogUniform) {
        queue.write_buffer(&self.fog_buffer, 0, bytemuck::cast_slice(&[*fog]));
    }

    pub fn update_point_lights(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        lights: &[PointLightGpu],
    ) {
        use wgpu::util::DeviceExt;

        let sentinel = [PointLightGpu {
            position: [0.0; 4],
            color_range: [0.0; 4],
        }];
        let payload: &[PointLightGpu] = if lights.is_empty() { &sentinel } else { lights };

        if payload.len() > self.point_light_capacity {
            self.point_light_buffer =
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("point_lights"),
                    contents: bytemuck::cast_slice(payload),
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                });
            self.point_light_capacity = payload.len();
            self.rebuild_bind_group(device);
        } else {
            queue.write_buffer(&self.point_light_buffer, 0, bytemuck::cast_slice(payload));
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn build_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    camera_buffer: &wgpu::Buffer,
    light_buffer: &wgpu::Buffer,
    point_light_buffer: &wgpu::Buffer,
    fog_buffer: &wgpu::Buffer,
    cell_light_view: &wgpu::TextureView,
    cell_light_sampler: &wgpu::Sampler,
    cell_light_buffer: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("global_uniforms"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: light_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: point_light_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: fog_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: wgpu::BindingResource::TextureView(cell_light_view),
            },
            wgpu::BindGroupEntry {
                binding: 5,
                resource: wgpu::BindingResource::Sampler(cell_light_sampler),
            },
            wgpu::BindGroupEntry {
                binding: 6,
                resource: cell_light_buffer.as_entire_binding(),
            },
        ],
    })
}

fn create_cell_light_texture(device: &wgpu::Device, width: u32, height: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("cell_light"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    })
}
