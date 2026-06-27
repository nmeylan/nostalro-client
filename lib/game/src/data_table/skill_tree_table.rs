use std::collections::HashMap;

use ragnarok_formats::grf::GrfArchive;
use ragnarok_formats::lua_table;

const SKILL_TREE_PATH: &str = "data/skilltreeview.txt";

#[derive(Debug, Clone)]
pub struct SkillTreeEntry {
    pub skill_name: String,
    pub position: u8,
    pub max_level: u8,
    pub prerequisite_positions: Vec<u8>,
}

pub struct SkillTreeTable {
    trees: HashMap<u16, Vec<SkillTreeEntry>>,
}

impl SkillTreeTable {
    pub fn from_entries(trees: HashMap<u16, Vec<SkillTreeEntry>>) -> Self {
        Self { trees }
    }

    pub fn load(grf: &GrfArchive) -> Self {
        let data = grf.read_file(SKILL_TREE_PATH).unwrap_or_default();
        let content = lua_table::decode_euc_kr(&data);
        let trees = parse_skill_tree_view(&content);
        tracing::info!("Loaded skill tree table: {} job trees", trees.len());
        Self { trees }
    }

    pub fn get_tree(&self, job_id: u16) -> Option<&[SkillTreeEntry]> {
        self.trees.get(&job_id).map(|v| v.as_slice())
    }

    /// e.g. Knight(7) -> [(0, "Novice"), (1, "Swordsman"), (7, "Knight")]
    pub fn job_skill_tabs(class: u16) -> Vec<(u16, &'static str)> {
        match class {
            0 => vec![(0, "Novice")],
            1 => vec![(0, "Novice"), (1, "Swordsman")],
            2 => vec![(0, "Novice"), (2, "Mage")],
            3 => vec![(0, "Novice"), (3, "Archer")],
            4 => vec![(0, "Novice"), (4, "Acolyte")],
            5 => vec![(0, "Novice"), (5, "Merchant")],
            6 => vec![(0, "Novice"), (6, "Thief")],
            7 => vec![(0, "Novice"), (1, "Swordsman"), (7, "Knight")],
            8 => vec![(0, "Novice"), (2, "Mage"), (8, "Priest")],
            9 => vec![(0, "Novice"), (2, "Mage"), (9, "Wizard")],
            10 => vec![(0, "Novice"), (1, "Swordsman"), (10, "Blacksmith")],
            11 => vec![(0, "Novice"), (3, "Archer"), (11, "Hunter")],
            12 => vec![(0, "Novice"), (6, "Thief"), (12, "Assassin")],
            14 => vec![(0, "Novice"), (1, "Swordsman"), (14, "Crusader")],
            15 => vec![(0, "Novice"), (4, "Acolyte"), (15, "Monk")],
            16 => vec![(0, "Novice"), (2, "Mage"), (16, "Sage")],
            17 => vec![(0, "Novice"), (6, "Thief"), (17, "Rogue")],
            18 => vec![(0, "Novice"), (5, "Merchant"), (18, "Alchemist")],
            19 => vec![(0, "Novice"), (3, "Archer"), (19, "Bard")],
            20 => vec![(0, "Novice"), (3, "Archer"), (20, "Dancer")],
            4008 => vec![(0, "Novice"), (1, "Swordsman"), (4008, "Lord Knight")],
            4009 => vec![(0, "Novice"), (4, "Acolyte"), (4009, "High Priest")],
            4010 => vec![(0, "Novice"), (2, "Mage"), (4010, "High Wizard")],
            4011 => vec![(0, "Novice"), (5, "Merchant"), (4011, "Whitesmith")],
            4012 => vec![(0, "Novice"), (3, "Archer"), (4012, "Sniper")],
            4013 => vec![(0, "Novice"), (6, "Thief"), (4013, "Assassin Cross")],
            4015 => vec![(0, "Novice"), (1, "Swordsman"), (4015, "Paladin")],
            4016 => vec![(0, "Novice"), (4, "Acolyte"), (4016, "Champion")],
            4017 => vec![(0, "Novice"), (2, "Mage"), (4017, "Professor")],
            4018 => vec![(0, "Novice"), (6, "Thief"), (4018, "Stalker")],
            4019 => vec![(0, "Novice"), (5, "Merchant"), (4019, "Creator")],
            4020 => vec![(0, "Novice"), (3, "Archer"), (4020, "Clown")],
            4021 => vec![(0, "Novice"), (3, "Archer"), (4021, "Gypsy")],
            23 => vec![(23, "Super Novice")],
            24 => vec![(24, "Gunslinger")],
            25 => vec![(25, "Ninja")],
            4046 => vec![(4046, "Taekwon")],
            4047 => vec![(4046, "Taekwon"), (4047, "Star Gladiator")],
            4049 => vec![(4046, "Taekwon"), (4049, "Soul Linker")],
            _ => vec![(0, "Novice")],
        }
    }
}

fn parse_skill_tree_view(content: &str) -> HashMap<u16, Vec<SkillTreeEntry>> {
    let mut trees: HashMap<u16, Vec<SkillTreeEntry>> = HashMap::new();
    let mut current_job: Option<u16> = None;
    let mut in_block = false;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }

        if line == "{" {
            in_block = true;
            continue;
        }
        if line == "}" {
            in_block = false;
            current_job = None;
            continue;
        }

        if !in_block {
            if let Ok(job_id) = line.parse::<u16>() {
                current_job = Some(job_id);
            }
            continue;
        }

        if let Some(job_id) = current_job {
            let line = line.trim_end_matches('@');
            let parts: Vec<&str> = line.split('#').collect();
            // Format: SKILL_NAME#POSITION#[PREREQ_POS...]#MAX_LEVEL
            if parts.len() >= 3 {
                let skill_name = parts[0].to_string();
                let position = parts[1].parse::<u8>().unwrap_or(0);
                let max_level = parts[parts.len() - 1].parse::<u8>().unwrap_or(1);

                let mut prerequisite_positions = Vec::new();
                for part in &parts[2..parts.len() - 1] {
                    if let Ok(pos) = part.parse::<u8>() {
                        prerequisite_positions.push(pos);
                    }
                }

                trees.entry(job_id).or_default().push(SkillTreeEntry {
                    skill_name,
                    position,
                    max_level,
                    prerequisite_positions,
                });
            }
        }
    }
    trees
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_swordsman_tree() {
        let content = r#"
1
{
SM_SWORD#1#10@
SM_RECOVERY#2#10@
SM_BASH#3#10@
SM_PROVOKE#4#10@
SM_AUTOBERSERK#5#1@
SM_MOVINGRECOVERY#6#1@
SM_TWOHAND#8#1#10@
SM_MAGNUM#10#3#10@
SM_ENDURE#11#4#10@
SM_FATALBLOW#12#1@
}
"#;
        let trees = parse_skill_tree_view(content);
        let tree = trees.get(&1).unwrap();
        assert_eq!(tree.len(), 10);

        let bash = tree.iter().find(|e| e.skill_name == "SM_BASH").unwrap();
        assert_eq!(bash.position, 3);
        assert_eq!(bash.max_level, 10);
        assert!(bash.prerequisite_positions.is_empty());

        let twohand = tree.iter().find(|e| e.skill_name == "SM_TWOHAND").unwrap();
        assert_eq!(twohand.position, 8);
        assert_eq!(twohand.max_level, 10);
        assert_eq!(twohand.prerequisite_positions, vec![1]);

        let magnum = tree.iter().find(|e| e.skill_name == "SM_MAGNUM").unwrap();
        assert_eq!(magnum.max_level, 10);
        assert_eq!(magnum.prerequisite_positions, vec![3]);
    }

    #[test]
    fn parse_mage_tree_with_multiple_prereqs() {
        let content = r#"
2
{
MG_SAFETYWALL#18#4#11#10@
MG_FIREWALL#19#6#12#10@
}
"#;
        let trees = parse_skill_tree_view(content);
        let tree = trees.get(&2).unwrap();

        let safetywall = tree
            .iter()
            .find(|e| e.skill_name == "MG_SAFETYWALL")
            .unwrap();
        assert_eq!(safetywall.max_level, 10);
        assert_eq!(safetywall.prerequisite_positions, vec![4, 11]);

        let firewall = tree.iter().find(|e| e.skill_name == "MG_FIREWALL").unwrap();
        assert_eq!(firewall.prerequisite_positions, vec![6, 12]);
    }

    #[test]
    fn parse_multiple_jobs() {
        let content = "0\n{\nNV_BASIC#0#9@\n}\n\n1\n{\nSM_SWORD#1#10@\n}\n";
        let trees = parse_skill_tree_view(content);
        assert!(trees.contains_key(&0));
        assert!(trees.contains_key(&1));
        assert_eq!(trees.get(&0).unwrap().len(), 1);
        assert_eq!(trees.get(&1).unwrap().len(), 1);
    }

    #[test]
    fn job_skill_tabs_knight() {
        let tabs = SkillTreeTable::job_skill_tabs(7);
        assert_eq!(tabs.len(), 3);
        assert_eq!(tabs[0], (0, "Novice"));
        assert_eq!(tabs[1], (1, "Swordsman"));
        assert_eq!(tabs[2], (7, "Knight"));
    }

    #[test]
    fn job_skill_tabs_unknown_defaults_to_novice() {
        let tabs = SkillTreeTable::job_skill_tabs(9999);
        assert_eq!(tabs.len(), 1);
        assert_eq!(tabs[0], (0, "Novice"));
    }
}
