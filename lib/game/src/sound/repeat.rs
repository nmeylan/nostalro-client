use super::SoundQueue;

struct RepeatEntry {
    name: String,
    gid: u32,
    term_s: f32,
    timer_s: f32,
}

/// Server-driven repeating sounds (`ZC_SOUND` with the Repeat action). Each
/// entry replays every `term` at the actor's current position.
#[derive(Default)]
pub struct RepeatSoundScheduler {
    entries: Vec<RepeatEntry>,
}

impl RepeatSoundScheduler {
    pub fn new() -> Self {
        Self::default()
    }

    /// Play once immediately and start repeating.
    pub fn start(&mut self, name: String, gid: u32, term_ms: u32) {
        self.entries.push(RepeatEntry {
            name,
            gid,
            term_s: term_ms as f32 / 1000.0,
            timer_s: 0.0,
        });
    }

    /// Remove the first repeat matching `name` (ZC_SOUND Stop).
    pub fn stop(&mut self, name: &str) {
        if let Some(i) = self.entries.iter().position(|e| e.name == name) {
            self.entries.remove(i);
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn update(
        &mut self,
        dt: f32,
        resolve_pos: &dyn Fn(u32) -> Option<[f32; 3]>,
        out: &mut SoundQueue,
    ) {
        for e in &mut self.entries {
            e.timer_s += dt;
            if e.timer_s >= e.term_s {
                e.timer_s = 0.0;
                if let Some(pos) = resolve_pos(e.gid) {
                    out.world(e.name.clone(), pos);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeats_at_term_and_stops_by_name() {
        let mut s = RepeatSoundScheduler::new();
        s.start("wolf.wav".into(), 42, 1000);
        let pos = |_gid: u32| Some([1.0, 2.0, 3.0]);
        let mut q = SoundQueue::new();
        s.update(0.5, &pos, &mut q);
        assert_eq!(q.pending.len(), 0);
        s.update(0.6, &pos, &mut q);
        assert_eq!(q.pending.len(), 1);
        s.stop("wolf.wav");
        s.update(2.0, &pos, &mut q);
        assert_eq!(q.pending.len(), 1);
    }

    #[test]
    fn skips_tick_when_actor_gone() {
        let mut s = RepeatSoundScheduler::new();
        s.start("wolf.wav".into(), 42, 100);
        let gone = |_gid: u32| None;
        let mut q = SoundQueue::new();
        s.update(0.2, &gone, &mut q);
        assert_eq!(q.pending.len(), 0);
    }
}
