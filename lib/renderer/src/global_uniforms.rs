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
            shadow_strength: 0.5,
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

pub struct GlobalUniforms {
    pub camera_buffer: wgpu::Buffer,
    pub light_buffer: wgpu::Buffer,
    pub point_light_buffer: wgpu::Buffer,
    pub fog_buffer: wgpu::Buffer,
    pub point_light_capacity: usize,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
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

        // Storage buffers must be non-empty; seed with one zero-range light that
        // the shader skips via `if (lr <= 0.0) continue;`.
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
            ],
        });

        let bind_group = Self::create_bind_group(
            device,
            &bind_group_layout,
            &camera_buffer,
            &light_buffer,
            &point_light_buffer,
            &fog_buffer,
        );

        Self {
            camera_buffer,
            light_buffer,
            point_light_buffer,
            fog_buffer,
            point_light_capacity,
            bind_group_layout,
            bind_group,
        }
    }

    fn create_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        camera_buffer: &wgpu::Buffer,
        light_buffer: &wgpu::Buffer,
        point_light_buffer: &wgpu::Buffer,
        fog_buffer: &wgpu::Buffer,
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
            ],
        })
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

    /// Replace all point lights. Lights are static for the life of the loaded
    /// map; reupload on map change. The buffer is grown when needed; the bind
    /// group is rebuilt on grow because the underlying buffer is new.
    pub fn update_point_lights(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        lights: &[PointLightGpu],
    ) {
        use wgpu::util::DeviceExt;

        // Always upload at least one entry — wgpu disallows zero-size storage
        // buffers, and the shader skips zero-range entries.
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
            self.bind_group = Self::create_bind_group(
                device,
                &self.bind_group_layout,
                &self.camera_buffer,
                &self.light_buffer,
                &self.point_light_buffer,
                &self.fog_buffer,
            );
        } else {
            queue.write_buffer(&self.point_light_buffer, 0, bytemuck::cast_slice(payload));
        }
    }
}
