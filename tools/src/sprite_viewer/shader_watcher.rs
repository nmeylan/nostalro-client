use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

pub struct ShaderWatcher {
    pub dirty: Arc<AtomicBool>,
    pub shader_path: PathBuf,
    _watcher: RecommendedWatcher,
}

impl ShaderWatcher {
    pub fn new(shaders_dir: &Path, shader_filename: &str) -> Result<Self, notify::Error> {
        let dirty = Arc::new(AtomicBool::new(false));
        let dirty_flag = dirty.clone();
        let watched_name = shader_filename.to_string();

        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                match event.kind {
                    EventKind::Modify(_) | EventKind::Create(_) => {
                        let matches = event.paths.iter().any(|p| {
                            p.file_name()
                                .and_then(|n| n.to_str())
                                .is_some_and(|n| n == watched_name)
                        });
                        if matches {
                            dirty_flag.store(true, Ordering::Relaxed);
                        }
                    }
                    _ => {}
                }
            }
        })?;

        watcher.watch(shaders_dir, RecursiveMode::NonRecursive)?;

        let shader_path = shaders_dir.join(shader_filename);

        Ok(Self {
            dirty,
            shader_path,
            _watcher: watcher,
        })
    }

    pub fn check_and_reload(&self) -> Option<String> {
        if self.dirty.swap(false, Ordering::Relaxed) {
            match std::fs::read_to_string(&self.shader_path) {
                Ok(source) => {
                    tracing::info!("Reloaded shader: {}", self.shader_path.display());
                    Some(source)
                }
                Err(e) => {
                    tracing::error!("Failed to read shader {}: {e}", self.shader_path.display());
                    None
                }
            }
        } else {
            None
        }
    }
}
