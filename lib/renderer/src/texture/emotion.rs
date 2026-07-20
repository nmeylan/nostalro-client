use super::create_texture_bind_group_from_rgba;
use ragnarok_formats::act::{ActFile, Motion};
use ragnarok_formats::grf::GrfArchive;
use ragnarok_formats::spr::{RgbaImageData, SprFile};

pub const EMOTION_ICON_PREFIX: &str = "@emo/";
const EMOTION_SPR_PATH: &str = "data/sprite/이팩트/emotion.spr";
const EMOTION_ACT_PATH: &str = "data/sprite/이팩트/emotion.act";

pub(super) fn load_emotion_icons(
    grf: &GrfArchive,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
) -> Vec<(String, wgpu::BindGroup, u32, u32)> {
    let (Ok(spr_data), Ok(act_data)) =
        (grf.read_file(EMOTION_SPR_PATH), grf.read_file(EMOTION_ACT_PATH))
    else {
        return Vec::new();
    };
    let (Ok(spr), Ok(act)) = (SprFile::parse(&spr_data), ActFile::parse(&act_data)) else {
        return Vec::new();
    };
    let (images, indexed_count) = spr.to_rgba_images();
    let mut icons = Vec::new();
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
            layout,
            &name,
            wgpu::FilterMode::Nearest,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            wgpu::AddressMode::ClampToEdge,
        );
        icons.push((name, bind_group, w, h));
    }
    icons
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
