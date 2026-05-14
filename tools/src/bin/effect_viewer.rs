use ragnarok_tools::effect_viewer;

const DEFAULT_GRF_PATH: &str = "data/data.grf";

fn main() {
    let args = parse_args();
    effect_viewer::run(args);
}

fn parse_args() -> effect_viewer::Args {
    let args: Vec<String> = std::env::args().collect();
    let mut grf_path = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--grf" => {
                i += 1;
                if i < args.len() {
                    grf_path = Some(args[i].clone());
                }
            }
            "--help" | "-h" => {
                println!("Effect Viewer - hot-reloadable effect playback tool");
                println!();
                println!("Usage: effect-viewer [--grf <path>]");
                println!();
                println!("Options:");
                println!("  --grf <path>   Path to the GRF file (defaults to {DEFAULT_GRF_PATH})");
                println!();
                println!("Controls:");
                println!("  → / ←          Next / prev effect");
                println!("  ↑ / ↓          Prev / next filter family");
                println!("  Tab            Open browser (filter by typing, Enter to pick)");
                println!("  R              Replay current");
                println!("  Space          Pause / resume");
                println!("  + / -          Speed up / down");
                println!("  B              Toggle background (blue / black)");
                println!("  Esc            Quit");
                println!();
                println!("Hot reload:");
                println!("  Edit and recompile tools/effect-viewer-hot/ - the running");
                println!("  viewer picks up the new .so on its next render frame.");
                std::process::exit(0);
            }
            _ => {}
        }
        i += 1;
    }
    let grf_path = grf_path.unwrap_or_else(|| DEFAULT_GRF_PATH.to_string());
    effect_viewer::Args { grf_path }
}
