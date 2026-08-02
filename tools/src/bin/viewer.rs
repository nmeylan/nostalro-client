use ragnarok_tools::viewer;

const DEFAULT_GRF_PATH: &str = ragnarok_resources::grf::DEFAULT_ARCHIVE;
const DEFAULT_MAP: &str = "prontera";

fn main() {
    let args = parse_args();
    viewer::run(args);
}

fn parse_args() -> viewer::Args {
    let args: Vec<String> = std::env::args().collect();
    let mut grf_path: Option<String> = None;
    let mut map_name: Option<String> = None;
    let mut effect_id: Option<u16> = None;
    let mut cell: Option<(i32, i32)> = None;
    let mut direction: Option<u8> = None;
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
            "--effect" => {
                i += 1;
                if i < args.len() {
                    effect_id = args[i].parse::<u16>().ok();
                }
            }
            "--cell" => {
                i += 1;
                if i < args.len() {
                    cell = parse_cell(&args[i]);
                    if cell.is_none() {
                        eprintln!("Invalid --cell '{}', expected 'X,Y'", args[i]);
                    }
                }
            }
            "--direction" => {
                i += 1;
                if i < args.len() {
                    direction = args[i].parse::<u8>().ok().map(|d| d & 0x07);
                }
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            _ => {}
        }
        i += 1;
    }

    viewer::Args {
        grf_path: grf_path.unwrap_or_else(|| DEFAULT_GRF_PATH.to_string()),
        map_name: map_name.unwrap_or_else(|| DEFAULT_MAP.to_string()),
        effect_id,
        cell,
        direction,
    }
}

fn parse_cell(s: &str) -> Option<(i32, i32)> {
    let (x, y) = s.split_once(',')?;
    Some((x.trim().parse().ok()?, y.trim().parse().ok()?))
}

fn print_help() {
    println!("Unified viewer - map + character + effect preview");
    println!();
    println!(
        "Usage: viewer [--grf <path>] [--map <name>] [--cell X,Y] [--direction N] [--effect <id>]"
    );
    println!();
    println!("Options:");
    println!("  --grf <path>       Path to GRF (default: {DEFAULT_GRF_PATH})");
    println!("  --map <name>       Map to load (default: {DEFAULT_MAP})");
    println!("  --cell X,Y         Character GAT cell (default: walkable GAT center)");
    println!("  --direction N      Character facing 0..7 (default: 0)");
    println!("  --effect <id>      Effect id to play on startup");
    println!();
    println!("Controls:");
    println!("  Left click         Move character to clicked cell");
    println!("  B                  Cycle background: RSW map -> ground proxy -> clear");
    println!("  Right drag         Orbit camera");
    println!("  Scroll wheel       Zoom");
    println!("  +/-                Zoom in/out");
    println!("  C                  Reset camera on character");
    println!("  Space              Toggle pause (animation + effects)");
    println!("  Tab                Open effect browser");
    println!();
    println!("Character:");
    println!("  Arrow Up/Down      Cycle action (idle/attack/cast/...)");
    println!("  Arrow Left/Right   Cycle direction");
    println!("  Q / W              Cycle weapon view id");
    println!("  S                  Toggle sex");
    println!("  h / Shift+H        Cycle head id");
    println!("  e / Shift+E        Cycle headgear (top)");
    println!("  D / F              Cycle shield");
    println!();
    println!("Effects:");
    println!("  N / P              Next / prev effect from preset list");
    println!("  R                  Replay current effect at character position");
}
