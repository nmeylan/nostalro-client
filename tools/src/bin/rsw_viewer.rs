use ragnarok_tools::rsw_viewer;

const DEFAULT_GRF_PATH: &str = ragnarok_resources::grf::DEFAULT_ARCHIVE;

fn main() {
    let args = parse_args();
    rsw_viewer::run(args);
}

fn parse_args() -> rsw_viewer::Args {
    let args: Vec<String> = std::env::args().collect();
    let mut grf_path = None;
    let mut map_name = None;
    let mut x = None;
    let mut y = None;
    let mut yaw = None;
    let mut pitch = None;
    let mut distance = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--grf" => {
                i += 1;
                if i < args.len() {
                    grf_path = Some(args[i].clone());
                }
            }
            "--map" => {
                i += 1;
                if i < args.len() {
                    map_name = Some(args[i].clone());
                }
            }
            "--x" => {
                i += 1;
                if i < args.len() {
                    x = args[i].parse::<i32>().ok();
                }
            }
            "--y" => {
                i += 1;
                if i < args.len() {
                    y = args[i].parse::<i32>().ok();
                }
            }
            "--yaw" => {
                i += 1;
                if i < args.len() {
                    yaw = args[i].parse::<f32>().ok().map(f32::to_radians);
                }
            }
            "--pitch" => {
                i += 1;
                if i < args.len() {
                    pitch = args[i].parse::<f32>().ok().map(f32::to_radians);
                }
            }
            "--distance" => {
                i += 1;
                if i < args.len() {
                    distance = args[i].parse::<f32>().ok();
                }
            }
            "--help" | "-h" => {
                println!("RSW Viewer - 3D map renderer for Ragnarok Online");
                println!();
                println!(
                    "Usage: rsw-viewer [--grf <path>] [--map <map_name>] [--x <cell>] [--y <cell>]\n                  [--yaw <deg>] [--pitch <deg>] [--distance <d>]"
                );
                println!();
                println!("Options:");
                println!("  --grf <path>   Path to the GRF file (defaults to {DEFAULT_GRF_PATH})");
                println!("  --map <name>   Map name to load (e.g., 'prontera')");
                println!("                 If not specified, opens the map browser");
                println!("  --x <cell>     Cell x to center the camera on (requires --y)");
                println!("  --y <cell>     Cell y to center the camera on (requires --x)");
                println!("  --yaw <deg>    Camera yaw in degrees (0 = looking along +Z)");
                println!("  --pitch <deg>  Camera pitch in degrees (clamped to 6..89)");
                println!("  --distance <d> Camera distance from the target (clamped to 20..3000)");
                println!();
                println!("Controls:");
                println!("  Left drag      Orbit camera around map");
                println!("  Right drag     Pan camera");
                println!("  Scroll wheel   Zoom in/out");
                println!("  b/B            Open map browser to switch maps");
                println!("  g/G            Toggle grid overlay");
                println!("  h/H            Toggle hover highlight");
                println!("  o/O            Cycle overlay mode");
                println!("  r/R            Reset camera position");
                println!("  +/-            Zoom in/out");
                println!("  Space          Pause/Resume water animation");
                println!("  1              Show controls panel");
                println!("  2              Show map information");
                println!("  Esc            Close info panel / map browser");
                std::process::exit(0);
            }
            _ => {}
        }
        i += 1;
    }

    let grf_path = grf_path.unwrap_or_else(|| DEFAULT_GRF_PATH.to_string());
    let start_cell = x.zip(y);

    rsw_viewer::Args {
        grf_path,
        map_name,
        start_cell,
        yaw,
        pitch,
        distance,
    }
}
