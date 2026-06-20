use ragnarok_formats::grf::GrfArchive;
use std::collections::HashMap;

pub struct TextureCache {
    textures: HashMap<String, wgpu::BindGroup>,
    sizes: HashMap<String, (u32, u32)>,
    pub bind_group_layout: wgpu::BindGroupLayout,
    dpi_scale: f32,
}

impl TextureCache {
    pub fn new(device: &wgpu::Device, dpi_scale: f32) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
            dpi_scale,
        }
    }

    pub fn get_or_load(
        &mut self,
        name: &str,
        grf: &GrfArchive,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        dpi_upscale: bool,
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

            // Extension casing varies in GRFs (e.g. yuno ground/model textures
            // use `.BMP`); match case-insensitively so the color key applies.
            let is_bmp = name.to_ascii_lowercase().ends_with(".bmp");

            // RO BMP convention: magenta (FF00FF) pixels become transparent
            if is_bmp {
                apply_magenta_transparency(&mut img);
            }

            let logical_w = img.width();
            let logical_h = img.height();

            // CPU-upscale UI textures by dpi_scale for crisp rendering on high DPI
            let bind_group = if is_bmp {
                if dpi_upscale && self.dpi_scale > 1.0 {
                    let phys_w = (logical_w as f32 * self.dpi_scale) as u32;
                    let phys_h = (logical_h as f32 * self.dpi_scale) as u32;
                    let upscaled = image::imageops::resize(
                        &img,
                        phys_w,
                        phys_h,
                        image::imageops::FilterType::CatmullRom,
                    );
                    // Linear sampling avoids triangle-diagonal UV precision artifacts
                    // that cause shearing with Nearest at fractional DPI scales
                    create_texture_bind_group_filtered(
                        device,
                        queue,
                        &upscaled,
                        &self.bind_group_layout,
                        name,
                        wgpu::FilterMode::Linear,
                        wgpu::AddressMode::ClampToEdge,
                    )
                } else if dpi_upscale {
                    create_texture_bind_group_filtered(
                        device,
                        queue,
                        &img,
                        &self.bind_group_layout,
                        name,
                        wgpu::FilterMode::Nearest,
                        wgpu::AddressMode::ClampToEdge,
                    )
                } else {
                    create_texture_bind_group_nearest(
                        device,
                        queue,
                        &img,
                        &self.bind_group_layout,
                        name,
                    )
                }
            } else {
                create_texture_bind_group(device, queue, &img, &self.bind_group_layout, name)
            };
            self.textures.insert(name.to_string(), bind_group);
            self.sizes.insert(name.to_string(), (logical_w, logical_h));
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

/// Load `path` from `grf` and build a bind group, applying the two
/// transparency conventions used by STR-style effect textures:
///   * magenta (FF00FF) → transparent (RO BMP color key)
///   * pure black → transparent (so additive layers don't add brightness)
///
/// If the exact `path` is missing, retries with `.bmp`/`.tga` swap — RO GRFs
/// mix both conventions for the same logical asset name. Returns `None` if no
/// variant resolves or decoding fails.
pub fn load_keyed_texture(
    path: &str,
    grf: &GrfArchive,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
) -> Option<(wgpu::BindGroup, u32, u32)> {
    let (data, resolved_path) = read_with_ext_fallback(path, grf)?;

    let img = match image::load_from_memory(&data) {
        Ok(i) => i,
        Err(_) => {
            let fmt = format_from_extension(&resolved_path)?;
            image::load_from_memory_with_format(&data, fmt).ok()?
        }
    };

    let mut rgba = img.to_rgba8();
    ragnarok_formats::apply_magenta_transparency(rgba.as_mut());
    for px in rgba.pixels_mut() {
        if px[0] == 0 && px[1] == 0 && px[2] == 0 {
            px[3] = 0;
        }
    }

    let w = rgba.width();
    let h = rgba.height();
    // Sample the native-resolution texture with `Nearest` filtering — flame
    // textures (ring_*.tga, magic_*.tga) are small pixel-art with sharp
    // feathered tips. `Linear` interpolates the small native pattern as the
    // cone geometry stretches it over a large surface, smearing the distinct
    // flame tongues into a blurry smooth fan; `Nearest` keeps them crisp.
    // The faint blocky tongue edges `Nearest` would otherwise leave are
    // softened by the alpha gamma curve in `effect_frustum.wgsl`.
    //
    // Effect textures are already perceptual sRGB and must NOT be re-decoded:
    // a second sRGB→linear decode on read darkens midtones and shifts tints
    // when combined with the additive accumulation done by effect primitives.
    let bg = create_texture_bind_group_from_rgba(
        device,
        queue,
        rgba.as_raw(),
        w,
        h,
        layout,
        path,
        wgpu::FilterMode::Nearest,
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::AddressMode::Repeat,
    );
    Some((bg, w, h))
}

fn read_with_ext_fallback(path: &str, grf: &GrfArchive) -> Option<(Vec<u8>, String)> {
    if let Ok(d) = grf.read_file(path) {
        return Some((d, path.to_string()));
    }
    let alt = if let Some(stem) = path.strip_suffix(".tga") {
        format!("{stem}.bmp")
    } else if let Some(stem) = path.strip_suffix(".bmp") {
        format!("{stem}.tga")
    } else {
        return None;
    };
    grf.read_file(&alt).ok().map(|d| (d, alt))
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
    create_texture_bind_group_filtered(
        device,
        queue,
        img,
        layout,
        label,
        wgpu::FilterMode::Linear,
        wgpu::AddressMode::Repeat,
    )
}

pub fn create_texture_bind_group_nearest(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    img: &image::RgbaImage,
    layout: &wgpu::BindGroupLayout,
    label: &str,
) -> wgpu::BindGroup {
    create_texture_bind_group_filtered(
        device,
        queue,
        img,
        layout,
        label,
        wgpu::FilterMode::Nearest,
        wgpu::AddressMode::Repeat,
    )
}

pub fn create_font_atlas_bind_group(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    img: &image::RgbaImage,
    layout: &wgpu::BindGroupLayout,
    label: &str,
) -> wgpu::BindGroup {
    create_texture_bind_group_from_rgba(
        device,
        queue,
        img.as_raw(),
        img.width(),
        img.height(),
        layout,
        label,
        wgpu::FilterMode::Linear,
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::AddressMode::ClampToEdge,
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
    address_mode: wgpu::AddressMode,
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
        address_mode_u: address_mode,
        address_mode_v: address_mode,
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
    address_mode: wgpu::AddressMode,
) -> wgpu::BindGroup {
    create_texture_bind_group_from_rgba(
        device,
        queue,
        img.as_raw(),
        img.width(),
        img.height(),
        layout,
        label,
        filter,
        wgpu::TextureFormat::Rgba8UnormSrgb,
        address_mode,
    )
}
