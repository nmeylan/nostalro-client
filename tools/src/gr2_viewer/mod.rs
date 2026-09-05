//! GR2 model viewer: loads a Granny model from the GRF, renders it with
//! `Gr2ModelRenderer`, and plays its action animations (0=stand from the model
//! file, 1..4 from `data/model/3dmob_bone/{N}_{action}.gr2`). Also supports a
//! headless `--screenshot` mode that renders one frame to a PNG.

use std::path::Path;
use std::sync::Arc;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use ragnarok_formats::gr2::{Gr2Container, Gr2File};
use ragnarok_formats::grf::GrfArchive;
use ragnarok_game::gr2_model::{AnimationClip, Gr2Action, SkeletonPose, animation_file_path};
use ragnarok_renderer::gr2_model::{Gr2ModelAsset, Gr2ModelDraw, Gr2ModelPipeline};
use ragnarok_renderer::{
    Camera, GlobalUniforms, LightUniform, RenderDevice, TextureCache, block_on,
};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowAttributes, WindowId};

pub struct Args {
    pub grf_path: String,
    pub model: String,
    pub action: usize,
    pub screenshot: Option<String>,
    pub time: f32,
    pub width: u32,
    pub height: u32,
    /// Initial camera yaw in degrees (0 = front-on).
    pub yaw: f32,
    /// Image file (bmp/png/…) swapped onto the model's emblem texture slot.
    pub emblem: Option<String>,
}

const ACTION_NAMES: [&str; 5] = ["stand", "move", "attack", "dead", "damage"];
const ACTIONS: [Gr2Action; 5] = Gr2Action::ALL;
const CLEAR_COLOR: wgpu::Color = wgpu::Color {
    r: 0.15,
    g: 0.16,
    b: 0.20,
    a: 1.0,
};

fn parse_gr2(bytes: &[u8]) -> Option<Gr2File> {
    let container = Gr2Container::parse(bytes)
        .map_err(|e| eprintln!("gr2 container parse failed: {e:?}"))
        .ok()?;
    Gr2File::parse(&container)
        .map_err(|e| eprintln!("gr2 extract failed: {e:?}"))
        .ok()
}

/// GRF path for a model argument: full paths pass through, bare names go under
/// `data/model/3dmob/`.
fn resolve_model_path(model: &str) -> String {
    let lower = model.to_ascii_lowercase();
    if lower.contains('/') || lower.contains('\\') {
        lower
    } else {
        ragnarok_resources::model::mob(&lower)
    }
}

struct Gr2Assets {
    file: Gr2File,
    pose: SkeletonPose,
    clips: [Option<AnimationClip>; 5],
}

fn load_assets(grf: &GrfArchive, model_path: &str) -> Option<Gr2Assets> {
    let bytes = grf
        .read_file(model_path)
        .map_err(|e| eprintln!("cannot read {model_path}: {e}"))
        .ok()?;
    let file = parse_gr2(&bytes)?;
    let pose = SkeletonPose::from_model(&file, 0)?;

    let bone_type = ragnarok_game::gr2_model::bone_type_from_name(model_path);
    let clips = std::array::from_fn(|i| match ACTIONS[i] {
        Gr2Action::Stand => AnimationClip::from_gr2(&file, 0),
        action => {
            let path = animation_file_path(bone_type?, action)?;
            let bytes = grf.read_file(&path).ok()?;
            let anim_file = parse_gr2(&bytes)?;
            AnimationClip::from_gr2(&anim_file, 0)
        }
    });

    Some(Gr2Assets { file, pose, clips })
}

struct Scene {
    pipeline: Gr2ModelPipeline,
    renderer: Gr2ModelDraw,
    assets: Gr2Assets,
    camera: Camera,
    global_uniforms: GlobalUniforms,
    action: usize,
    time: f32,
    default_yaw: f32,
}

/// Swap `image_path` onto the model's emblem texture slot (the embedded
/// texture named `emblem.*`). BMPs get the usual magenta-key transparency.
fn apply_emblem_override(
    scene: &mut Scene,
    image_path: &str,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture_cache: &TextureCache,
) {
    let bytes = match std::fs::read(image_path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("cannot read emblem image {image_path}: {e}");
            return;
        }
    };
    let img = match image::load_from_memory(&bytes) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("cannot decode emblem image {image_path}: {e}");
            return;
        }
    };
    let mut rgba = img.to_rgba8();
    if image_path.to_ascii_lowercase().ends_with(".bmp") {
        ragnarok_formats::apply_magenta_transparency(rgba.as_mut());
    }
    let bind_group = ragnarok_renderer::gr2_model::create_emblem_bind_group(
        device,
        queue,
        rgba.as_raw(),
        rgba.width(),
        rgba.height(),
        texture_cache,
        "gr2_viewer_emblem",
    );
    if scene.renderer.set_emblem_texture(bind_group) {
        eprintln!("emblem override: {image_path}");
    } else {
        eprintln!("model has no emblem texture slot");
    }
}

impl Scene {
    fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_cache: &TextureCache,
        surface_format: wgpu::TextureFormat,
        grf: &GrfArchive,
        args: &Args,
        aspect: f32,
    ) -> Option<Self> {
        let assets = load_assets(grf, &resolve_model_path(&args.model))?;
        let mut global_uniforms = GlobalUniforms::new(device);
        let pipeline =
            Gr2ModelPipeline::new(device, surface_format, &global_uniforms, texture_cache);
        let asset = Gr2ModelAsset::from_gr2(&assets.file, 0, device, queue, texture_cache)?;
        let renderer = Gr2ModelDraw::new(device, &pipeline, std::rc::Rc::new(asset));

        let mut light = LightUniform::default();
        // Down-from-behind-the-camera in RO coords (negative Y is up); the
        // shader lights faces whose normal points toward the light.
        light.light_dir = [0.3, -0.8, -0.5, 0.0];
        light.ambient_color = [0.55, 0.55, 0.55, 1.0];
        global_uniforms.update_light(queue, &light);
        global_uniforms.update_point_lights(device, queue, &[]);

        let mut scene = Scene {
            pipeline,
            renderer,
            assets,
            camera: Camera::with_aspect(aspect),
            global_uniforms,
            action: args.action,
            time: 0.0,
            default_yaw: args.yaw.to_radians(),
        };
        scene.frame_camera();
        // These models are Z-up; a +90° X-rotation maps model +Z to world -Y
        // (up in RO coordinates) without flipping handedness.
        scene.renderer.set_transform(
            queue,
            glam::Mat4::from_rotation_x(std::f32::consts::FRAC_PI_2),
        );
        if let Some(path) = &args.emblem {
            apply_emblem_override(&mut scene, path, device, queue, texture_cache);
        }
        Some(scene)
    }

    fn frame_camera(&mut self) {
        let c = self.renderer.asset().center;
        let s = self.renderer.asset().size;
        // Match the model's Z-up → world -Y-up instance rotation.
        self.camera.target = glam::Vec3::new(c[0], -c[2], c[1]);
        let radius = 0.5 * (s[0] * s[0] + s[1] * s[1] + s[2] * s[2]).sqrt().max(1.0);
        self.camera.distance = radius / (self.camera.fov_y * 0.5).tan() * 1.3;
        self.camera.yaw = self.default_yaw;
        self.camera.pitch = 0.35;
    }

    fn set_action(&mut self, action: usize) {
        if action < ACTIONS.len() && self.assets.clips[action].is_some() {
            self.action = action;
            self.time = 0.0;
        } else {
            eprintln!("action {action} not available for this model");
        }
    }

    fn action_label(&self) -> String {
        let avail: Vec<&str> = (0..ACTIONS.len())
            .filter(|&i| self.assets.clips[i].is_some())
            .map(|i| ACTION_NAMES[i])
            .collect();
        format!("{} [{}]", ACTION_NAMES[self.action], avail.join(", "))
    }

    fn update(&mut self, queue: &wgpu::Queue, dt: f32) {
        self.time += dt;
        let palette = match &self.assets.clips[self.action] {
            Some(clip) if clip.duration > 0.0 => {
                clip.skinning_palette(&self.assets.pose, self.time % clip.duration)
            }
            Some(clip) => clip.skinning_palette(&self.assets.pose, 0.0),
            None => self.assets.pose.bind_palette(),
        };
        self.renderer.set_palette(queue, &palette);
        self.global_uniforms.update_camera(queue, &self.camera);
    }

    fn encode_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        color: &wgpu::TextureView,
        depth: &wgpu::TextureView,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("gr2_viewer"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: color,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(CLEAR_COLOR),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
        self.renderer
            .render(&mut pass, &self.pipeline, &self.global_uniforms);
    }
}

// === Windowed mode ===

struct App {
    args: Args,
    window: Option<Arc<Window>>,
    device: Option<RenderDevice>,
    scene: Option<Scene>,
    mouse_pos: (f32, f32),
    last_mouse: (f32, f32),
    orbiting: bool,
    last_frame: Instant,

    /// Earliest instant the next frame may render. The event loop sleeps
    /// (`ControlFlow::WaitUntil`) until this point instead of spinning.
    next_frame: Instant,
}

/// The preferred `Mailbox` present mode never blocks to throttle us, so the
/// redraw cadence has to be paced here.
const FRAME_INTERVAL: Duration = Duration::from_micros(16_667);

impl App {
    fn update_title(&self) {
        if let (Some(window), Some(scene)) = (&self.window, &self.scene) {
            window.set_title(&format!(
                "GR2 Viewer - {} - {}",
                self.args.model,
                scene.action_label()
            ));
        }
    }

    fn render_frame(&mut self) {
        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f32();
        self.last_frame = now;

        let (Some(device), Some(scene)) = (&self.device, &mut self.scene) else {
            return;
        };

        scene.camera.aspect =
            device.surface_config.width as f32 / device.surface_config.height.max(1) as f32;
        scene.update(&device.queue, dt);

        let output = match device.surface.get_current_texture() {
            Ok(tex) => tex,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                device.reconfigure_surface();
                return;
            }
            Err(_) => return,
        };
        let view = output.texture.create_view(&Default::default());
        let mut encoder = device.device.create_command_encoder(&Default::default());
        scene.encode_pass(&mut encoder, &view, &device.depth_view);
        device.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = WindowAttributes::default()
            .with_title("GR2 Viewer")
            .with_inner_size(winit::dpi::PhysicalSize::new(
                self.args.width,
                self.args.height,
            ));
        let window = Arc::new(event_loop.create_window(attrs).unwrap());
        let device = block_on(RenderDevice::new(window.clone()));

        let grf = match GrfArchive::open(Path::new(&self.args.grf_path)) {
            Ok(g) => g,
            Err(e) => {
                eprintln!("Failed to open GRF '{}': {e}", self.args.grf_path);
                event_loop.exit();
                return;
            }
        };
        let texture_cache = TextureCache::new(&device.device, 1.0);
        let aspect = device.surface_config.width as f32 / device.surface_config.height as f32;
        let scene = Scene::new(
            &device.device,
            &device.queue,
            &texture_cache,
            device.surface_format,
            &grf,
            &self.args,
            aspect,
        );
        if scene.is_none() {
            eprintln!("Failed to load model '{}'", self.args.model);
            event_loop.exit();
            return;
        }
        self.device = Some(device);
        self.scene = scene;
        self.window = Some(window);
        self.update_title();
        self.last_frame = Instant::now();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(device) = &mut self.device {
                    device.resize(size.width, size.height);
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state != winit::event::ElementState::Pressed {
                    return;
                }
                match &event.logical_key {
                    Key::Named(NamedKey::Escape) => event_loop.exit(),
                    Key::Character(ch) => {
                        if let Ok(n) = ch.parse::<usize>() {
                            if let Some(scene) = &mut self.scene {
                                scene.set_action(n);
                            }
                            self.update_title();
                        } else if ch == "r" || ch == "R" {
                            if let Some(scene) = &mut self.scene {
                                scene.frame_camera();
                            }
                        }
                    }
                    _ => {}
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let dy = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, dy) => dy,
                    winit::event::MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 40.0,
                };
                if let Some(scene) = &mut self.scene {
                    scene.camera.distance = (scene.camera.distance * (1.0 - dy * 0.1)).max(1.0);
                }
            }
            WindowEvent::MouseInput { button, state, .. } => {
                if matches!(
                    button,
                    winit::event::MouseButton::Left | winit::event::MouseButton::Right
                ) {
                    self.orbiting = state == winit::event::ElementState::Pressed;
                    self.last_mouse = self.mouse_pos;
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_pos = (position.x as f32, position.y as f32);
                if self.orbiting {
                    let dx = self.mouse_pos.0 - self.last_mouse.0;
                    let dy = self.mouse_pos.1 - self.last_mouse.1;
                    if let Some(scene) = &mut self.scene {
                        scene.camera.yaw += dx * 0.01;
                        scene.camera.pitch = (scene.camera.pitch + dy * 0.01).clamp(-1.4, 1.4);
                    }
                    self.last_mouse = self.mouse_pos;
                }
            }
            WindowEvent::RedrawRequested => self.render_frame(),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        if now >= self.next_frame {
            self.next_frame = now + FRAME_INTERVAL;
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.next_frame));
    }
}

// === Headless screenshot mode ===

fn screenshot(args: &Args, out_path: &str) {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all().with_env(),
        ..Default::default()
    });
    let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))
    .expect("no gpu adapter");
    let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("gr2-viewer-headless"),
        ..Default::default()
    }))
    .expect("request device");

    let grf = GrfArchive::open(Path::new(&args.grf_path)).expect("open grf");
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let texture_cache = TextureCache::new(&device, 1.0);
    let aspect = args.width as f32 / args.height as f32;
    let mut scene = Scene::new(&device, &queue, &texture_cache, format, &grf, args, aspect)
        .expect("load model");

    let extent = wgpu::Extent3d {
        width: args.width,
        height: args.height,
        depth_or_array_layers: 1,
    };
    let color = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("gr2_screenshot_color"),
        size: extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let depth = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("gr2_screenshot_depth"),
        size: extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });

    scene.time = args.time;
    scene.update(&queue, 0.0);

    let color_view = color.create_view(&Default::default());
    let depth_view = depth.create_view(&Default::default());
    let mut encoder = device.create_command_encoder(&Default::default());
    scene.encode_pass(&mut encoder, &color_view, &depth_view);

    let row_pitch = (args.width * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("gr2_screenshot_readback"),
        size: (row_pitch * args.height) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &color,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row_pitch),
                rows_per_image: Some(args.height),
            },
        },
        extent,
    );
    queue.submit(std::iter::once(encoder.finish()));

    let slice = readback.slice(..);
    let (tx, rx) = mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |res| {
        let _ = tx.send(res);
    });
    let _ = device.poll(wgpu::PollType::wait_indefinitely());
    rx.recv().expect("map_async channel").expect("buffer map");

    let data = slice.get_mapped_range();
    let mut rgba = Vec::with_capacity((args.width * args.height * 4) as usize);
    for y in 0..args.height {
        let start = (y * row_pitch) as usize;
        rgba.extend_from_slice(&data[start..start + (args.width * 4) as usize]);
    }
    drop(data);

    let img = image::RgbaImage::from_raw(args.width, args.height, rgba).expect("image");
    img.save(out_path).expect("save png");
    println!(
        "wrote {out_path} ({}x{}, action={}, t={})",
        args.width, args.height, ACTION_NAMES[scene.action], args.time
    );
}

pub fn run(args: Args) {
    if let Some(out) = args.screenshot.clone() {
        screenshot(&args, &out);
        return;
    }
    let event_loop = EventLoop::new().unwrap();
    let mut app = App {
        args,
        window: None,
        device: None,
        scene: None,
        mouse_pos: (0.0, 0.0),
        last_mouse: (0.0, 0.0),
        orbiting: false,
        last_frame: Instant::now(),
        next_frame: Instant::now(),
    };
    event_loop.run_app(&mut app).unwrap();
}
