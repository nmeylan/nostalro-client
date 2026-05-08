pub use models::enums::weapon::WeaponType;

use crate::entity::EntityType;
use crate::name_table::NameTable;

pub fn entity_type_from_job(job: u16) -> EntityType {
    match job {
        0..=44 | 4001..=5999 => EntityType::Player,
        45 => EntityType::Npc,
        46..=999 => EntityType::Npc,
        1000..=3999 => EntityType::Monster,
        _ => EntityType::Monster,
    }
}

pub fn npc_sprite_path(name: &str) -> String {
    format!("data/sprite/npc/{name}")
}

pub fn monster_sprite_path(name: &str) -> String {
    format!("data/sprite/몬스터/{name}")
}

pub fn entity_sprite_base_path(name_table: &NameTable, job: u16) -> Option<String> {
    let name = name_table.get_name(job)?;
    match entity_type_from_job(job) {
        EntityType::Npc => Some(npc_sprite_path(name)),
        EntityType::Monster => Some(monster_sprite_path(name)),
        EntityType::Player => None,
    }
}

fn job_name_kr(job_class: u16) -> &'static str {
    match job_class {
        0 => "초보자",
        1 => "검사",
        2 => "마법사",
        3 => "궁수",
        4 => "성직자",
        5 => "상인",
        6 => "도둑",
        7 => "기사",
        8 => "프리스트",
        9 => "위저드",
        10 => "제철공",
        11 => "헌터",
        12 => "어세신",
        14 => "크루세이더",
        15 => "몽크",
        16 => "세이지",
        17 => "로그",
        18 => "연금술사",
        19 => "바드",
        20 => "무희",
        23 => "슈퍼노비스",
        // Transcendent 1st classes reuse base sprites
        4001 => "초보자",
        4002 => "검사",
        4003 => "마법사",
        4004 => "궁수",
        4005 => "성직자",
        4006 => "상인",
        4007 => "도둑",
        // Transcendent 2nd classes have their own sprites
        4008 => "로드나이트",
        4009 => "하이프리",
        4010 => "하이위저드",
        4011 => "화이트스미스",
        4012 => "스나이퍼",
        4013 => "어쌔신크로스",
        4015 => "팔라딘",
        4016 => "챔피온",
        4017 => "프로페서",
        4018 => "스토커",
        4019 => "크리에이터",
        4020 => "클라운",
        4021 => "집시",
        _ => "초보자",
    }
}

fn sex_kr(sex: u8) -> &'static str {
    if sex == 0 { "여" } else { "남" }
}

/// Returns the GRF path base for a body sprite (without file extension).
/// Example: `data/sprite/인간족/몸통/남/초보자_남`
pub fn body_sprite_path(job_class: u16, sex: u8) -> String {
    let job = job_name_kr(job_class);
    let sex_str = sex_kr(sex);
    format!("data/sprite/인간족/몸통/{sex_str}/{job}_{sex_str}")
}

pub fn head_sprite_path(head_id: u16, sex: u8) -> String {
    let sex_str = sex_kr(sex);
    format!("data/sprite/인간족/머리통/{sex_str}/{head_id}_{sex_str}")
}

pub fn head_palette_path(head_id: u16, sex: u8, palette_id: u16) -> String {
    let sex_str = sex_kr(sex);
    format!("data/palette/머리/머리{head_id}_{sex_str}_{palette_id}.pal")
}

pub fn body_palette_path(job_class: u16, sex: u8, palette_id: u16) -> String {
    let job = job_name_kr(job_class);
    let sex_str = sex_kr(sex);
    format!("data/palette/몸/{job}_{sex_str}_{palette_id}.pal")
}

fn weapon_suffix(weapon_type: WeaponType) -> &'static str {
    match weapon_type {
        WeaponType::Dagger => "_단검",
        WeaponType::Sword1H | WeaponType::Sword2H => "_검",
        WeaponType::Spear1H | WeaponType::Spear2H => "_창",
        WeaponType::Axe1H | WeaponType::Axe2H => "_도끼",
        WeaponType::Mace | WeaponType::Mace2H => "_클럽",
        WeaponType::Staff | WeaponType::Staff2H => "_로드",
        WeaponType::Bow => "_활",
        WeaponType::Knuckle => "_너클",
        WeaponType::Musical => "_악기",
        WeaponType::Whip => "_채찍",
        WeaponType::Book => "_책",
        WeaponType::Katar => "_카타르_카타르",
        WeaponType::DoubleDd => "_단검_단검",
        WeaponType::DoubleSs => "_검_검",
        WeaponType::DoubleAa => "_도끼_도끼",
        WeaponType::DoubleDs => "_단검_검",
        WeaponType::DoubleDa => "_단검_도끼",
        WeaponType::DoubleSa => "_검_도끼",
        _ => "_검",
    }
}

/// Converts a packet weapon value to a WeaponType.
/// The value may be a raw view_id (0–17) or an item_id when the server
/// has no ViewID configured for the weapon.  When an item_id is received
/// it is resolved to a weapon type via standard item_id ranges.
pub fn weapon_view_id_to_type(id: u16) -> Option<WeaponType> {
    dbg!("weapon view", id);
    match id {
        0 => None,
        1 => Some(WeaponType::Dagger),
        2 => Some(WeaponType::Sword1H),
        3 => Some(WeaponType::Sword2H),
        4 => Some(WeaponType::Spear1H),
        5 => Some(WeaponType::Spear2H),
        6 => Some(WeaponType::Axe1H),
        7 => Some(WeaponType::Axe2H),
        8 => Some(WeaponType::Mace),
        9 => Some(WeaponType::Mace2H),
        10 => Some(WeaponType::Staff),
        11 => Some(WeaponType::Bow),
        12 => Some(WeaponType::Knuckle),
        13 => Some(WeaponType::Musical),
        14 => Some(WeaponType::Whip),
        15 => Some(WeaponType::Book),
        16 => Some(WeaponType::Katar),
        23 => Some(WeaponType::Staff2H),
        25 => Some(WeaponType::DoubleDd),
        26 => Some(WeaponType::DoubleSs),
        27 => Some(WeaponType::DoubleAa),
        28 => Some(WeaponType::DoubleDs),
        29 => Some(WeaponType::DoubleDa),
        30 => Some(WeaponType::DoubleSa),
        _ => weapon_type_from_item_id(id),
    }
}

// Fallback when server sends item_id instead of view_id.
fn weapon_type_from_item_id(id: u16) -> Option<WeaponType> {
    if id < 1100 {
        return None;
    }
    if (1116..=1118).contains(&id) {
        return Some(WeaponType::Sword2H);
    }
    if (1314..=1315).contains(&id) {
        return Some(WeaponType::Axe2H);
    }
    if (1410..=1412).contains(&id) {
        return Some(WeaponType::Spear2H);
    }
    if (1472..=1473).contains(&id) {
        return Some(WeaponType::Staff);
    }
    if id == 1599 {
        return Some(WeaponType::Mace);
    }
    match id {
        1100..1150 => Some(WeaponType::Sword1H),
        1150..1200 => Some(WeaponType::Sword2H),
        1200..1250 => Some(WeaponType::Dagger),
        1250..1300 => Some(WeaponType::Katar),
        1300..1350 => Some(WeaponType::Axe1H),
        1350..1400 => Some(WeaponType::Axe2H),
        1400..1450 => Some(WeaponType::Spear1H),
        1450..1500 => Some(WeaponType::Spear2H),
        1500..1550 => Some(WeaponType::Mace),
        1550..1600 => Some(WeaponType::Book),
        1600..1650 => Some(WeaponType::Staff),
        1700..1750 => Some(WeaponType::Bow),
        1800..1850 => Some(WeaponType::Knuckle),
        1900..1950 => Some(WeaponType::Musical),
        1950..2000 => Some(WeaponType::Whip),
        2000..2050 => Some(WeaponType::Staff2H),
        13000..13050 => Some(WeaponType::Dagger),
        _ => None,
    }
}

/// Combines two single-hand weapon types into the dual-wield weapon type.
pub fn dual_wield_type(right: WeaponType, left: WeaponType) -> Option<WeaponType> {
    match (right, left) {
        (WeaponType::Dagger, WeaponType::Dagger) => Some(WeaponType::DoubleDd),
        (WeaponType::Sword1H, WeaponType::Sword1H) => Some(WeaponType::DoubleSs),
        (WeaponType::Axe1H, WeaponType::Axe1H) => Some(WeaponType::DoubleAa),
        (WeaponType::Dagger, WeaponType::Sword1H) | (WeaponType::Sword1H, WeaponType::Dagger) => {
            Some(WeaponType::DoubleDs)
        }
        (WeaponType::Dagger, WeaponType::Axe1H) | (WeaponType::Axe1H, WeaponType::Dagger) => {
            Some(WeaponType::DoubleDa)
        }
        (WeaponType::Sword1H, WeaponType::Axe1H) | (WeaponType::Axe1H, WeaponType::Sword1H) => {
            Some(WeaponType::DoubleSa)
        }
        _ => None,
    }
}

pub fn headgear_sprite_path(suffix: &str, sex: u8) -> String {
    let sex_str = sex_kr(sex);
    format!("data/sprite/악세사리/{sex_str}/{sex_str}{suffix}")
}

fn shield_name_kr(view_id: u16) -> Option<&'static str> {
    match view_id {
        1 => Some("가드"),
        2 => Some("버클러"),
        3 => Some("쉴드"),
        4 => Some("미러쉴드"),
        _ => None,
    }
}

fn shield_view_from_item_id(id: u16) -> Option<u16> {
    match id {
        2101 | 2102 | 2112 | 2116..=2120 => Some(1),
        2103 | 2104 | 2114 | 2126 => Some(2),
        2105 | 2106 | 2113 => Some(3),
        2107 | 2108 | 2110 | 2111 | 2115 | 2127 | 2128 => Some(4),
        _ => None,
    }
}

pub fn resolve_shield_view_id(id: u16) -> u16 {
    if id >= 1 && id <= 4 {
        return id;
    }
    shield_view_from_item_id(id).unwrap_or(id)
}

pub fn shield_sprite_path(view_id: u16, job_class: u16, sex: u8) -> Option<String> {
    let resolved = resolve_shield_view_id(view_id);
    let shield = shield_name_kr(resolved)?;
    let job = job_name_kr(job_class);
    let sex_str = sex_kr(sex);
    Some(format!("data/sprite/방패/{job}/{job}_{sex_str}_{shield}"))
}

/// Numeric path format used by some GRFs (e.g. dhxj-style)
pub fn shield_sprite_path_numeric(view_id: u16, job_class: u16, sex: u8) -> String {
    let job = job_name_kr(job_class);
    let sex_str = sex_kr(sex);
    format!("data/sprite/방패/{job}/{job}_{sex_str}_{view_id}")
}

pub fn weapon_sprite_path(job_class: u16, sex: u8, weapon_type: WeaponType) -> String {
    let job = job_name_kr(job_class);
    let sex_str = sex_kr(sex);
    let suffix = weapon_suffix(weapon_type);
    format!("data/sprite/인간족/{job}/{job}_{sex_str}{suffix}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn novice_male_path() {
        assert_eq!(
            body_sprite_path(0, 1),
            "data/sprite/인간족/몸통/남/초보자_남"
        );
    }

    #[test]
    fn novice_female_path() {
        assert_eq!(
            body_sprite_path(0, 0),
            "data/sprite/인간족/몸통/여/초보자_여"
        );
    }

    #[test]
    fn knight_male_path() {
        assert_eq!(body_sprite_path(7, 1), "data/sprite/인간족/몸통/남/기사_남");
    }

    #[test]
    fn lord_knight_has_own_sprite() {
        assert_eq!(
            body_sprite_path(4008, 1),
            "data/sprite/인간족/몸통/남/로드나이트_남"
        );
    }

    #[test]
    fn clown_has_own_sprite() {
        assert_eq!(
            body_sprite_path(4020, 1),
            "data/sprite/인간족/몸통/남/클라운_남"
        );
    }

    #[test]
    fn high_priest_path() {
        assert_eq!(
            body_sprite_path(4009, 1),
            "data/sprite/인간족/몸통/남/하이프리_남"
        );
    }

    #[test]
    fn unknown_class_falls_back_to_novice() {
        assert_eq!(
            body_sprite_path(9999, 1),
            "data/sprite/인간족/몸통/남/초보자_남"
        );
    }

    #[test]
    fn head_male_path() {
        assert_eq!(head_sprite_path(1, 1), "data/sprite/인간족/머리통/남/1_남");
    }

    #[test]
    fn head_female_path() {
        assert_eq!(head_sprite_path(3, 0), "data/sprite/인간족/머리통/여/3_여");
    }

    #[test]
    fn knight_dagger_weapon_path() {
        assert_eq!(
            weapon_sprite_path(7, 1, WeaponType::Dagger),
            "data/sprite/인간족/기사/기사_남_단검"
        );
    }

    #[test]
    fn novice_sword_weapon_path() {
        assert_eq!(
            weapon_sprite_path(0, 1, WeaponType::Sword1H),
            "data/sprite/인간족/초보자/초보자_남_검"
        );
    }

    #[test]
    fn female_weapon_path() {
        assert_eq!(
            weapon_sprite_path(7, 0, WeaponType::Spear1H),
            "data/sprite/인간족/기사/기사_여_창"
        );
    }

    #[test]
    fn weapon_view_id_zero_is_none() {
        assert!(weapon_view_id_to_type(0).is_none());
    }

    #[test]
    fn weapon_view_id_maps_correctly() {
        assert_eq!(weapon_view_id_to_type(1), Some(WeaponType::Dagger));
        assert_eq!(weapon_view_id_to_type(2), Some(WeaponType::Sword1H));
        assert_eq!(weapon_view_id_to_type(11), Some(WeaponType::Bow));
        assert_eq!(weapon_view_id_to_type(16), Some(WeaponType::Katar));
        assert_eq!(weapon_view_id_to_type(25), Some(WeaponType::DoubleDd));
        assert_eq!(weapon_view_id_to_type(30), Some(WeaponType::DoubleSa));
    }

    #[test]
    fn weapon_item_id_fallback() {
        assert_eq!(weapon_view_id_to_type(1101), Some(WeaponType::Sword1H));
        assert_eq!(weapon_view_id_to_type(1201), Some(WeaponType::Dagger));
        assert_eq!(weapon_view_id_to_type(1701), Some(WeaponType::Bow));
        assert_eq!(weapon_view_id_to_type(1250), Some(WeaponType::Katar));
        assert_eq!(weapon_view_id_to_type(1450), Some(WeaponType::Spear2H));
        assert_eq!(weapon_view_id_to_type(1116), Some(WeaponType::Sword2H));
        assert!(weapon_view_id_to_type(999).is_none());
    }

    #[test]
    fn entity_type_from_job_boundaries() {
        use crate::entity::EntityType;
        assert_eq!(entity_type_from_job(0), EntityType::Player);
        assert_eq!(entity_type_from_job(44), EntityType::Player);
        assert_eq!(entity_type_from_job(45), EntityType::Npc);
        assert_eq!(entity_type_from_job(46), EntityType::Npc);
        assert_eq!(entity_type_from_job(999), EntityType::Npc);
        assert_eq!(entity_type_from_job(1000), EntityType::Monster);
        assert_eq!(entity_type_from_job(1002), EntityType::Monster);
        assert_eq!(entity_type_from_job(3999), EntityType::Monster);
        assert_eq!(entity_type_from_job(4001), EntityType::Player);
        assert_eq!(entity_type_from_job(5999), EntityType::Player);
    }

    #[test]
    fn body_palette_path_formats_correctly() {
        assert_eq!(body_palette_path(1, 1, 3), "data/palette/몸/검사_남_3.pal");
        assert_eq!(
            body_palette_path(0, 0, 1),
            "data/palette/몸/초보자_여_1.pal"
        );
    }

    #[test]
    fn head_palette_path_formats_correctly() {
        assert_eq!(
            head_palette_path(1, 1, 3),
            "data/palette/머리/머리1_남_3.pal"
        );
        assert_eq!(
            head_palette_path(5, 0, 7),
            "data/palette/머리/머리5_여_7.pal"
        );
    }

    #[test]
    fn npc_and_monster_sprite_paths() {
        assert_eq!(npc_sprite_path("1_ETC_01"), "data/sprite/npc/1_ETC_01");
        assert_eq!(monster_sprite_path("Poring"), "data/sprite/몬스터/Poring");
    }

    #[test]
    fn shield_view_id_resolution() {
        assert_eq!(resolve_shield_view_id(1), 1);
        assert_eq!(resolve_shield_view_id(2), 2);
        assert_eq!(resolve_shield_view_id(4), 4);
        assert_eq!(resolve_shield_view_id(2101), 1); // Guard
        assert_eq!(resolve_shield_view_id(2103), 2); // Buckler
        assert_eq!(resolve_shield_view_id(2105), 3); // Shield
        assert_eq!(resolve_shield_view_id(2107), 4); // Mirror Shield
    }

    #[test]
    fn shield_sprite_path_with_item_id() {
        let path = shield_sprite_path(2103, 12, 1);
        assert_eq!(path.unwrap(), "data/sprite/방패/어세신/어세신_남_버클러");
    }
}
