#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DamageMessage {
    /// Target flinches and takes damage
    Attacked,
    /// Target takes damage without flinch (magic skills, endure)
    AttackedNoMotion,
    /// Multi-hit: per-hit damage, final hit can trigger total display
    AttackedMultiHit { total_damage: i32 },
}

pub struct ScheduledHit {
    pub message: DamageMessage,
    pub damage: i32,
    pub fire_at: f32,
    pub attacker_gid: u32,
    pub skill_id: u16,
    pub is_last_hit: bool,
    pub is_critical: bool,
    pub hit_index: u16,
}

pub struct ScheduledHitQueue {
    events: Vec<ScheduledHit>,
}

impl ScheduledHitQueue {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn push(&mut self, event: ScheduledHit) {
        self.events.push(event);
    }

    pub fn drain_ready(&mut self, now: f32) -> Vec<ScheduledHit> {
        let mut ready = Vec::new();
        let mut i = 0;
        while i < self.events.len() {
            if self.events[i].fire_at <= now {
                ready.push(self.events.remove(i));
            } else {
                i += 1;
            }
        }
        ready
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drain_ready_returns_events_at_or_before_now() {
        let mut queue = ScheduledHitQueue::new();
        queue.push(ScheduledHit {
            message: DamageMessage::Attacked,
            damage: 100,
            fire_at: 1.0,
            attacker_gid: 1,
            skill_id: 0,
            is_last_hit: true,
            is_critical: false,
            hit_index: 0,
        });
        queue.push(ScheduledHit {
            message: DamageMessage::Attacked,
            damage: 200,
            fire_at: 2.0,
            attacker_gid: 1,
            skill_id: 0,
            is_last_hit: true,
            is_critical: false,
            hit_index: 0,
        });
        queue.push(ScheduledHit {
            message: DamageMessage::Attacked,
            damage: 50,
            fire_at: 0.5,
            attacker_gid: 1,
            skill_id: 0,
            is_last_hit: true,
            is_critical: false,
            hit_index: 0,
        });

        let ready = queue.drain_ready(1.0);
        assert_eq!(ready.len(), 2);
        assert_eq!(ready[0].damage, 100);
        assert_eq!(ready[1].damage, 50);
        assert_eq!(queue.events.len(), 1);
        assert_eq!(queue.events[0].damage, 200);
    }

    #[test]
    fn multi_hit_scheduling_distributes_hits() {
        let mut queue = ScheduledHitQueue::new();
        let total_damage = 600;
        let hit_count = 3u16;
        let per_hit = total_damage / hit_count as i32;
        let base_time = 10.0;
        let delay = 0.5;
        let double_attack_term = 0.2;

        for i in 0..hit_count {
            let hit_time = base_time + delay + (i as f32 * double_attack_term);
            queue.push(ScheduledHit {
                message: DamageMessage::AttackedMultiHit { total_damage },
                damage: per_hit,
                fire_at: hit_time,
                attacker_gid: 1,
                skill_id: 10,
                is_last_hit: i == hit_count - 1,
                is_critical: false,
                hit_index: i,
            });
        }

        // At base_time + delay, only first hit fires
        let ready = queue.drain_ready(base_time + delay);
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].damage, 200);
        assert!(!ready[0].is_last_hit);

        // At base_time + delay + 0.2, second hit fires
        let ready = queue.drain_ready(base_time + delay + 0.2);
        assert_eq!(ready.len(), 1);
        assert!(!ready[0].is_last_hit);

        // At base_time + delay + 0.4, last hit fires
        let ready = queue.drain_ready(base_time + delay + 0.4);
        assert_eq!(ready.len(), 1);
        assert!(ready[0].is_last_hit);
        assert!(queue.is_empty());
    }
}
