use std::collections::HashMap;
use ragnarok_formats::grf::GrfArchive;

pub struct TextureCache {
    textures: HashMap<String, wgpu::BindGroup>,
    sizes: HashMap<String, (u32, u32)>,
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
            sizes: HashMap::new(),
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
            let mut img = match image::load_from_memory(&data) {
                Ok(i) => i.to_rgba8(),
                Err(_) => match format_from_extension(name) {
                    Some(fmt) => match image::load_from_memory_with_format(&data, fmt) {
                        Ok(i) => i.to_rgba8(),
                        Err(e) => {
                            tracing::warn!("Failed to decode texture {name}: {e}");
                            return None;
                        }
                    },
                    None => {
                        tracing::warn!("Failed to decode texture {name}: unknown format");
                        return None;
                    }
                },
            };

            // RO BMP convention: magenta (FF00FF) pixels become transparent
            if name.ends_with(".bmp") {
                apply_magenta_transparency(&mut img);
            }

            let w = img.width();
            let h = img.height();
            let bind_group = if name.ends_with(".bmp") {
                create_texture_bind_group_nearest(device, queue, &img, &self.bind_group_layout, name)
            } else {
                create_texture_bind_group(device, queue, &img, &self.bind_group_layout, name)
            };
            self.textures.insert(name.to_string(), bind_group);
            self.sizes.insert(name.to_string(), (w, h));
        }
        self.textures.get(name)
    }

    pub fn get(&self, name: &str) -> Option<&wgpu::BindGroup> {
        self.textures.get(name)
    }

    pub fn texture_size(&self, name: &str) -> Option<(u32, u32)> {
        self.sizes.get(name).copied()
    }

    pub fn insert(&mut self, name: &str, bind_group: wgpu::BindGroup, width: u32, height: u32) {
        self.textures.insert(name.to_string(), bind_group);
        self.sizes.insert(name.to_string(), (width, height));
    }
}

fn format_from_extension(name: &str) -> Option<image::ImageFormat> {
    let ext = name.rsplit('.').next()?.to_ascii_lowercase();
    match ext.as_str() {
        "tga" => Some(image::ImageFormat::Tga),
        "bmp" => Some(image::ImageFormat::Bmp),
        "png" => Some(image::ImageFormat::Png),
        "jpg" | "jpeg" => Some(image::ImageFormat::Jpeg),
        _ => None,
    }
}

fn apply_magenta_transparency(img: &mut image::RgbaImage) {
    ragnarok_formats::apply_magenta_transparency(img.as_mut());
}

pub fn create_texture_bind_group(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    img: &image::RgbaImage,
    layout: &wgpu::BindGroupLayout,
    label: &str,
) -> wgpu::BindGroup {
    create_texture_bind_group_filtered(device, queue, img, layout, label, wgpu::FilterMode::Linear)
}

pub fn create_texture_bind_group_nearest(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    img: &image::RgbaImage,
    layout: &wgpu::BindGroupLayout,
    label: &str,
) -> wgpu::BindGroup {
    create_texture_bind_group_filtered(device, queue, img, layout, label, wgpu::FilterMode::Nearest)
}

pub fn create_font_atlas_bind_group(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    img: &image::RgbaImage,
    layout: &wgpu::BindGroupLayout,
    label: &str,
) -> wgpu::BindGroup {
    create_texture_bind_group_from_rgba(
        device, queue, img.as_raw(), img.width(), img.height(),
        layout, label, wgpu::FilterMode::Linear, wgpu::TextureFormat::Rgba8Unorm,
    )
}

pub fn create_texture_bind_group_from_rgba(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    rgba_data: &[u8],
    width: u32,
    height: u32,
    layout: &wgpu::BindGroupLayout,
    label: &str,
    filter: wgpu::FilterMode,
    format: wgpu::TextureFormat,
) -> wgpu::BindGroup {
    let size = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
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
        rgba_data,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4 * width),
            rows_per_image: Some(height),
        },
        size,
    );

    let view = texture.create_view(&Default::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        mag_filter: filter,
        min_filter: filter,
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

fn create_texture_bind_group_filtered(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    img: &image::RgbaImage,
    layout: &wgpu::BindGroupLayout,
    label: &str,
    filter: wgpu::FilterMode,
) -> wgpu::BindGroup {
    create_texture_bind_group_from_rgba(device, queue, img.as_raw(), img.width(), img.height(), layout, label, filter, wgpu::TextureFormat::Rgba8UnormSrgb)
}
