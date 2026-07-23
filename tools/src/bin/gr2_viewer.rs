use ragnarok_tools::gr2_viewer;

const DEFAULT_GRF_PATH: &str = "data/data.grf";

fn main() {
    let args = parse_args();
    gr2_viewer::run(args);
}

fn parse_args() -> gr2_viewer::Args {
    let argv: Vec<String> = std::env::args().collect();
    let mut grf_path = None;
    let mut model = None;
    let mut action = 0usize;
    let mut screenshot = None;
    let mut time = 0.0f32;
    let mut yaw = 34.0f32;
    let mut emblem = None;
    let mut i = 1;
    while i < argv.len() {
        match argv[i].as_str() {
            "--grf" => {
                i += 1;
                grf_path = argv.get(i).cloned();
            }
            "--action" => {
                i += 1;
                action = argv.get(i).and_then(|s| s.parse().ok()).unwrap_or(0);
            }
            "--screenshot" => {
                i += 1;
                screenshot = argv.get(i).cloned();
            }
            "--time" => {
                i += 1;
                time = argv.get(i).and_then(|s| s.parse().ok()).unwrap_or(0.0);
            }
            "--yaw" => {
                i += 1;
                yaw = argv.get(i).and_then(|s| s.parse().ok()).unwrap_or(34.0);
            }
            "--emblem" => {
                i += 1;
                emblem = argv.get(i).cloned();
            }
            "--help" | "-h" => {
                println!("GR2 Viewer - Granny model renderer for Ragnarok Online");
                println!();
                println!("Usage: gr2-viewer [--grf <path>] [options] <model.gr2>");
                println!();
                println!("  <model.gr2>          Model name (e.g. 'empelium90_0.gr2', looked up");
                println!("                       under data/model/3dmob/) or a full GRF path");
                println!();
                println!("Options:");
                println!("  --grf <path>         GRF file (defaults to {DEFAULT_GRF_PATH})");
                println!(
                    "  --action <0-4>       Initial action: 0=stand 1=move 2=attack 3=dead 4=damage"
                );
                println!("  --screenshot <png>   Render one frame headless to a PNG and exit");
                println!("  --time <secs>        Animation time for --screenshot");
                println!("  --yaw <degrees>      Initial camera yaw (default 34, 0 = front-on)");
                println!("  --emblem <image>     Swap the model's emblem texture (guild flag);");
                println!("                       bmp (magenta = transparent), png, jpg…");
                println!();
                println!("Controls:");
                println!("  Drag                 Orbit camera");
                println!("  Scroll wheel         Zoom");
                println!("  0-4                  Switch action animation");
                println!("  r                    Reset camera");
                println!("  Esc                  Quit");
                std::process::exit(0);
            }
            other if !other.starts_with('-') => model = Some(other.to_string()),
            _ => {}
        }
        i += 1;
    }

    let Some(model) = model else {
        eprintln!("missing <model.gr2> argument (try --help)");
        std::process::exit(1);
    };

    gr2_viewer::Args {
        grf_path: grf_path.unwrap_or_else(|| DEFAULT_GRF_PATH.to_string()),
        model,
        action,
        screenshot,
        time,
        width: 1024,
        height: 768,
        yaw,
        emblem,
    }
}
