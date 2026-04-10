/// A known skill with its current state (from server packets).
pub struct SkillData {
    pub id: u16,
    pub name: String,
    pub level: i16,
    pub sp_cost: i16,
    pub attack_range: i16,
    pub upgradable: bool,
    pub skill_type: i32,
}

/// Collection of known skills for a character.
pub struct SkillList {
    skills: Vec<SkillData>,
    open: bool,
}

impl SkillList {
    pub fn new() -> Self {
        Self {
            skills: Vec::new(),
            open: false,
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    pub fn open(&mut self) {
        self.open = true;
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    pub fn clear(&mut self) {
        self.skills.clear();
        self.open = false;
    }

    pub fn set_skills(&mut self, skills: Vec<SkillData>) {
        self.skills = skills;
    }

    pub fn update_skill(
        &mut self,
        id: u16,
        level: i16,
        sp_cost: i16,
        attack_range: i16,
        upgradable: bool,
    ) {
        if let Some(skill) = self.skills.iter_mut().find(|s| s.id == id) {
            skill.level = level;
            skill.sp_cost = sp_cost;
            skill.attack_range = attack_range;
            skill.upgradable = upgradable;
        }
    }

    pub fn add_skill(&mut self, skill: SkillData) {
        if let Some(existing) = self.skills.iter_mut().find(|s| s.id == skill.id) {
            existing.level = skill.level;
            existing.sp_cost = skill.sp_cost;
            existing.attack_range = skill.attack_range;
            existing.upgradable = skill.upgradable;
            existing.skill_type = skill.skill_type;
        } else {
            self.skills.push(skill);
        }
    }

    pub fn get_skill(&self, id: u16) -> Option<&SkillData> {
        self.skills.iter().find(|s| s.id == id)
    }

    pub fn get_skill_by_name(&self, name: &str) -> Option<&SkillData> {
        self.skills.iter().find(|s| s.name == name)
    }

    pub fn skills(&self) -> &[SkillData] {
        &self.skills
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_skill(id: u16, name: &str, level: i16) -> SkillData {
        SkillData {
            id,
            name: name.to_string(),
            level,
            sp_cost: 10,
            attack_range: 1,
            upgradable: true,
            skill_type: 1,
        }
    }

    #[test]
    fn set_and_query_skills() {
        let mut list = SkillList::new();
        list.set_skills(vec![
            make_skill(1, "SM_SWORD", 10),
            make_skill(5, "SM_BASH", 5),
        ]);
        assert_eq!(list.skills().len(), 2);
        assert_eq!(list.get_skill(5).unwrap().level, 5);
        assert_eq!(list.get_skill_by_name("SM_SWORD").unwrap().id, 1);
        assert!(list.get_skill(99).is_none());
    }

    #[test]
    fn update_skill_modifies_existing() {
        let mut list = SkillList::new();
        list.set_skills(vec![make_skill(5, "SM_BASH", 5)]);
        list.update_skill(5, 6, 15, 1, true);
        let skill = list.get_skill(5).unwrap();
        assert_eq!(skill.level, 6);
        assert_eq!(skill.sp_cost, 15);
    }

    #[test]
    fn add_skill_inserts_or_updates() {
        let mut list = SkillList::new();
        list.add_skill(make_skill(5, "SM_BASH", 5));
        assert_eq!(list.skills().len(), 1);
        list.add_skill(make_skill(5, "SM_BASH", 7));
        assert_eq!(list.skills().len(), 1);
        assert_eq!(list.get_skill(5).unwrap().level, 7);
        list.add_skill(make_skill(10, "SM_MAGNUM", 3));
        assert_eq!(list.skills().len(), 2);
    }

    #[test]
    fn toggle_open_close() {
        let mut list = SkillList::new();
        assert!(!list.is_open());
        list.toggle();
        assert!(list.is_open());
        list.close();
        assert!(!list.is_open());
        list.open();
        assert!(list.is_open());
    }
}
