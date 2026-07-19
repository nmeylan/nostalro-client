use ragnarok_formats::act::{ActFile, Motion};
use ragnarok_formats::grf::GrfArchive;
use ragnarok_formats::spr::{RgbaImageData, SprFile};
use std::collections::HashMap;

pub const EMOTION_ICON_PREFIX: &str = "@emo/";
const EMOTION_SPR_PATH: &str = "data/sprite/이팩트/emotion.spr";
const EMOTION_ACT_PATH: &str = "data/sprite/이팩트/emotion.act";

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
        if name.starts_with(EMOTION_ICON_PREFIX) {
            if !self.textures.contains_key(name) {
                self.load_emotion_icons(grf, device, queue);
            }
            return self.textures.get(name);
        }
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

    fn load_emotion_icons(&mut self, grf: &GrfArchive, device: &wgpu::Device, queue: &wgpu::Queue) {
        let (Ok(spr_data), Ok(act_data)) =
            (grf.read_file(EMOTION_SPR_PATH), grf.read_file(EMOTION_ACT_PATH))
        else {
            return;
        };
        let (Ok(spr), Ok(act)) = (SprFile::parse(&spr_data), ActFile::parse(&act_data)) else {
            return;
        };
        let (images, indexed_count) = spr.to_rgba_images();
        for (action_idx, action) in act.actions.iter().enumerate() {
            if action.motions.is_empty() {
                continue;
            }
            let motion = &action.motions[action.motions.len() / 5];
            let Some((w, h, rgba)) = composite_emote_frame(motion, &images, indexed_count) else {
                continue;
            };
            let name = format!("{EMOTION_ICON_PREFIX}{action_idx}");
            let bind_group = create_texture_bind_group_from_rgba(
                device,
                queue,
                &rgba,
                w,
                h,
                &self.bind_group_layout,
                &name,
                wgpu::FilterMode::Nearest,
                wgpu::TextureFormat::Rgba8UnormSrgb,
                wgpu::AddressMode::ClampToEdge,
            );
            self.sizes.insert(name.clone(), (w, h));
            self.textures.insert(name, bind_group);
        }
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
    let format = if path.contains("/item/") {
        // Avoid texture color wash for items
        wgpu::TextureFormat::Rgba8UnormSrgb
    } else {
        wgpu::TextureFormat::Rgba8Unorm
    };
    let bg = create_texture_bind_group_from_rgba(
        device,
        queue,
        rgba.as_raw(),
        w,
        h,
        layout,
        path,
        wgpu::FilterMode::Nearest,
        format,
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

/// Flatten one emote animation frame (all its layers) into a single RGBA image,
/// source-over compositing each clip at its centred offset.
fn composite_emote_frame(
    motion: &Motion,
    images: &[RgbaImageData],
    indexed_count: usize,
) -> Option<(u32, u32, Vec<u8>)> {
    let mut placed: Vec<(&RgbaImageData, i32, i32)> = Vec::new();
    for clip in &motion.clips {
        if clip.sprite_index < 0 {
            continue;
        }
        let idx = if clip.sprite_type == 0 {
            clip.sprite_index as usize
        } else {
            indexed_count + clip.sprite_index as usize
        };
        let Some(img) = images.get(idx) else { continue };
        if img.width == 0 || img.height == 0 {
            continue;
        }
        let left = clip.x - img.width as i32 / 2;
        let top = clip.y - img.height as i32 / 2;
        placed.push((img, left, top));
    }
    if placed.is_empty() {
        return None;
    }

    let min_l = placed.iter().map(|(_, l, _)| *l).min().unwrap();
    let min_t = placed.iter().map(|(_, _, t)| *t).min().unwrap();
    let max_r = placed.iter().map(|(im, l, _)| l + im.width as i32).max().unwrap();
    let max_b = placed.iter().map(|(im, _, t)| t + im.height as i32).max().unwrap();
    let w = (max_r - min_l) as u32;
    let h = (max_b - min_t) as u32;
    if w == 0 || h == 0 {
        return None;
    }

    let mut buf = vec![0u8; (w * h * 4) as usize];
    for (img, left, top) in placed {
        let ox = (left - min_l) as u32;
        let oy = (top - min_t) as u32;
        for y in 0..img.height {
            for x in 0..img.width {
                let si = ((y * img.width + x) * 4) as usize;
                let sa = img.data[si + 3] as f32 / 255.0;
                if sa <= 0.0 {
                    continue;
                }
                let di = (((oy + y) * w + (ox + x)) * 4) as usize;
                for c in 0..3 {
                    let s = img.data[si + c] as f32;
                    let d = buf[di + c] as f32;
                    buf[di + c] = (s * sa + d * (1.0 - sa)) as u8;
                }
                let da = buf[di + 3] as f32 / 255.0;
                buf[di + 3] = ((sa + da * (1.0 - sa)) * 255.0) as u8;
            }
        }
    }
    Some((w, h, buf))
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
