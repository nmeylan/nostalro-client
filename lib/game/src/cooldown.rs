use models::enums::skill_enums::SkillEnum;

pub struct CooldownTracker {
    global_cooldown_end: f32,
    skill_cooldowns: Vec<(SkillEnum, f32)>,
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

    pub fn set_skill_cooldown(&mut self, skill: SkillEnum, duration_secs: f32, now: f32) {
        let end = now + duration_secs;
        if let Some(entry) = self.skill_cooldowns.iter_mut().find(|(s, _)| *s == skill) {
            if end > entry.1 {
                entry.1 = end;
            }
        } else {
            self.skill_cooldowns.push((skill, end));
        }
    }

    pub fn is_on_cooldown(&self, skill: SkillEnum, now: f32) -> bool {
        if now < self.global_cooldown_end {
            return true;
        }
        self.skill_cooldowns
            .iter()
            .any(|(s, end)| *s == skill && now < *end)
    }

    pub fn remaining_secs(&self, skill: SkillEnum, now: f32) -> f32 {
        let global_remaining = (self.global_cooldown_end - now).max(0.0);
        let skill_remaining = self
            .skill_cooldowns
            .iter()
            .find(|(s, _)| *s == skill)
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
        assert!(cd.is_on_cooldown(SkillEnum::NvBasic, 11.0));
        assert!(cd.is_on_cooldown(SkillEnum::WzStormgust, 11.0));
        assert!(!cd.is_on_cooldown(SkillEnum::NvBasic, 12.1));
    }

    #[test]
    fn per_skill_cooldown_does_not_block_other_skills_end_to_end() {
        let mut cd = CooldownTracker::new();
        cd.set_skill_cooldown(SkillEnum::AlHeal, 1.5, 10.0);
        assert!(cd.is_on_cooldown(SkillEnum::AlHeal, 10.5));
        assert!(!cd.is_on_cooldown(SkillEnum::AlDecagi, 10.5));
        assert!((cd.remaining_secs(SkillEnum::AlHeal, 11.0) - 0.5).abs() < f32::EPSILON);
        assert!(!cd.is_on_cooldown(SkillEnum::AlHeal, 11.6));
    }

    #[test]
    fn cooldown_expires_after_duration() {
        let mut cd = CooldownTracker::new();
        cd.set_global_cooldown(1.0, 0.0);
        cd.set_skill_cooldown(SkillEnum::SmBash, 2.0, 0.0);
        assert!(cd.is_on_cooldown(SkillEnum::SmBash, 0.5));
        assert!(!cd.is_on_cooldown(SkillEnum::SmBash, 2.1));
        assert_eq!(cd.remaining_secs(SkillEnum::SmBash, 1.5), 0.5);
    }

    #[test]
    fn set_cooldown_extends_but_does_not_shorten() {
        let mut cd = CooldownTracker::new();
        cd.set_skill_cooldown(SkillEnum::NvBasic, 5.0, 0.0);
        cd.set_skill_cooldown(SkillEnum::NvBasic, 2.0, 0.0);
        assert!(cd.is_on_cooldown(SkillEnum::NvBasic, 4.0));

        cd.set_skill_cooldown(SkillEnum::NvBasic, 10.0, 0.0);
        assert!(cd.is_on_cooldown(SkillEnum::NvBasic, 9.0));
    }

    #[test]
    fn remaining_secs_returns_max_of_global_and_skill() {
        let mut cd = CooldownTracker::new();
        cd.set_global_cooldown(3.0, 0.0);
        cd.set_skill_cooldown(SkillEnum::NvBasic, 5.0, 0.0);
        assert!((cd.remaining_secs(SkillEnum::NvBasic, 1.0) - 4.0).abs() < f32::EPSILON);
        assert!((cd.remaining_secs(SkillEnum::MgFirebolt, 1.0) - 2.0).abs() < f32::EPSILON);
    }
}
