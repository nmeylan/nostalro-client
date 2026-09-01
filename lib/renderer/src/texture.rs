use ragnarok_formats::grf::GrfArchive;
use std::collections::HashMap;

mod emotion;

pub use emotion::EMOTION_ICON_PREFIX;

pub struct TextureCache {
    textures: HashMap<String, wgpu::BindGroup>,
    sizes: HashMap<String, (u32, u32)>,
    pub bind_group_layout: wgpu::BindGroupLayout,
    dpi_scale: f32,
    filter_world: bool,
    world_textures: Vec<String>,
    /// Ground textures, sampled without wrapping. A cell maps the whole texture,
    /// so a repeating sampler blends the far edge in at every cell boundary.
    ground: HashMap<String, wgpu::BindGroup>,
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
            filter_world: true,
            world_textures: Vec::new(),
            ground: HashMap::new(),
        }
    }

    /// Filters ground and model textures instead of point-sampling them.
    /// Textures already uploaded are rebuilt when a `grf` is given.
    pub fn set_world_filtering(
        &mut self,
        on: bool,
        grf: Option<&GrfArchive>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        if self.filter_world == on {
            return;
        }
        self.filter_world = on;
        let Some(grf) = grf else {
            return;
        };
        for name in std::mem::take(&mut self.world_textures) {
            self.remove(&name);
            self.get_or_load(&name, grf, device, queue, false);
        }
        for name in self.ground.keys().cloned().collect::<Vec<_>>() {
            self.ground.remove(&name);
            self.get_or_load_ground(&name, grf, device, queue);
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
        if name.starts_with(EMOTION_ICON_PREFIX) {
            if !self.textures.contains_key(name) {
                for (icon_name, bind_group, w, h) in
                    emotion::load_emotion_icons(grf, device, queue, &self.bind_group_layout)
                {
                    self.textures.insert(icon_name.clone(), bind_group);
                    self.sizes.insert(icon_name, (w, h));
                }
            }
            return self.textures.get(name);
        }
        if !self.textures.contains_key(name) {
            let mut img = decode_texture(name, grf)?;

            let is_bmp = name.to_ascii_lowercase().ends_with(".bmp");
            if is_bmp {
                apply_magenta_transparency(&mut img);
            }

            let logical_w = img.width();
            let logical_h = img.height();

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
                } else if !self.filter_world {
                    create_texture_bind_group_filtered(
                        device,
                        queue,
                        &img,
                        &self.bind_group_layout,
                        name,
                        wgpu::FilterMode::Nearest,
                        wgpu::AddressMode::Repeat,
                    )
                } else {
                    create_world_texture_bind_group(
                        device,
                        queue,
                        &img,
                        &self.bind_group_layout,
                        name,
                        wgpu::AddressMode::Repeat,
                    )
                }
            } else {
                create_texture_bind_group(device, queue, &img, &self.bind_group_layout, name)
            };
            if !dpi_upscale {
                self.world_textures.push(name.to_string());
            }
            self.textures.insert(name.to_string(), bind_group);
            self.sizes.insert(name.to_string(), (logical_w, logical_h));
        }
        self.textures.get(name)
    }

    pub fn get_or_load_ground(
        &mut self,
        name: &str,
        grf: &GrfArchive,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Option<&wgpu::BindGroup> {
        if !self.ground.contains_key(name) {
            let mut img = decode_texture(name, grf)?;
            apply_magenta_transparency(&mut img);
            let bind_group = if self.filter_world {
                create_world_texture_bind_group(
                    device,
                    queue,
                    &img,
                    &self.bind_group_layout,
                    name,
                    wgpu::AddressMode::ClampToEdge,
                )
            } else {
                create_texture_bind_group_filtered(
                    device,
                    queue,
                    &img,
                    &self.bind_group_layout,
                    name,
                    wgpu::FilterMode::Nearest,
                    wgpu::AddressMode::ClampToEdge,
                )
            };
            self.sizes
                .insert(name.to_string(), (img.width(), img.height()));
            self.ground.insert(name.to_string(), bind_group);
        }
        self.ground.get(name)
    }

    pub fn get_ground(&self, name: &str) -> Option<&wgpu::BindGroup> {
        self.ground.get(name)
    }

    pub fn get(&self, name: &str) -> Option<&wgpu::BindGroup> {
        self.textures.get(name)
    }

    pub fn texture_size(&self, name: &str) -> Option<(u32, u32)> {
        self.sizes.get(name).copied()
    }

    pub fn remove(&mut self, name: &str) {
        self.textures.remove(name);
        self.sizes.remove(name);
    }

    pub fn insert(&mut self, name: &str, bind_group: wgpu::BindGroup, width: u32, height: u32) {
        self.textures.insert(name.to_string(), bind_group);
        self.sizes.insert(name.to_string(), (width, height));
    }
}

pub fn decode_emblem(blob: &[u8]) -> Option<image::RgbaImage> {
    let bytes = ragnarok_formats::zlib_decompress(blob).unwrap_or_else(|| blob.to_vec());
    let img = image::load_from_memory_with_format(&bytes, image::ImageFormat::Bmp).ok()?;
    let mut rgba = img.to_rgba8();
    ragnarok_formats::apply_magenta_transparency(rgba.as_mut());
    Some(rgba)
}

#[cfg(test)]
mod emblem_tests {
    use super::*;

    fn bmp_24x24() -> Vec<u8> {
        let mut img = image::RgbImage::new(24, 24);
        img.put_pixel(0, 0, image::Rgb([255, 0, 255]));
        img.put_pixel(1, 1, image::Rgb([10, 20, 30]));
        let mut out = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(img)
            .write_to(&mut out, image::ImageFormat::Bmp)
            .unwrap();
        out.into_inner()
    }

    #[test]
    fn decode_emblem_zlib_bmp_makes_magenta_transparent() {
        let blob = ragnarok_formats::zlib_compress(&bmp_24x24());
        let rgba = decode_emblem(&blob).expect("decode");
        assert_eq!(rgba.dimensions(), (24, 24));
        assert_eq!(rgba.get_pixel(0, 0).0, [0, 0, 0, 0]);
        assert_eq!(rgba.get_pixel(1, 1).0, [10, 20, 30, 255]);
    }
}

pub fn load_keyed_texture(
    path: &str,
    grf: &GrfArchive,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    address_mode: wgpu::AddressMode,
    filter: wgpu::FilterMode,
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
    let bg = create_texture_bind_group_from_rgba(
        device,
        queue,
        rgba.as_raw(),
        w,
        h,
        layout,
        path,
        filter,
        wgpu::TextureFormat::Rgba8UnormSrgb,
        address_mode,
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

fn decode_texture(name: &str, grf: &GrfArchive) -> Option<image::RgbaImage> {
    let data = match grf.read_file(name) {
        Ok(d) => d,
        Err(e) => {
            tracing::warn!("Failed to load texture {name}: {e}");
            return None;
        }
    };
    match image::load_from_memory(&data) {
        Ok(i) => Some(i.to_rgba8()),
        Err(_) => match format_from_extension(name) {
            Some(fmt) => match image::load_from_memory_with_format(&data, fmt) {
                Ok(i) => Some(i.to_rgba8()),
                Err(e) => {
                    tracing::warn!("Failed to decode texture {name}: {e}");
                    None
                }
            },
            None => {
                tracing::warn!("Failed to decode texture {name}: unknown format");
                None
            }
        },
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

/// The `0.81` cut-off the ground and model shaders discard below.
const ALPHA_DISCARD: f32 = 0.81 * 255.0;

fn scaled_alpha(alpha: u8, scale: f32) -> u8 {
    (alpha as f32 * scale).min(255.0).round() as u8
}

/// Fraction of texels that would survive the discard test with alpha scaled.
fn alpha_coverage(img: &image::RgbaImage, scale: f32) -> f32 {
    let passing = img
        .pixels()
        .filter(|p| scaled_alpha(p.0[3], scale) as f32 >= ALPHA_DISCARD)
        .count();
    passing as f32 / (img.width() * img.height()).max(1) as f32
}

/// Alpha scale that brings this level's coverage back to the full-size one.
/// Box filtering alone shrinks a cut-out at every level until thin features -
/// a cobweb strand, a leaf edge - fail the discard test and the face vanishes
/// once a coarse level is sampled; decimating instead just breaks them into
/// dots. Holding coverage keeps them whole, blurrier but there.
///
/// Note that in original game cobweb in orcsdun01 are barely visible, below code is a divergence from OG
fn scale_for_coverage(img: &image::RgbaImage, target: f32) -> f32 {
    let (mut lo, mut hi) = (1.0f32, 64.0f32);
    if alpha_coverage(img, hi) < target {
        return hi;
    }
    for _ in 0..16 {
        let mid = 0.5 * (lo + hi);
        if alpha_coverage(img, mid) < target {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    hi
}

fn halve(img: &image::RgbaImage) -> image::RgbaImage {
    let (w, h) = img.dimensions();
    image::imageops::resize(
        img,
        (w / 2).max(1),
        (h / 2).max(1),
        image::imageops::FilterType::Triangle,
    )
}

fn rescale_alpha(img: &image::RgbaImage, scale: f32) -> image::RgbaImage {
    let mut out = img.clone();
    for px in out.pixels_mut() {
        px.0[3] = scaled_alpha(px.0[3], scale);
    }
    out
}

pub fn create_world_texture_bind_group(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    img: &image::RgbaImage,
    layout: &wgpu::BindGroupLayout,
    label: &str,
    address_mode: wgpu::AddressMode,
) -> wgpu::BindGroup {
    let (width, height) = img.dimensions();
    let mip_level_count = width.max(height).max(1).ilog2() + 1;

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    let target_coverage = alpha_coverage(img, 1.0);
    let mut raw = std::borrow::Cow::Borrowed(img);
    for mip in 0..mip_level_count {
        if mip > 0 {
            raw = std::borrow::Cow::Owned(halve(raw.as_ref()));
        }
        let level = if mip == 0 || target_coverage <= 0.0 {
            std::borrow::Cow::Borrowed(raw.as_ref())
        } else {
            std::borrow::Cow::Owned(rescale_alpha(
                raw.as_ref(),
                scale_for_coverage(raw.as_ref(), target_coverage),
            ))
        };
        let (w, h) = level.dimensions();
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: mip,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            level.as_raw(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * w),
                rows_per_image: Some(h),
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
    }

    let view = texture.create_view(&Default::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: address_mode,
        address_mode_v: address_mode,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::MipmapFilterMode::Linear,
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

pub fn create_texture_bind_group_clamped(
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
        wgpu::AddressMode::ClampToEdge,
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

