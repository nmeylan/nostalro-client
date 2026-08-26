use models::enums::effect_id::EffectId;
use ragnarok_effects::effect_queue::EffectQueue;
use ragnarok_effects::effect_trait::GroundSampler;
use ragnarok_formats::gat::GatFile;
use ragnarok_formats::map_coordinates::MapCoordinates;

pub const MAP_CLOUD_KEY: u32 = 0xFFEF_FFFF;

pub fn ground_sampler(gat: &GatFile, coords: &MapCoordinates) -> GroundSampler {
    let corners: Vec<[f32; 4]> = gat
        .cells
        .iter()
        .map(|c| [c.height_sw, c.height_se, c.height_nw, c.height_ne])
        .collect();
    let (w, h) = (gat.width, gat.height);
    let coords = *coords;
    std::sync::Arc::new(move |wx: f32, wz: f32| {
        let (fx, fy) = coords.world_to_cell_f(wx, wz);
        let (cx, cy) = (fx.floor(), fy.floor());
        if cx < 0.0 || cy < 0.0 || cx >= w as f32 || cy >= h as f32 {
            return 0.0;
        }
        let [sw, se, nw, ne] = corners[(cy as i32 * w + cx as i32) as usize];
        let (tx, ty) = (fx - cx, fy - cy);
        let top = sw * (1.0 - tx) + se * tx;
        let bot = nw * (1.0 - tx) + ne * tx;
        top * (1.0 - ty) + bot * ty
    })
}

#[derive(Default)]
pub struct MapCloudScheduler {
    wanted: Option<EffectId>,
    spawned: Option<EffectId>,
}

impl MapCloudScheduler {
    pub fn set_map(&mut self, map_name: &str) {
        self.wanted = crate::data_table::map_cloud_table::map_cloud_effect(map_name);
    }

    pub fn update(
        &mut self,
        player_id: Option<u32>,
        effects_enabled: bool,
        queue: &mut EffectQueue,
    ) {
        let target = match (self.wanted, player_id, effects_enabled) {
            (Some(id), Some(pid), true) => Some((id, pid)),
            _ => None,
        };
        match (self.spawned, target) {
            (Some(live), Some((id, _))) if live == id => {}
            (live, target) => {
                if live.is_some() {
                    queue.despawn(MAP_CLOUD_KEY);
                }
                if let Some((id, pid)) = target {
                    queue.spawn_on_keyed(id, pid, MAP_CLOUD_KEY);
                }
                self.spawned = target.map(|(id, _)| id);
            }
        }
    }

    pub fn clear(&mut self, queue: &mut EffectQueue) {
        self.wanted = None;
        if self.spawned.take().is_some() {
            queue.despawn(MAP_CLOUD_KEY);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn waits_for_the_player_then_spawns_once_and_follows_the_effect_toggle() {
        let mut sched = MapCloudScheduler::default();
        let mut queue = EffectQueue::new();

        sched.set_map("einbroch");
        sched.update(None, true, &mut queue);
        assert!(queue.drain().is_empty(), "no player entity yet");

        sched.update(Some(7), true, &mut queue);
        let reqs = queue.drain();
        assert_eq!(reqs.len(), 1);
        assert_eq!(reqs[0].effect_id, EffectId::Cloud4);
        assert_eq!(reqs[0].key, Some(MAP_CLOUD_KEY));

        sched.update(Some(7), true, &mut queue);
        assert!(queue.drain().is_empty(), "already live, not respawned");

        sched.update(Some(7), false, &mut queue);
        assert_eq!(queue.drain_despawns(), vec![MAP_CLOUD_KEY]);

        sched.update(Some(7), true, &mut queue);
        assert_eq!(queue.drain().len(), 1, "respawned when effects come back");
    }

    #[test]
    fn swapping_to_a_cloudless_map_despawns() {
        let mut sched = MapCloudScheduler::default();
        let mut queue = EffectQueue::new();
        sched.set_map("einbroch");
        sched.update(Some(7), true, &mut queue);
        queue.drain();

        sched.set_map("prontera");
        sched.update(Some(7), true, &mut queue);
        assert_eq!(queue.drain_despawns(), vec![MAP_CLOUD_KEY]);
        assert!(queue.drain().is_empty());
    }
}
