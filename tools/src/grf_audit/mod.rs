//! Works out which GRF entries the client can actually reach.
//!
//! Two tools sit on top of this: `grf-prune` drops what is unreachable, and
//! `grf-verify` reports what is reachable but absent. Both need the same answer,
//! so the reachability walk lives here.
//!
//! The walk starts from roots — paths the client names outright (the
//! `ragnarok-resources` registry) or builds from a data table — and follows
//! every file-to-file reference from there: a map to its ground and models, a
//! model to its textures, a sprite to its animation.

pub mod closure;
pub mod roots;
pub mod server;

use ragnarok_formats::grf::GrfArchive;
use std::collections::{BTreeMap, HashMap, HashSet};

/// What kind of resource an entry is, which decides whether we trust ourselves
/// to say it is unused; see the `enumerable` map on [`Report`].
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Category {
    Map,
    Texture,
    Model,
    Sprite,
    Palette,
    Sound,
    Imf,
    /// Anything we have no story for. Never pruned.
    Unclassified,
}

impl Category {
    /// A folder alone is not enough: these trees also hold formats the client
    /// never opens (`.fna` beside the `.imf`s, stray `.psd`s beside textures).
    /// Anything whose extension we do not model stays [`Category::Unclassified`]
    /// and is therefore never dropped.
    pub fn of(path: &str) -> Self {
        let rest = path.strip_prefix("data/").unwrap_or(path);
        let ext = rest
            .rsplit('/')
            .next()
            .and_then(|f| f.rsplit_once('.'))
            .map(|(_, e)| e);
        let is = |exts: &[&str]| ext.is_some_and(|e| exts.contains(&e));

        if rest.starts_with("texture/") && is(&["bmp", "tga", "jpg", "jpeg", "png", "gif"]) {
            Category::Texture
        } else if rest.starts_with("model/") && is(&["rsm", "gr2"]) {
            Category::Model
        } else if rest.starts_with("sprite/") && is(&["spr", "act"]) {
            Category::Sprite
        } else if rest.starts_with("palette/") && is(&["pal"]) {
            Category::Palette
        } else if rest.starts_with("wav/") && is(&["wav", "mp3"]) {
            Category::Sound
        } else if rest.starts_with("imf/") && is(&["imf"]) {
            Category::Imf
        } else if !rest.contains('/') && is(&["rsw", "gnd", "gat"]) {
            Category::Map
        } else {
            Category::Unclassified
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Category::Map => "map",
            Category::Texture => "texture",
            Category::Model => "model",
            Category::Sprite => "sprite",
            Category::Palette => "palette",
            Category::Sound => "sound",
            Category::Imf => "imf",
            Category::Unclassified => "unclassified",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        [
            Category::Map,
            Category::Texture,
            Category::Model,
            Category::Sprite,
            Category::Palette,
            Category::Sound,
            Category::Imf,
        ]
        .into_iter()
        .find(|c| c.name() == name)
    }
}

/// Where a reference came from, so `grf-verify` can say who wanted a missing file.
#[derive(Clone, Debug)]
pub enum Origin {
    /// A constant in the resource registry.
    Registry,
    /// Built from a data table's contents.
    Table(&'static str),
    /// Named by another GRF entry.
    File(String),
}

impl std::fmt::Display for Origin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Origin::Registry => write!(f, "resource registry"),
            Origin::Table(t) => write!(f, "{t}"),
            Origin::File(p) => write!(f, "{p}"),
        }
    }
}

/// Whether the client truly needs a path, or merely tries it.
///
/// Most roots are combinations — every job crossed with every palette id, every
/// headgear crossed with both sexes. The client asks for those and copes when
/// they are absent, so their absence is not a defect. Only [`Need::Required`]
/// misses are worth reporting.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Need {
    Required,
    Probed,
}

pub struct Missing {
    pub path: String,
    pub origin: Origin,
    pub need: Need,
}

pub struct Entry {
    pub name: String,
    pub bytes: u64,
    pub category: Category,
}

pub struct Report {
    pub entries: Vec<Entry>,
    /// Indices into `entries` the walk reached.
    pub reached: HashSet<usize>,
    pub missing: Vec<Missing>,
    /// Categories we are willing to call complete for this archive, with the
    /// reason when we are not.
    pub enumerable: BTreeMap<Category, Result<(), String>>,
    pub roots: usize,
}

impl Report {
    pub fn unreached(&self) -> impl Iterator<Item = &Entry> {
        self.entries
            .iter()
            .enumerate()
            .filter(|(i, _)| !self.reached.contains(i))
            .map(|(_, e)| e)
    }

    /// Unreached entries in a category we trust, i.e. safe to drop.
    pub fn prunable<'a>(
        &'a self,
        allowed: &'a HashSet<Category>,
    ) -> impl Iterator<Item = &'a Entry> {
        self.unreached().filter(move |e| {
            allowed.contains(&e.category) && self.enumerable.get(&e.category) == Some(&Ok(()))
        })
    }

    pub fn total_bytes(&self) -> u64 {
        self.entries.iter().map(|e| e.bytes).sum()
    }
}

pub struct Options {
    /// Path to a rathena checkout. Narrows the map list (and therefore every
    /// texture and model only those maps use) to what the server can send.
    pub server_path: Option<std::path::PathBuf>,
    /// Head sprites are numbered, not listed anywhere; probe ids up to here.
    pub max_head_id: u16,
    /// Same for hair/clothes palette ids.
    pub max_palette_id: u16,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            server_path: None,
            max_head_id: 64,
            max_palette_id: 32,
        }
    }
}

pub fn normalize(name: &str) -> String {
    name.replace('\\', "/").to_lowercase()
}

pub fn run(grf: &GrfArchive, opts: &Options) -> Report {
    let entries: Vec<Entry> = grf
        .file_list()
        .into_iter()
        .map(|f| {
            let name = normalize(&f.name);
            Entry {
                category: Category::of(&name),
                name,
                bytes: f.compressed_size_aligned as u64,
            }
        })
        .collect();
    let index: HashMap<&str, usize> = entries
        .iter()
        .enumerate()
        .map(|(i, e)| (e.name.as_str(), i))
        .collect();

    let server = opts
        .server_path
        .as_deref()
        .map(server::ServerData::load)
        .transpose();
    let (server, server_error) = match server {
        Ok(s) => (s, None),
        Err(e) => (None, Some(e)),
    };

    let roots = roots::collect(grf, opts, server.as_ref());
    let n_roots = roots.len();
    let walk = closure::walk(grf, roots);

    let mut reached = HashSet::with_capacity(walk.reached.len());
    for path in &walk.reached {
        if let Some(&i) = index.get(path.as_str()) {
            reached.insert(i);
        }
    }

    let mut enumerable = BTreeMap::new();
    enumerable.insert(Category::Texture, Ok(()));
    enumerable.insert(Category::Model, Ok(()));
    enumerable.insert(Category::Imf, Ok(()));
    enumerable.insert(
        Category::Sprite,
        if roots::has_identity_tables(grf) {
            Ok(())
        } else {
            Err(
                "no identity lua in the archive: the builtin job table may not \
                 name every sprite this server can send"
                    .to_string(),
            )
        },
    );
    enumerable.insert(
        Category::Palette,
        if roots::has_identity_tables(grf) {
            Ok(())
        } else {
            Err("depends on the job list, which is incomplete here".to_string())
        },
    );
    enumerable.insert(
        Category::Sound,
        Err(
            "monster and NPC sounds are chosen by the server, not by any client \
             table we can enumerate"
                .to_string(),
        ),
    );
    enumerable.insert(
        Category::Map,
        match (&server, &server_error) {
            (Some(s), _) => {
                if s.maps.is_empty() {
                    Err("the server checkout listed no maps".to_string())
                } else {
                    Ok(())
                }
            }
            (None, Some(e)) => Err(format!("could not read the server checkout: {e}")),
            (None, None) => {
                Err("every map is a root without --server; pass one to narrow them".to_string())
            }
        },
    );
    enumerable.insert(Category::Unclassified, Err("not understood".to_string()));

    Report {
        entries,
        reached,
        missing: walk.missing,
        enumerable,
        roots: n_roots,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The whole pipeline on a tiny archive: a registry constant makes one
    /// sprite a root, the walk pulls in its animation, and the texture nothing
    /// names is the only thing offered up for pruning.
    #[test]
    fn only_unreachable_entries_are_offered_for_pruning() {
        let dir = std::env::temp_dir().join("grf_audit_report_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.grf");

        let cursors = ragnarok_resources::sprite::CURSORS_SPR;
        let animation = ragnarok_resources::sprite::CURSORS_ACT;
        let orphan = "data/texture/nobody_wants_me.bmp";

        let mut grf = GrfArchive::create(&path).unwrap();
        grf.add_file(cursors, b"spr").unwrap();
        grf.add_file(animation, b"act").unwrap();
        grf.add_file(orphan, b"bmp").unwrap();
        grf.save().unwrap();
        drop(grf);

        let grf = GrfArchive::open(&path).unwrap();
        let report = run(&grf, &Options::default());

        let reached: Vec<&str> = report
            .reached
            .iter()
            .map(|&i| report.entries[i].name.as_str())
            .collect();
        assert!(reached.contains(&cursors), "named by the registry");
        assert!(reached.contains(&animation), "pulled in by its sprite");
        assert!(!reached.contains(&orphan));

        let allowed: HashSet<Category> = [Category::Texture].into_iter().collect();
        let prunable: Vec<&str> = report.prunable(&allowed).map(|e| e.name.as_str()).collect();
        assert_eq!(prunable, vec![orphan]);

        // Maps cannot be pruned without a narrowing list, even when unreached.
        let all: HashSet<Category> = [Category::Texture, Category::Map].into_iter().collect();
        assert!(report.enumerable[&Category::Map].is_err());
        assert_eq!(report.prunable(&all).count(), 1, "the map rule still holds");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
