pub struct CooldownTracker {
    global_cooldown_end: f32,
    skill_cooldowns: Vec<(u16, f32)>,
}

impl Default for CooldownTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl CooldownTracker {
    pub fn new() -> Self {
        Self {
            global_cooldown_end: 0.0,
            skill_cooldowns: Vec::new(),
        }
    }

    pub fn set_global_cooldown(&mut self, duration_secs: f32, now: f32) {
        let end = now + duration_secs;
        if end > self.global_cooldown_end {
            self.global_cooldown_end = end;
        }
    }

    pub fn set_skill_cooldown(&mut self, skill_id: u16, duration_secs: f32, now: f32) {
        let end = now + duration_secs;
        if let Some(entry) = self
            .skill_cooldowns
            .iter_mut()
            .find(|(id, _)| *id == skill_id)
        {
            if end > entry.1 {
                entry.1 = end;
            }
        } else {
            self.skill_cooldowns.push((skill_id, end));
        }
    }

    pub fn is_on_cooldown(&self, skill_id: u16, now: f32) -> bool {
        if now < self.global_cooldown_end {
            return true;
        }
        self.skill_cooldowns
            .iter()
            .any(|(id, end)| *id == skill_id && now < *end)
    }

    pub fn remaining_secs(&self, skill_id: u16, now: f32) -> f32 {
        let global_remaining = (self.global_cooldown_end - now).max(0.0);
        let skill_remaining = self
            .skill_cooldowns
            .iter()
            .find(|(id, _)| *id == skill_id)
            .map(|(_, end)| (*end - now).max(0.0))
            .unwrap_or(0.0);
        global_remaining.max(skill_remaining)
    }

    pub fn clear(&mut self) {
        self.global_cooldown_end = 0.0;
        self.skill_cooldowns.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn global_cooldown_blocks_all_skills() {
        let mut cd = CooldownTracker::new();
        cd.set_global_cooldown(2.0, 10.0);
        assert!(cd.is_on_cooldown(1, 11.0));
        assert!(cd.is_on_cooldown(999, 11.0));
        assert!(!cd.is_on_cooldown(1, 12.1));
    }

    #[test]
    fn per_skill_cooldown_does_not_block_other_skills_end_to_end() {
        let mut cd = CooldownTracker::new();
        cd.set_skill_cooldown(28, 1.5, 10.0);
        assert!(cd.is_on_cooldown(28, 10.5));
        assert!(!cd.is_on_cooldown(29, 10.5));
        assert!((cd.remaining_secs(28, 11.0) - 0.5).abs() < f32::EPSILON);
        assert!(!cd.is_on_cooldown(28, 11.6));
    }

    #[test]
    fn cooldown_expires_after_duration() {
        let mut cd = CooldownTracker::new();
        cd.set_global_cooldown(1.0, 0.0);
        cd.set_skill_cooldown(5, 2.0, 0.0);
        assert!(cd.is_on_cooldown(5, 0.5));
        assert!(!cd.is_on_cooldown(5, 2.1));
        assert_eq!(cd.remaining_secs(5, 1.5), 0.5);
    }

    #[test]
    fn set_cooldown_extends_but_does_not_shorten() {
        let mut cd = CooldownTracker::new();
        cd.set_skill_cooldown(1, 5.0, 0.0);
        cd.set_skill_cooldown(1, 2.0, 0.0);
        assert!(cd.is_on_cooldown(1, 4.0));

        cd.set_skill_cooldown(1, 10.0, 0.0);
        assert!(cd.is_on_cooldown(1, 9.0));
    }

    #[test]
    fn remaining_secs_returns_max_of_global_and_skill() {
        let mut cd = CooldownTracker::new();
        cd.set_global_cooldown(3.0, 0.0);
        cd.set_skill_cooldown(1, 5.0, 0.0);
        assert!((cd.remaining_secs(1, 1.0) - 4.0).abs() < f32::EPSILON);
        assert!((cd.remaining_secs(99, 1.0) - 2.0).abs() < f32::EPSILON);
    }
}
