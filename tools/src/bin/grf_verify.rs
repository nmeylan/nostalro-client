//! Reports resources the client asks for that the archive does not contain.
//!
//! Exits non-zero when anything is missing, so it can gate a build.

use ragnarok_formats::grf::GrfArchive;
use ragnarok_tools::grf_audit::{self, Need, Options, Origin};
use std::collections::BTreeMap;
use std::path::PathBuf;

const USAGE: &str = "\
usage: grf-verify <archive.grf> [options]

  --server <path>     rathena checkout, or a text file of map names; checks
                      only those maps
  --limit <n>         paths to print per group (default 20, 0 for all)
  --quiet             counts only, no paths

exits 1 when anything is missing.
";

struct Args {
    archive: PathBuf,
    server: Option<PathBuf>,
    limit: usize,
    quiet: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut it = std::env::args().skip(1);
    let archive = it.next().ok_or_else(|| USAGE.to_string())?;
    if archive.starts_with('-') {
        return Err(USAGE.to_string());
    }
    let mut args = Args {
        archive: PathBuf::from(archive),
        server: None,
        limit: 20,
        quiet: false,
    };
    while let Some(flag) = it.next() {
        match flag.as_str() {
            "--server" => {
                args.server =
                    Some(PathBuf::from(it.next().ok_or_else(|| {
                        format!("--server needs a path\n\n{USAGE}")
                    })?))
            }
            "--limit" => {
                args.limit = it
                    .next()
                    .ok_or_else(|| format!("--limit needs a number\n\n{USAGE}"))?
                    .parse()
                    .map_err(|e| format!("--limit: {e}"))?
            }
            "--quiet" => args.quiet = true,
            "-h" | "--help" => return Err(USAGE.to_string()),
            other => return Err(format!("unknown flag `{other}`\n\n{USAGE}")),
        }
    }
    Ok(args)
}

fn main() {
    let args = match parse_args() {
        Ok(a) => a,
        Err(msg) => {
            eprintln!("{msg}");
            std::process::exit(2);
        }
    };

    let grf = match GrfArchive::open(&args.archive) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("cannot open {}: {e}", args.archive.display());
            std::process::exit(1);
        }
    };

    let report = grf_audit::run(
        &grf,
        &Options {
            server_path: args.server,
            ..Options::default()
        },
    );

    println!(
        "{} — {} entries, {} roots, {} reached",
        args.archive.display(),
        report.entries.len(),
        report.roots,
        report.reached.len()
    );

    let required: Vec<&grf_audit::Missing> = report
        .missing
        .iter()
        .filter(|m| m.need == Need::Required)
        .collect();
    let probed = report.missing.len() - required.len();
    if probed > 0 {
        println!(
            "{probed} probed paths absent (job/palette/headgear combinations the \
             client tries and does without) — not reported"
        );
    }

    if required.is_empty() {
        println!("\nnothing missing.");
        return;
    }

    // A missing file named by one the client also wants is a knock-on effect;
    // group by who asked so the root cause reads first.
    let mut by_origin: BTreeMap<String, Vec<&str>> = BTreeMap::new();
    for m in required.iter() {
        let key = match &m.origin {
            Origin::Registry => "declared in the resource registry".to_string(),
            Origin::Table(t) => format!("built from {t}"),
            Origin::File(p) => format!("referenced by {}", group_of(p)),
        };
        by_origin.entry(key).or_default().push(&m.path);
    }

    println!("\n{} missing:", required.len());
    for (origin, mut paths) in by_origin {
        paths.sort_unstable();
        println!("\n  {origin} — {}", paths.len());
        if args.quiet {
            continue;
        }
        let limit = if args.limit == 0 {
            paths.len()
        } else {
            args.limit
        };
        for path in paths.iter().take(limit) {
            println!("    {path}");
        }
        if paths.len() > limit {
            println!("    … {} more", paths.len() - limit);
        }
    }

    std::process::exit(1);
}

/// Collapses per-file referrers into their file type, so one line covers all
/// 570 maps rather than printing a heading each.
fn group_of(path: &str) -> &'static str {
    match path.rsplit('.').next() {
        Some("rsw") => "a map (.rsw)",
        Some("gnd") => "a ground mesh (.gnd)",
        Some("rsm") => "a model (.rsm)",
        Some("str") => "an effect script (.str)",
        Some("spr") => "a sprite (.spr)",
        Some("act") => "an animation (.act)",
        Some("gr2") => "a 3D model (.gr2)",
        _ => "another entry",
    }
}
