//! Dump an STR effect file's layers and per-frame animation keys.
//! Usage: cargo run -p ragnarok-formats --example dump_str -- <grf> <path-in-grf>
use std::path::PathBuf;

use ragnarok_formats::grf::GrfArchive;
use ragnarok_formats::str_effect::StrEffectFile;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let grf_path = args.get(1).ok_or("usage: dump_str <grf> <path>")?;
    let path = args.get(2).ok_or("usage: dump_str <grf> <path>")?;
    let archive = GrfArchive::open(&PathBuf::from(grf_path))?;
    let data = archive.read_file(path)?;
    let str_file = StrEffectFile::parse(&data)?;

    println!(
        "version={:?} fps={} max_key={} layers={}",
        str_file.version,
        str_file.fps,
        str_file.max_key,
        str_file.layers.len()
    );
    for (li, layer) in str_file.layers.iter().enumerate() {
        println!(
            "\n== layer {li} | textures={:?} | frames={} ==",
            layer.textures,
            layer.frames.len()
        );
        for f in &layer.frames {
            println!(
                "  idx={:>4} type={} off=[{:>8.2},{:>8.2}] angle={:>7.2} tex={:>4.1} anim={} delay={:>5.2} color=[{:.2},{:.2},{:.2},{:.2}] blend=({},{})",
                f.frame_index,
                f.frame_type,
                f.offset[0],
                f.offset[1],
                f.angle,
                f.texture_index,
                f.animation_mode,
                f.delay,
                f.color[0],
                f.color[1],
                f.color[2],
                f.color[3],
                f.blend_src,
                f.blend_dst,
            );
            if f.frame_type == 0 {
                println!(
                    "        pos=[{:.0},{:.0},{:.0},{:.0} | {:.0},{:.0},{:.0},{:.0}]",
                    f.positions[0], f.positions[1], f.positions[2], f.positions[3],
                    f.positions[4], f.positions[5], f.positions[6], f.positions[7],
                );
            }
        }
    }
    Ok(())
}
