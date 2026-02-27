mod config;

use config::Config;
use ragnarok_formats::gnd::GndFile;
use ragnarok_formats::grf::GrfArchive;
use ragnarok_formats::rsw::RswFile;
use ragnarok_renderer::Renderer;
use std::path::Path;
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

struct App {
    config: Config,
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    grf: Option<GrfArchive>,
    right_mouse_down: bool,
    last_mouse_pos: Option<(f64, f64)>,
}

impl App {
    fn new(config: Config) -> Self {
        Self {
            config,
            window: None,
            renderer: None,
            grf: None,
            right_mouse_down: false,
            last_mouse_pos: None,
        }
    }

    fn load_map(&mut self, map_name: &str) {
        let grf = match &self.grf {
            Some(g) => g,
            None => return,
        };

        let rsw_path = format!("data/{map_name}.rsw");
        let rsw_data = match grf.read_file(&rsw_path) {
            Ok(d) => d,
            Err(e) => {
                tracing::error!("Failed to read RSW {rsw_path}: {e}");
                return;
            }
        };
        let rsw = match RswFile::parse(&rsw_data) {
            Ok(r) => r,
            Err(e) => {
                tracing::error!("Failed to parse RSW: {e}");
                return;
            }
        };

        let gnd_path = format!("data/{map_name}.gnd");
        let gnd_data = match grf.read_file(&gnd_path) {
            Ok(d) => d,
            Err(e) => {
                tracing::error!("Failed to read GND {gnd_path}: {e}");
                return;
            }
        };
        let gnd = match GndFile::parse(&gnd_data) {
            Ok(g) => g,
            Err(e) => {
                tracing::error!("Failed to parse GND: {e}");
                return;
            }
        };

        println!(
            "Map: {map_name} ({}x{}, {} textures, {} surfaces, {} lightmaps)",
            gnd.width,
            gnd.height,
            gnd.textures.len(),
            gnd.surfaces.len(),
            gnd.lightmaps.len()
        );

        if let Some(renderer) = &mut self.renderer {
            renderer.load_map(&gnd, &rsw, grf);
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attrs = WindowAttributes::default()
            .with_title("Ragnarok Online")
            .with_inner_size(winit::dpi::PhysicalSize::new(
                self.config.screen_width,
                self.config.screen_height,
            ));

        let window = Arc::new(event_loop.create_window(attrs).unwrap());
        let renderer = pollster::block_on(Renderer::new(window.clone()));

        self.window = Some(window);
        self.renderer = Some(renderer);

        // Load GRF and default map
        if let Some(grf_path) = self.config.grf_paths.first() {
            match GrfArchive::open(Path::new(grf_path)) {
                Ok(grf) => {
                    println!("GRF loaded: {} ({} files)", grf_path, grf.file_count());
                    self.grf = Some(grf);
                    self.load_map("prontera");
                }
                Err(e) => {
                    tracing::error!("Failed to open GRF {grf_path}: {e}");
                }
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size.width, size.height);
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if button == MouseButton::Right {
                    self.right_mouse_down = state == ElementState::Pressed;
                    if !self.right_mouse_down {
                        self.last_mouse_pos = None;
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                if self.right_mouse_down {
                    if let Some((lx, ly)) = self.last_mouse_pos {
                        let dx = (position.x - lx) as f32;
                        let dy = (position.y - ly) as f32;
                        if let Some(renderer) = &mut self.renderer {
                            renderer.camera.yaw += dx * 0.005;
                            renderer.camera.pitch = (renderer.camera.pitch - dy * 0.005)
                                .clamp(0.1, std::f32::consts::FRAC_PI_2 - 0.01);
                        }
                    }
                    self.last_mouse_pos = Some((position.x, position.y));
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let scroll = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 40.0,
                };
                if let Some(renderer) = &mut self.renderer {
                    renderer.camera.distance =
                        (renderer.camera.distance - scroll * 20.0).clamp(50.0, 1500.0);
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.render();
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let config = Config::load_or_default("config.json");
    println!("ragnarok-client (packetver: {})", config.packetver);

    let event_loop = EventLoop::new().unwrap();
    let mut app = App::new(config);
    event_loop.run_app(&mut app).unwrap();
}
