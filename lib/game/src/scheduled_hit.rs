#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DamageMessage {
    /// Target flinches and takes damage
    Attacked,
    /// Target takes damage without flinch (magic skills, endure)
    AttackedNoMotion,
    /// Multi-hit: per-hit damage, final hit can trigger total display
    AttackedMultiHit { total_damage: i32 },
}

#[derive(Clone, Copy)]
pub struct ScheduledHit {
    pub message: DamageMessage,
    pub damage: i32,
    pub fire_at: f32,
    pub attacker_gid: u32,
    pub skill_id: u16,
    pub is_last_hit: bool,
    pub is_critical: bool,
    pub hit_index: u16,
    pub attacked_mt_secs: f32,
}

impl ScheduledHit {
    pub fn single(damage: i32, skill_id: u16, is_critical: bool) -> Self {
        Self {
            message: DamageMessage::Attacked,
            damage,
            fire_at: 0.0,
            attacker_gid: 0,
            skill_id,
            is_last_hit: true,
            is_critical,
            hit_index: 0,
            attacked_mt_secs: 0.288,
        }
    }

    pub fn multi_hit(
        damage: i32,
        total_damage: i32,
        skill_id: u16,
        hit_index: u16,
        is_last_hit: bool,
    ) -> Self {
        Self {
            message: DamageMessage::AttackedMultiHit { total_damage },
            damage,
            fire_at: 0.0,
            attacker_gid: 0,
            skill_id,
            is_last_hit,
            is_critical: false,
            hit_index,
            attacked_mt_secs: 0.288,
        }
    }
}

/// Seconds between the blows of a multi-hit swing.
pub const DOUBLE_ATTACK_TERM: f32 = 0.2;

/// One melee/ranged swing as the server described it, before it is spread over
/// the timeline.
pub struct Swing {
    /// Main-hand damage for the whole swing; split evenly across `count`.
    pub damage: i32,
    /// Off-hand damage, `0` unless a dual-wielding player swung.
    pub left_damage: i32,
    pub count: u16,
    pub is_endure: bool,
    pub is_critical: bool,
    pub attacker_gid: u32,
    pub attacked_mt_secs: f32,
    /// When the first blow lands.
    pub fire_at: f32,
}

impl Swing {
    /// The main-hand blows plus, when the attacker dual-wields, a trailing
    /// off-hand blow carrying its own damage value.
    pub fn schedule(&self) -> Vec<ScheduledHit> {
        let count = self.count.max(1);
        let off_hand = self.left_damage != 0;
        let total_damage = self.damage + self.left_damage;
        let per_hit = if count > 1 && self.damage > 0 {
            self.damage / count as i32
        } else {
            self.damage
        };
        let message = if self.is_endure {
            DamageMessage::AttackedNoMotion
        } else if count > 1 {
            DamageMessage::AttackedMultiHit { total_damage }
        } else {
            DamageMessage::Attacked
        };
        let hit = |damage, offset: f32, hit_index, is_last_hit| ScheduledHit {
            message,
            damage,
            fire_at: self.fire_at + offset,
            attacker_gid: self.attacker_gid,
            skill_id: 0,
            is_last_hit,
            is_critical: self.is_critical,
            hit_index,
            attacked_mt_secs: self.attacked_mt_secs,
        };

        let mut hits: Vec<ScheduledHit> = (0..count)
            .map(|i| {
                // An off-hand blow to come bunches the main-hand ones together
                // instead of spacing them a full term apart.
                let offset = match (off_hand, i) {
                    (false, i) => i as f32 * DOUBLE_ATTACK_TERM,
                    (true, 0) => 0.0,
                    (true, _) => DOUBLE_ATTACK_TERM / 2.0,
                };
                hit(per_hit, offset, i, !off_hand && i == count - 1)
            })
            .collect();
        if off_hand {
            hits.push(hit(
                self.left_damage,
                DOUBLE_ATTACK_TERM * 1.75,
                count,
                true,
            ));
        }
        hits
    }
}

pub struct ScheduledHitQueue {
    events: Vec<ScheduledHit>,
}

impl Default for ScheduledHitQueue {
    fn default() -> Self {
        Self::new()
    }
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

    fn swing(damage: i32, left_damage: i32, count: u16) -> Swing {
        Swing {
            damage,
            left_damage,
            count,
            is_endure: false,
            is_critical: false,
            attacker_gid: 1,
            attacked_mt_secs: 0.288,
            fire_at: 10.0,
        }
    }

    #[test]
    fn a_dual_wield_trails_its_off_hand_blow_behind_the_bunched_main_hand() {
        let plain = swing(100, 0, 2).schedule();
        assert_eq!(plain.len(), 2);
        assert_eq!(
            plain.iter().map(|h| h.fire_at).collect::<Vec<_>>(),
            vec![10.0, 10.0 + DOUBLE_ATTACK_TERM]
        );
        assert!(plain[1].is_last_hit);

        let dual = swing(100, 30, 2).schedule();
        assert_eq!(dual.len(), 3);
        assert_eq!(
            dual.iter().map(|h| h.fire_at).collect::<Vec<_>>(),
            vec![
                10.0,
                10.0 + DOUBLE_ATTACK_TERM / 2.0,
                10.0 + DOUBLE_ATTACK_TERM * 1.75,
            ]
        );
        // The main hand splits its own damage; the off-hand blow keeps its value
        // and is the one that closes the burst.
        assert_eq!(
            dual.iter().map(|h| h.damage).collect::<Vec<_>>(),
            vec![50, 50, 30]
        );
        assert!(!dual[0].is_last_hit && !dual[1].is_last_hit && dual[2].is_last_hit);
        assert_eq!(
            dual[2].message,
            DamageMessage::AttackedMultiHit { total_damage: 130 }
        );

        let mut numbers = crate::damage_number::DamageNumberManager::new();
        for hit in &dual {
            numbers.emit(2, 0.0, hit, false, true);
        }
        assert_eq!(numbers.numbers.last().unwrap().value, 130);
    }

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
            attacked_mt_secs: 0.288,
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
            attacked_mt_secs: 0.288,
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
            attacked_mt_secs: 0.288,
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
                attacked_mt_secs: 0.288,
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
