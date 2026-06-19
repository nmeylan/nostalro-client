use ragnarok_tools::rsw_viewer;

const DEFAULT_GRF_PATH: &str = "data/data.grf";

fn main() {
    let args = parse_args();
    rsw_viewer::run(args);
}

fn parse_args() -> rsw_viewer::Args {
    let args: Vec<String> = std::env::args().collect();
    let mut grf_path = None;
    let mut map_name = None;
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
            "--help" | "-h" => {
                println!("RSW Viewer - 3D map renderer for Ragnarok Online");
                println!();
                println!("Usage: rsw-viewer [--grf <path>] [--map <map_name>]");
                println!();
                println!("Options:");
                println!("  --grf <path>   Path to the GRF file (defaults to {DEFAULT_GRF_PATH})");
                println!("  --map <name>   Map name to load (e.g., 'prontera')");
                println!("                 If not specified, opens the map browser");
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

    rsw_viewer::Args { grf_path, map_name }
}
