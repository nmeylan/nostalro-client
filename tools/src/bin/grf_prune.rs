//! Reports which GRF entries nothing can reach, and optionally writes a copy
//! without them.
//!
//! Reports only unless `--write` is given; the input archive is never modified.

use ragnarok_formats::grf::GrfArchive;
use ragnarok_tools::grf_audit::{self, Category, Options};
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

const USAGE: &str = "\
usage: grf-prune <archive.grf> [options]

  --write <out.grf>   write the pruned archive (default: report only)
  --server <path>     rathena checkout, or a text file of map names (one per
                      line); maps outside the list stop being roots, freeing
                      the textures and models only they use
  --prune <cats>      comma-separated categories to drop
                      (default: texture,model,imf)
  --list [<cat>]      print the paths that would be dropped
  --keep <prefix>     never drop entries under this prefix (repeatable)

categories: map texture model sprite palette sound imf
";

struct Args {
    archive: PathBuf,
    write: Option<PathBuf>,
    server: Option<PathBuf>,
    prune: HashSet<Category>,
    list: Option<Option<Category>>,
    keep: Vec<String>,
}

fn parse_args() -> Result<Args, String> {
    let mut it = std::env::args().skip(1);
    let archive = it.next().ok_or_else(|| USAGE.to_string())?;
    if archive.starts_with('-') {
        return Err(USAGE.to_string());
    }
    let mut args = Args {
        archive: PathBuf::from(archive),
        write: None,
        server: None,
        prune: [Category::Texture, Category::Model, Category::Imf]
            .into_iter()
            .collect(),
        list: None,
        keep: Vec::new(),
    };
    while let Some(flag) = it.next() {
        let mut value = || {
            it.next()
                .ok_or_else(|| format!("{flag} needs a value\n\n{USAGE}"))
        };
        match flag.as_str() {
            "--write" => args.write = Some(PathBuf::from(value()?)),
            "--server" => args.server = Some(PathBuf::from(value()?)),
            "--keep" => args.keep.push(grf_audit::normalize(&value()?)),
            "--prune" => {
                args.prune = value()?
                    .split(',')
                    .map(|c| {
                        Category::parse(c.trim())
                            .ok_or_else(|| format!("unknown category `{c}`\n\n{USAGE}"))
                    })
                    .collect::<Result<_, _>>()?;
            }
            "--list" => {
                // The optional category sits in the next slot only if it parses.
                args.list = Some(None);
            }
            "-h" | "--help" => return Err(USAGE.to_string()),
            other if other.starts_with('-') => {
                return Err(format!("unknown flag `{other}`\n\n{USAGE}"));
            }
            other => match Category::parse(other) {
                Some(c) if args.list == Some(None) => args.list = Some(Some(c)),
                _ => return Err(format!("unexpected argument `{other}`\n\n{USAGE}")),
            },
        }
    }
    Ok(args)
}

fn mb(bytes: u64) -> String {
    format!("{:.1}MB", bytes as f64 / 1e6)
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
            server_path: args.server.clone(),
            ..Options::default()
        },
    );

    let kept_by_flag = |name: &str| args.keep.iter().any(|p| name.starts_with(p.as_str()));
    let prunable: Vec<&grf_audit::Entry> = report
        .prunable(&args.prune)
        .filter(|e| !kept_by_flag(&e.name))
        .collect();
    let prune_names: HashSet<&str> = prunable.iter().map(|e| e.name.as_str()).collect();

    let dropped_bytes: u64 = prunable.iter().map(|e| e.bytes).sum();
    let total = report.total_bytes();

    println!(
        "{} — {} entries, {}",
        args.archive.display(),
        report.entries.len(),
        mb(total)
    );
    println!(
        "{} roots, {} entries reached\n",
        report.roots,
        report.reached.len()
    );

    #[derive(Default)]
    struct Tally {
        keep_files: usize,
        keep_bytes: u64,
        drop_files: usize,
        drop_bytes: u64,
    }
    let mut tally: BTreeMap<Category, Tally> = BTreeMap::new();
    for entry in &report.entries {
        let t = tally.entry(entry.category).or_default();
        if prune_names.contains(entry.name.as_str()) {
            t.drop_files += 1;
            t.drop_bytes += entry.bytes;
        } else {
            t.keep_files += 1;
            t.keep_bytes += entry.bytes;
        }
    }

    println!(
        "{:<14} {:>7} {:>10} {:>7} {:>10}",
        "", "keep", "", "drop", ""
    );
    for (category, t) in &tally {
        let note = match report.enumerable.get(category) {
            Some(Err(reason)) => format!("  ({reason})"),
            _ if !args.prune.contains(category) => "  (not selected)".to_string(),
            _ => String::new(),
        };
        println!(
            "{:<14} {:>7} {:>10} {:>7} {:>10}{}",
            category.name(),
            t.keep_files,
            mb(t.keep_bytes),
            t.drop_files,
            mb(t.drop_bytes),
            note
        );
    }

    println!(
        "\ntotal: keep {} / drop {} ({:.1}% smaller)",
        mb(total - dropped_bytes),
        mb(dropped_bytes),
        100.0 * dropped_bytes as f64 / total.max(1) as f64
    );

    if let Some(filter) = args.list {
        println!();
        for entry in prunable
            .iter()
            .filter(|e| filter.is_none_or(|c| e.category == c))
        {
            println!("{}", entry.name);
        }
    }

    let Some(out) = args.write else {
        println!("\n(report only — pass --write <out.grf> to build the pruned archive)");
        return;
    };
    if let Err(e) = write_pruned(&args.archive, &out, &prune_names) {
        eprintln!("\nwrite failed: {e}");
        std::process::exit(1);
    }
    let written = std::fs::metadata(&out).map(|m| m.len()).unwrap_or(0);
    println!("\nwrote {} ({})", out.display(), mb(written));
}

/// Copies the archive, then removes the dropped entries and repacks so the
/// freed space is actually reclaimed.
fn write_pruned(source: &Path, out: &Path, drop: &HashSet<&str>) -> Result<(), String> {
    if out == source {
        return Err("refusing to overwrite the source archive".to_string());
    }
    std::fs::copy(source, out).map_err(|e| format!("copy: {e}"))?;

    let mut copy = GrfArchive::open_rw(out).map_err(|e| format!("open {}: {e}", out.display()))?;
    for name in drop {
        copy.remove_file(name)
            .map_err(|e| format!("remove {name}: {e}"))?;
    }
    copy.save().map_err(|e| format!("save: {e}"))?;
    copy.repack().map_err(|e| format!("repack: {e}"))?;
    Ok(())
}
