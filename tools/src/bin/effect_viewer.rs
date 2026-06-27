use std::path::PathBuf;

use models::enums::EnumWithNumberValue;
use models::enums::effect_id::EffectId;
use ragnarok_tools::effect_viewer;

const DEFAULT_GRF_PATH: &str = "data/data.grf";

struct ParsedArgs {
    grf_path: String,
    export_gif: Option<u16>,
    out: Option<PathBuf>,
}

fn main() {
    let parsed = parse_args();
    let args = effect_viewer::Args {
        grf_path: parsed.grf_path,
    };
    match parsed.export_gif {
        Some(id_u16) => {
            let Ok(effect_id) = EffectId::try_from_value(id_u16 as usize) else {
                eprintln!("Unknown effect id: {id_u16}");
                std::process::exit(2);
            };
            let out = parsed
                .out
                .unwrap_or_else(|| PathBuf::from(format!("gif_export/{}.gif", id_u16)));
            effect_viewer::run_batch_export(args, effect_id, out);
        }
        None => effect_viewer::run(args),
    }
}

fn parse_args() -> ParsedArgs {
    let args: Vec<String> = std::env::args().collect();
    let mut grf_path = None;
    let mut export_gif = None;
    let mut out = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--grf" => {
                i += 1;
                if i < args.len() {
                    grf_path = Some(args[i].clone());
                }
            }
            "--export-gif" => {
                i += 1;
                if i < args.len() {
                    match args[i].parse::<u16>() {
                        Ok(v) => export_gif = Some(v),
                        Err(e) => {
                            eprintln!("Invalid --export-gif value '{}': {e}", args[i]);
                            std::process::exit(2);
                        }
                    }
                }
            }
            "--out" => {
                i += 1;
                if i < args.len() {
                    out = Some(PathBuf::from(args[i].clone()));
                }
            }
            "--help" | "-h" => {
                println!("Effect Viewer - hot-reloadable effect playback tool");
                println!();
                println!("Usage:");
                println!("  effect-viewer [--grf <path>]");
                println!("  effect-viewer --export-gif <effect-id> [--out <path>] [--grf <path>]");
                println!();
                println!("Options:");
                println!(
                    "  --grf <path>           Path to the GRF file (defaults to {DEFAULT_GRF_PATH})"
                );
                println!("  --export-gif <id>      Render <id> to GIF using the cdylib's default");
                println!("                         camera and exit. Window is created hidden.");
                println!("  --out <path>           GIF output path (default: gif_export/<id>.gif)");
                println!();
                println!("Controls (interactive):");
                println!("  → / ←          Next / prev effect");
                println!("  ↑ / ↓          Prev / next filter family");
                println!("  Tab            Open browser (filter by typing, Enter to pick)");
                println!("  R              Replay current");
                println!(
                    "  E              Export current effect to GIF (gif_export/<id>_<ts>.gif)"
                );
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
    ParsedArgs {
        grf_path: grf_path.unwrap_or_else(|| DEFAULT_GRF_PATH.to_string()),
        export_gif,
        out,
    }
}
