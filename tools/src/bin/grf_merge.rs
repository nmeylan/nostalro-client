//! Flattens a GRF stack into one archive, dropping what nothing reaches.
//!
//! Reports only unless `--write` is given; the input archives are never
//! modified.

use ragnarok_formats::grf::GrfArchive;
use ragnarok_tools::grf_audit::{self, Category, Options};
use ragnarok_tools::grf_merge;
use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;

const USAGE: &str = "\
usage: grf-merge <first.grf> [next.grf ...] [options]

  --write <out.grf>   write the merged archive (default: report only)
  --server <path>     rathena checkout, or a text file of map names (one per
                      line); maps outside the list stop being roots, freeing
                      the textures and models only they use
  --prune <cats>      comma-separated categories to drop
                      (default: texture,model)
  --list [<cat>]      print the paths that would be dropped
  --keep <prefix>     never drop entries under this prefix (repeatable)

categories: map texture ui model sprite palette sound imf

The first archive declared wins, as it does at runtime. Records of the
key-addressed data tables are folded across archives rather than taken whole
from the first, so an archive shipping a partial table keeps its rows instead
of losing them: the output deliberately holds more than the stack resolves to.
Entries are recompressed, so encrypted entries land unencrypted.
";

struct Args {
    archives: Vec<PathBuf>,
    write: Option<PathBuf>,
    server: Option<PathBuf>,
    prune: HashSet<Category>,
    list: Option<Option<Category>>,
    keep: Vec<String>,
}

fn parse_args() -> Result<Args, String> {
    let mut it = std::env::args().skip(1).peekable();
    let mut archives = Vec::new();
    while it.peek().is_some_and(|a| !a.starts_with('-')) {
        archives.push(PathBuf::from(it.next().unwrap()));
    }
    if archives.is_empty() {
        return Err(USAGE.to_string());
    }

    let mut args = Args {
        archives,
        write: None,
        server: None,
        // Not `imf`: the whole tree is a rounding error in bytes, and a
        // missing one silently reorders an actor's head over its body.
        prune: [Category::Texture, Category::Model].into_iter().collect(),
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
            "--list" => args.list = Some(None),
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

    let mut layers = Vec::with_capacity(args.archives.len());
    for path in &args.archives {
        match GrfArchive::open(path) {
            Ok(layer) => layers.push(layer),
            Err(e) => {
                eprintln!("cannot open {}: {e}", path.display());
                std::process::exit(1);
            }
        }
    }

    let paths: Vec<String> = args
        .archives
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    let mut grf = match GrfArchive::open_layered(&paths, None) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("cannot open the stack: {e}");
            std::process::exit(1);
        }
    };

    let merged = grf_merge::merge_tables(&layers);
    for (path, data) in &merged {
        grf.set_override(path, data.clone());
    }

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

    for (i, (path, layer)) in args.archives.iter().zip(&layers).enumerate() {
        println!(
            "{}. {} — {} entries",
            i + 1,
            path.display(),
            layer.file_count()
        );
    }
    println!("\nmerged — {} entries, {}", report.entries.len(), mb(total));
    println!(
        "{} roots, {} entries reached",
        report.roots,
        report.reached.len()
    );
    println!("{} table(s) folded across archives:", merged.len());
    for (path, data) in &merged {
        println!("  {path} ({})", mb(data.len() as u64));
    }
    println!();

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
        println!("\n(report only — pass --write <out.grf> to build the merged archive)");
        return;
    };

    let keep: Vec<String> = report
        .entries
        .iter()
        .filter(|e| !prune_names.contains(e.name.as_str()))
        .map(|e| e.name.clone())
        .collect();

    println!("\nwriting {} entries to {}", keep.len(), out.display());
    let total_entries = keep.len();
    let mut report_progress = |written: usize| {
        if written % 5000 == 0 || written == total_entries {
            println!(
                "  {written}/{total_entries} ({:.0}%)",
                100.0 * written as f64 / total_entries.max(1) as f64
            );
        }
    };
    let unreadable = match grf_merge::write_merged(&grf, &keep, &out, &mut report_progress) {
        Ok(skipped) => skipped,
        Err(e) => {
            eprintln!("\nwrite failed: {e}");
            std::process::exit(1);
        }
    };

    let written = std::fs::metadata(&out).map(|m| m.len()).unwrap_or(0);
    println!("\nwrote {} ({})", out.display(), mb(written));
    if !unreadable.is_empty() {
        println!(
            "{} entry/entries no archive could decompress:",
            unreadable.len()
        );
        for name in &unreadable {
            println!("  {name}");
        }
    }
}
