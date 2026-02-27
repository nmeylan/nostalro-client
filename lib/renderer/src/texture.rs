use std::collections::HashMap;
use ragnarok_formats::grf::GrfArchive;

pub struct TextureCache {
    textures: HashMap<String, wgpu::BindGroup>,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl TextureCache {
    pub fn new(device: &wgpu::Device) -> Self {
        let bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("texture"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        Self {
            textures: HashMap::new(),
            bind_group_layout,
        }
    }

    pub fn get_or_load(
        &mut self,
        name: &str,
        grf: &GrfArchive,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Option<&wgpu::BindGroup> {
        if !self.textures.contains_key(name) {
            let data = match grf.read_file(name) {
                Ok(d) => d,
                Err(e) => {
                    tracing::warn!("Failed to load texture {name}: {e}");
                    return None;
                }
            };
            let img = match image::load_from_memory(&data) {
                Ok(i) => i.to_rgba8(),
                Err(e) => {
                    tracing::warn!("Failed to decode texture {name}: {e}");
                    return None;
                }
            };

            let bind_group =
                create_texture_bind_group(device, queue, &img, &self.bind_group_layout, name);
            self.textures.insert(name.to_string(), bind_group);
        }
        self.textures.get(name)
    }

    pub fn get(&self, name: &str) -> Option<&wgpu::BindGroup> {
        self.textures.get(name)
    }
}

pub fn create_texture_bind_group(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    img: &image::RgbaImage,
    layout: &wgpu::BindGroupLayout,
    label: &str,
) -> wgpu::BindGroup {
    let size = wgpu::Extent3d {
        width: img.width(),
        height: img.height(),
        depth_or_array_layers: 1,
    };

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        img.as_raw(),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4 * img.width()),
            rows_per_image: Some(img.height()),
        },
        size,
    );

    let view = texture.create_view(&Default::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::MipmapFilterMode::Nearest,
        ..Default::default()
    });

    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(label),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    })
}
