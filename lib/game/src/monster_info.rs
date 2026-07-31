use models::enums::EnumWithNumberValue;
use models::enums::element::Element;
use models::enums::size::Size;

const LABEL: &str = "^FFFF00";
const VALUE: &str = "^FF00FF";

/// Per-element text colours, indexed like [`MonsterInfo::resistances`].
const ELEMENT_COLORS: [&str; 9] = [
    "^3B57FF", "^918033", "^FF3000", "^00FFEA", "^00FF06", "^FFC600", "^CF10D6", "^CACACA",
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
        assert_eq!(lines[0], "^FFFF00Name^FF00FF: Poring");
        assert_eq!(lines[1], "^FFFF00Size^FF00FF: Small");
        assert_eq!(lines[3], "^FFFF00Race^FF00FF: Plant");
        assert_eq!(lines[7], "^FFFF00Element^FF00FF: ^3B57FFLv1 Water");
        assert_eq!(lines[8], "^FFFF00Elemental Damage");
        assert_eq!(
            &lines[9..],
            [
                "^3B57FFWater^FF00FF: 100",
                "^918033Earth^FF00FF: 100",
                "^FF3000Fire^FF00FF: 100",
                "^00FFEAWind^FF00FF: 100",
                "^00FF06Poison^FF00FF: 100",
                "^FFC600Holy^FF00FF: 100",
                "^CF10D6Shadow^FF00FF: 100",
                "^CACACAGhost^FF00FF: 100",
                "^5332FEUndead^FF00FF: 100",
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
        assert_eq!(info.info_lines()[7], "^FFFF00Element^FF00FF: ^FF3000Fire");
    }
}
