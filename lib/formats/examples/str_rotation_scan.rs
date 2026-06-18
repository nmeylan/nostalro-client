//! Scan every `.str` in the GRF and report which use a non-zero layer angle
//! and/or a non-centred vertical offset (offset[1] != 240). One line per file:
//!   <name-without-dir-or-ext>  rot=<max_abs_angle_deg>  yshift=<bool>
//! Usage: cargo run -p ragnarok-formats --example str_rotation_scan -- <grf>
use std::path::PathBuf;

use ragnarok_formats::grf::GrfArchive;
use ragnarok_formats::str_effect::StrEffectFile;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let grf_path = args.get(1).ok_or("usage: str_rotation_scan <grf>")?;
    let archive = GrfArchive::open(&PathBuf::from(grf_path))?;

    let mut names: Vec<String> = archive
        .file_names()
        .iter()
        .filter(|n| n.to_lowercase().ends_with(".str"))
        .map(|s| s.to_string())
        .collect();
    names.sort();

    for path in &names {
        let Ok(data) = archive.read_file(path) else {
            continue;
        };
        let Ok(str_file) = StrEffectFile::parse(&data) else {
            continue;
        };
        let mut max_brand = 0f32;
        let mut y_shift = false;
        for layer in &str_file.layers {
            for f in &layer.frames {
                if f.frame_type == 0 {
                    max_brand = max_brand.max(f.angle.abs());
                    if (f.offset[1] - 240.0).abs() > 1.0 {
                        y_shift = true;
                    }
                }
            }
        }
        // brand angle -> degrees
        let max_deg = max_brand * 360.0 / 1024.0;
        let stem = path
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or(path)
            .trim_end_matches(".str")
            .trim_end_matches(".STR");
        println!("{stem}\t{max_deg:.1}\t{y_shift}");
    }
    Ok(())
}
