//! Reads `.gr2` entity models on a worker thread: everything a draw needs
//! except the GPU upload, which stays on the frame thread.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};

use ragnarok_formats::gr2::{Gr2Container, Gr2File};
use ragnarok_formats::grf::GrfArchive;
use ragnarok_game::gr2_model::{self, AnimationClip, Gr2Action, SkeletonPose};
use ragnarok_renderer::gr2_model::emblem_texture_index;
use ragnarok_renderer::{Gr2Geometry, Gr2TextureData, build_gr2_geometry, decode_textures};

/// One decoded `.gr2`, ready to be uploaded and instanced.
pub(crate) struct Gr2LoadedAsset {
    pub geometry: Gr2Geometry,
    pub textures: Vec<Gr2TextureData>,
    pub emblem_texture_index: Option<usize>,
    pub pose: SkeletonPose,
    pub clips: [Option<AnimationClip>; 5],
}

fn parse_gr2_file(bytes: &[u8], path: &str) -> Option<Gr2File> {
    let container = Gr2Container::parse(bytes)
        .map_err(|e| tracing::warn!("gr2 container parse failed for {path}: {e:?}"))
        .ok()?;
    Gr2File::parse(&container)
        .map_err(|e| tracing::warn!("gr2 extract failed for {path}: {e:?}"))
        .ok()
}

pub(crate) fn load_gr2_asset(
    grf: &GrfArchive,
    model_name: &str,
    path: &str,
) -> Option<Gr2LoadedAsset> {
    ragnarok_profiling::profile_function!();
    let bytes = match grf.read_file(path) {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!("cannot read gr2 model {path}: {e}");
            return None;
        }
    };
    let file = parse_gr2_file(&bytes, path)?;
    let Some(pose) = SkeletonPose::from_model(&file, 0) else {
        tracing::warn!("gr2 model {path} has no skeleton");
        return None;
    };
    let bone_type = gr2_model::bone_type_from_name(model_name);
    let clips: [Option<AnimationClip>; 5] = std::array::from_fn(|i| match Gr2Action::ALL[i] {
        Gr2Action::Stand => AnimationClip::from_gr2(&file, 0),
        action => {
            let anim_path = gr2_model::animation_file_path(bone_type?, action)?;
            let bytes = grf.read_file(&anim_path).ok()?;
            let anim_file = parse_gr2_file(&bytes, &anim_path)?;
            AnimationClip::from_gr2(&anim_file, 0)
        }
    });
    let Some(geometry) = build_gr2_geometry(&file, 0) else {
        tracing::warn!("gr2 model {path} produced no renderable geometry");
        return None;
    };

    Some(Gr2LoadedAsset {
        geometry,
        textures: decode_textures(&file),
        emblem_texture_index: emblem_texture_index(&file),
        pose,
        clips,
    })
}

struct Job {
    path: String,
    model_name: String,
    generation: u64,
}

struct Done {
    path: String,
    generation: u64,
    asset: Option<Gr2LoadedAsset>,
}

/// Reads GR2 models on a worker thread and hands the decoded result back to the
/// frame loop. One job per model file, however many entities are waiting on it.
pub(crate) struct Gr2Loader {
    jobs: Sender<Job>,
    done: Receiver<Done>,
    /// Entities waiting on each model file still being read.
    waiting: HashMap<String, Vec<u32>>,
    generation: u64,
}

impl Gr2Loader {
    pub(crate) fn spawn(grf: Arc<GrfArchive>) -> Self {
        let (job_tx, job_rx) = mpsc::channel::<Job>();
        let (done_tx, done_rx) = mpsc::channel::<Done>();
        std::thread::spawn(move || {
            for job in job_rx {
                let asset = load_gr2_asset(&grf, &job.model_name, &job.path);
                if done_tx
                    .send(Done {
                        path: job.path,
                        generation: job.generation,
                        asset,
                    })
                    .is_err()
                {
                    return;
                }
            }
        });
        Gr2Loader::new(job_tx, done_rx)
    }

    fn new(jobs: Sender<Job>, done: Receiver<Done>) -> Self {
        Gr2Loader {
            jobs,
            done,
            waiting: HashMap::new(),
            generation: 0,
        }
    }

    /// Queue `gid` for `path`, sending a job only when nothing else is already
    /// reading that model.
    pub(crate) fn request(&mut self, gid: u32, model_name: &str, path: &str) {
        match self.waiting.get_mut(path) {
            Some(gids) => {
                if !gids.contains(&gid) {
                    gids.push(gid);
                }
            }
            None => {
                self.waiting.insert(path.to_string(), vec![gid]);
                let _ = self.jobs.send(Job {
                    path: path.to_string(),
                    model_name: model_name.to_string(),
                    generation: self.generation,
                });
            }
        }
    }

    pub(crate) fn is_waiting(&self, gid: u32) -> bool {
        self.waiting.values().any(|gids| gids.contains(&gid))
    }

    /// Forget everything in flight; results already on their way are dropped.
    pub(crate) fn invalidate(&mut self) {
        self.waiting.clear();
        self.generation += 1;
    }

    /// Loads finished since the last call, as `(path, asset, waiting gids)`.
    pub(crate) fn take_ready(&mut self) -> Vec<(String, Option<Gr2LoadedAsset>, Vec<u32>)> {
        let mut ready = Vec::new();
        while let Ok(done) = self.done.try_recv() {
            if done.generation != self.generation {
                continue;
            }
            let Some(gids) = self.waiting.remove(&done.path) else {
                continue;
            };
            ready.push((done.path, done.asset, gids));
        }
        ready
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FLAG: &str = "data/model/3dmob/guildflag90_1.gr2";

    #[test]
    fn one_job_per_model_fans_out_to_every_waiter() {
        let (job_tx, job_rx) = mpsc::channel();
        let (done_tx, done_rx) = mpsc::channel();
        let mut loader = Gr2Loader::new(job_tx, done_rx);

        loader.request(10, "Guildflag90_1.gr2", FLAG);
        loader.request(11, "Guildflag90_1.gr2", FLAG);
        let job = job_rx.try_recv().expect("one job");
        assert_eq!(job.path, FLAG);
        assert!(
            job_rx.try_recv().is_err(),
            "second waiter re-read the model"
        );
        assert!(loader.is_waiting(11));

        done_tx
            .send(Done {
                path: FLAG.to_string(),
                generation: job.generation,
                asset: None,
            })
            .unwrap();
        let ready = loader.take_ready();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].2, vec![10, 11]);
        assert!(!loader.is_waiting(10));
    }

    #[test]
    fn a_load_outlived_by_its_map_is_dropped() {
        let (job_tx, job_rx) = mpsc::channel();
        let (done_tx, done_rx) = mpsc::channel();
        let mut loader = Gr2Loader::new(job_tx, done_rx);

        loader.request(10, "Guildflag90_1.gr2", FLAG);
        let job = job_rx.try_recv().expect("one job");
        loader.invalidate();
        done_tx
            .send(Done {
                path: FLAG.to_string(),
                generation: job.generation,
                asset: None,
            })
            .unwrap();

        assert!(loader.take_ready().is_empty());
        assert!(!loader.is_waiting(10));
    }
}
