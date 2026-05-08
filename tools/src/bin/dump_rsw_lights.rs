use ragnarok_formats::gnd::GndFile;
use ragnarok_formats::grf::GrfArchive;
use ragnarok_formats::rsw::{RswFile, RswObject};
use std::path::Path;

fn main() {
    let grf_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/data.grf".into());
    let maps: Vec<String> = std::env::args().skip(2).collect();
    let grf = GrfArchive::open(Path::new(&grf_path)).expect("open grf");

    let map_names: Vec<String> = if !maps.is_empty() {
        maps
    } else {
        vec!["prt_in".into(), "pay_dun03".into(), "alde_dun01".into()]
    };

    for map in &map_names {
        let rsw_path = format!("data/{}.rsw", map);
        let rsw_bytes = match grf.read_file(&rsw_path) {
            Ok(b) => b,
            Err(e) => {
                println!("[{}] rsw read error: {}", map, e);
                continue;
            }
        };
        let rsw = match RswFile::parse(&rsw_bytes) {
            Ok(r) => r,
            Err(e) => {
                println!("[{}] rsw parse error: {}", map, e);
                continue;
            }
        };
        let gnd_path = format!("data/{}", rsw.gnd_file);
        let gnd_bytes = match grf.read_file(&gnd_path) {
            Ok(b) => b,
            Err(e) => {
                println!("[{}] gnd read error ({}): {}", map, gnd_path, e);
                continue;
            }
        };
        let gnd = match GndFile::parse(&gnd_bytes) {
            Ok(g) => g,
            Err(e) => {
                println!("[{}] gnd parse error: {}", map, e);
                continue;
            }
        };

        let scale_factor = gnd.zoom / 10.0;
        let center_x = gnd.width as f32 * gnd.zoom / 2.0;
        let center_z = gnd.height as f32 * gnd.zoom / 2.0;

        // GND height stats
        let mut h_min = f32::INFINITY;
        let mut h_max = f32::NEG_INFINITY;
        let mut h_sum = 0.0f32;
        let mut h_count = 0usize;
        for c in &gnd.cells {
            for h in [c.height_sw, c.height_se, c.height_nw, c.height_ne] {
                if h.is_finite() {
                    h_min = h_min.min(h);
                    h_max = h_max.max(h);
                    h_sum += h;
                    h_count += 1;
                }
            }
        }
        let h_avg = h_sum / h_count.max(1) as f32;

        println!(
            "[{}] gnd {}x{} zoom={} center=({:.0},{:.0}) heights min={:.1} avg={:.1} max={:.1}",
            map, gnd.width, gnd.height, gnd.zoom, center_x, center_z, h_min, h_avg, h_max,
        );

        // First 3 models with their world-transformed positions
        let mut shown = 0;
        for o in &rsw.objects {
            if let RswObject::Model(m) = o {
                let wx = m.position[0] * scale_factor + center_x;
                let wy = m.position[1] * scale_factor;
                let wz = m.position[2] * scale_factor + center_z;
                println!(
                    "    model raw=[{:.1},{:.1},{:.1}] -> world=[{:.1},{:.1},{:.1}] ({})",
                    m.position[0], m.position[1], m.position[2], wx, wy, wz, m.model_name,
                );
                shown += 1;
                if shown >= 3 {
                    break;
                }
            }
        }

        // First 3 lights with their world-transformed positions
        let mut shown = 0;
        for o in &rsw.objects {
            if let RswObject::Light(l) = o {
                let wx = l.position[0] * scale_factor + center_x;
                let wy = l.position[1] * scale_factor;
                let wz = l.position[2] * scale_factor + center_z;
                println!(
                    "    light raw=[{:.1},{:.1},{:.1}] -> world=[{:.1},{:.1},{:.1}] range={:.1} color=[{:.2},{:.2},{:.2}]",
                    l.position[0],
                    l.position[1],
                    l.position[2],
                    wx,
                    wy,
                    wz,
                    l.range,
                    l.color[0],
                    l.color[1],
                    l.color[2],
                );
                shown += 1;
                if shown >= 3 {
                    break;
                }
            }
        }
    }
}
