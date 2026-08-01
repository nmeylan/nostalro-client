use models::enums::EnumWithNumberValue;
use models::enums::element::Element;
use models::enums::size::Size;

const LABEL: &str = "^243361";
const VALUE: &str = "^000000";

/// Per-element text colours, indexed like [`MonsterInfo::resistances`].
const ELEMENT_COLORS: [&str; 9] = [
    "^3B57FF", "^6E6231", "^D62800", "^007D73", "^008203", "^9C6C00", "^9B0BA1", "^6F6F6F",
    "^5332FE",
];

const RESISTANCE_LABELS: [&str; 9] = [
    "Water", "Earth", "Fire", "Wind", "Poison", "Holy", "Shadow", "Ghost", "Undead",
];

/// Wire order of `race` in ZC_MONSTER_INFO. It is the server's `e_race`, which
/// is NOT the ordering of `models::enums::mob::MobRace`.
const RACE_NAMES: [&str; 10] = [
    "Formless",
    "Undead",
    "Brute",
    "Plant",
    "Insect",
    "Fish",
    "Demon",
    "Demi-Human",
    "Angel",
    "Dragon",
];

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MonsterInfo {
    pub name: String,
    pub job: u16,
    pub level: i16,
    pub size: i16,
    pub hp: i32,
    pub def: i16,
    pub race: i16,
    pub mdef: i16,
    /// `element_level * 20 + element`, though a server may send a bare element.
    pub property: i16,
    /// water, earth, fire, wind, poison, holy, shadow, ghost, undead
    pub resistances: [u8; 9],
}

impl MonsterInfo {
    pub fn size_name(&self) -> String {
        match self.size {
            0..=2 => format!("{:?}", Size::from_value(self.size as usize)),
            other => format!("{other}"),
        }
    }

    pub fn race_name(&self) -> String {
        RACE_NAMES
            .get(self.race.max(0) as usize)
            .map(|n| n.to_string())
            .unwrap_or_else(|| format!("{}", self.race))
    }

    pub fn element_index(&self) -> usize {
        (self.property.max(0) % 20) as usize
    }

    pub fn element_level(&self) -> i16 {
        (self.property.max(0) / 20).min(5)
    }

    pub fn element_name(&self) -> String {
        let index = self.element_index();
        if index > Element::Undead.value() {
            return format!("{index}");
        }
        format!("{}", Element::from_value(index))
    }

    /// One display line per entry, carrying `^RRGGBB` colour codes.
    pub fn info_lines(&self) -> Vec<String> {
        let element_color = ELEMENT_COLORS
            .get(self.element_index().wrapping_sub(1))
            .copied()
            .unwrap_or(VALUE);
        let element_level = self.element_level();
        let element = if element_level >= 1 {
            format!("Lv{element_level} {}", self.element_name())
        } else {
            self.element_name()
        };

        let mut lines = vec![
            row("Name", &self.name),
            row("Size", &self.size_name()),
            row("Level", &self.level.to_string()),
            row("Race", &self.race_name()),
            row("HP", &self.hp.to_string()),
            row("DEF", &self.def.to_string()),
            row("MDEF", &self.mdef.to_string()),
            format!("{LABEL}Element{VALUE}: {element_color}{element}"),
            format!("{LABEL}Elemental Damage"),
        ];
        for (i, value) in self.resistances.iter().enumerate() {
            lines.push(format!(
                "{}{}{VALUE}: {value}",
                ELEMENT_COLORS[i], RESISTANCE_LABELS[i]
            ));
        }
        lines
    }
}

fn row(label: &str, value: &str) -> String {
    format!("{LABEL}{label}{VALUE}: {value}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_labelled_rows_and_nine_coloured_resistances() {
        let info = MonsterInfo {
            name: "Poring".to_string(),
            job: 1002,
            level: 1,
            size: 0,
            hp: 50,
            def: 0,
            race: 3,
            mdef: 5,
            property: 21,
            resistances: [100, 100, 100, 100, 100, 100, 100, 100, 100],
        };
        let lines = info.info_lines();

        assert_eq!(lines.len(), 18);
        assert_eq!(lines[0], "^243361Name^000000: Poring");
        assert_eq!(lines[1], "^243361Size^000000: Small");
        assert_eq!(lines[3], "^243361Race^000000: Plant");
        assert_eq!(lines[7], "^243361Element^000000: ^3B57FFLv1 Water");
        assert_eq!(lines[8], "^243361Elemental Damage");
        assert_eq!(
            &lines[9..],
            [
                "^3B57FFWater^000000: 100",
                "^6E6231Earth^000000: 100",
                "^D62800Fire^000000: 100",
                "^007D73Wind^000000: 100",
                "^008203Poison^000000: 100",
                "^9C6C00Holy^000000: 100",
                "^9B0BA1Shadow^000000: 100",
                "^6F6F6FGhost^000000: 100",
                "^5332FEUndead^000000: 100",
            ]
        );
    }

    #[test]
    fn bare_element_without_level_drops_the_level_prefix() {
        let info = MonsterInfo {
            property: 3,
            ..Default::default()
        };
        assert_eq!(info.element_level(), 0);
        assert_eq!(info.info_lines()[7], "^243361Element^000000: ^D62800Fire");
    }
}
