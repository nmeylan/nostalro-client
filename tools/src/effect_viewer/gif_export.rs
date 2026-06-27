//! Offscreen GIF capture for the effect viewer. Renders into a fixed-size
//! offscreen color target via `Renderer::render_into`, copies the texture to
//! a mappable buffer, and encodes one frame to disk per call.
//!
//! Output format matches `../ro-effects/effects/imgs/<bucket>/<id>.gif`:
//! 256x256, black background, 30 fps (delay=3 in GIF 1/100s units). Sim
//! ticks at 60 Hz; we capture every other tick.

use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;
use std::sync::mpsc;

use models::enums::effect_id::EffectId;

pub const GIF_W: u32 = 256;
pub const GIF_H: u32 = 256;
pub const GIF_FPS: u32 = 30;
pub const GIF_DELAY_HUNDREDTHS: u16 = 3;
pub const SIM_TICK_HZ: f32 = 60.0;
const BYTES_PER_PIXEL: u32 = 4;
const ROW_PITCH: u32 = GIF_W * BYTES_PER_PIXEL;
const BUFFER_SIZE: u64 = (ROW_PITCH * GIF_H) as u64;
const DEFAULT_DURATION_MS: u32 = 3000;
/// Several persistent effects in `effect_spec` carry a sentinel
/// `duration_ms = 99990` (~100s) meaning "loops indefinitely". For GIF
/// purposes we cap at this many ms so the encode terminates in a sane
/// amount of time and the output size stays comparable to the reference
/// gifs (which are typically <2s).
const MAX_DURATION_MS: u32 = 5000;

pub struct CaptureTarget {
    pub color: wgpu::Texture,
    pub color_view: wgpu::TextureView,
    pub depth: wgpu::Texture,
    pub depth_view: wgpu::TextureView,
    pub readback: wgpu::Buffer,
    pub format: wgpu::TextureFormat,
}

impl CaptureTarget {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let color = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("gif_capture_color"),
            size: wgpu::Extent3d {
                width: GIF_W,
                height: GIF_H,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: surface_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let color_view = color.create_view(&Default::default());
        let depth = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("gif_capture_depth"),
            size: wgpu::Extent3d {
                width: GIF_W,
                height: GIF_H,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let depth_view = depth.create_view(&Default::default());
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gif_capture_readback"),
            size: BUFFER_SIZE,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        Self {
            color,
            color_view,
            depth,
            depth_view,
            readback,
            format: surface_format,
        }
    }
}

pub struct GifSession {
    pub effect_id: EffectId,
    pub frames_total: u32,
    pub frames_captured: u32,
    pub sim_dt: f32,
    pub capture_every: u32,
    pub tick_counter: u32,
    pub target: CaptureTarget,
    encoder: gif::Encoder<BufWriter<File>>,
    out_path: PathBuf,
}

impl GifSession {
    pub fn begin(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        effect_id: EffectId,
        duration_ms: Option<u32>,
        out_path: PathBuf,
    ) -> std::io::Result<Self> {
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = File::create(&out_path)?;
        let writer = BufWriter::new(file);
        let mut encoder = gif::Encoder::new(writer, GIF_W as u16, GIF_H as u16, &[])
            .map_err(std::io::Error::other)?;
        encoder
            .set_repeat(gif::Repeat::Infinite)
            .map_err(std::io::Error::other)?;
        let duration_ms = duration_ms
            .unwrap_or(DEFAULT_DURATION_MS)
            .min(MAX_DURATION_MS);
        let frames_total = ((duration_ms as f32 / 1000.0) * GIF_FPS as f32)
            .round()
            .max(1.0) as u32;
        let capture_every = (SIM_TICK_HZ as u32 / GIF_FPS).max(1);
        Ok(Self {
            effect_id,
            frames_total,
            frames_captured: 0,
            sim_dt: 1.0 / SIM_TICK_HZ,
            capture_every,
            tick_counter: 0,
            target: CaptureTarget::new(device, surface_format),
            encoder,
            out_path,
        })
    }

    pub fn is_complete(&self) -> bool {
        self.frames_captured >= self.frames_total
    }

    /// Returns true if the current sim tick should be encoded after rendering
    /// to the capture target.
    pub fn tick_should_capture(&mut self) -> bool {
        let capture = self.tick_counter % self.capture_every == 0;
        self.tick_counter += 1;
        capture
    }

    /// Reads back the most recent render-into result and writes one GIF
    /// frame. Caller must have already invoked
    /// `Renderer::render_into(&target.color_view, &target.depth_view, ...)`
    /// in this submit cycle.
    pub fn write_current_frame(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.target.color,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &self.target.readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(ROW_PITCH),
                    rows_per_image: Some(GIF_H),
                },
            },
            wgpu::Extent3d {
                width: GIF_W,
                height: GIF_H,
                depth_or_array_layers: 1,
            },
        );
        queue.submit(std::iter::once(encoder.finish()));

        let slice = self.target.readback.slice(..);
        let (tx, rx) = mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |res| {
            let _ = tx.send(res);
        });
        let _ = device.poll(wgpu::PollType::wait_indefinitely());
        rx.recv().expect("map_async channel").expect("buffer map");

        let bgra = matches!(
            self.target.format,
            wgpu::TextureFormat::Bgra8UnormSrgb | wgpu::TextureFormat::Bgra8Unorm
        );
        let mut rgba = vec![0u8; (GIF_W * GIF_H * 4) as usize];
        {
            let data = slice.get_mapped_range();
            let mut dst = 0;
            let len = data.len();
            let mut src = 0;
            while src + 4 <= len {
                let (r, g, b) = if bgra {
                    (data[src + 2], data[src + 1], data[src])
                } else {
                    (data[src], data[src + 1], data[src + 2])
                };
                rgba[dst] = r;
                rgba[dst + 1] = g;
                rgba[dst + 2] = b;
                rgba[dst + 3] = 255;
                src += 4;
                dst += 4;
            }
        }
        self.target.readback.unmap();

        let mut frame = gif::Frame::from_rgba_speed(GIF_W as u16, GIF_H as u16, &mut rgba, 10);
        frame.delay = GIF_DELAY_HUNDREDTHS;
        if let Err(e) = self.encoder.write_frame(&frame) {
            tracing::warn!("gif write_frame failed: {e}");
        }
        self.frames_captured += 1;
    }

    pub fn out_path(&self) -> &PathBuf {
        &self.out_path
    }
}
