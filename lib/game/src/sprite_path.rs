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
        _ => "_검",
    }
}

/// Converts a packet weapon view ID to a WeaponType.
/// Returns None for view_id 0 (unarmed) or unknown values.
pub fn weapon_view_id_to_type(view_id: u16) -> Option<WeaponType> {
    match view_id {
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
        17 => Some(WeaponType::Staff2H),
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

pub fn shield_sprite_path(view_id: u16, job_class: u16, sex: u8) -> Option<String> {
    let shield = shield_name_kr(view_id)?;
    let job = job_name_kr(job_class);
    let sex_str = sex_kr(sex);
    Some(format!("data/sprite/방패/{job}/{job}_{sex_str}_{shield}"))
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
        assert_eq!(
            body_sprite_path(7, 1),
            "data/sprite/인간족/몸통/남/기사_남"
        );
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
        assert_eq!(
            head_sprite_path(1, 1),
            "data/sprite/인간족/머리통/남/1_남"
        );
    }

    #[test]
    fn head_female_path() {
        assert_eq!(
            head_sprite_path(3, 0),
            "data/sprite/인간족/머리통/여/3_여"
        );
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
    fn npc_and_monster_sprite_paths() {
        assert_eq!(npc_sprite_path("1_ETC_01"), "data/sprite/npc/1_ETC_01");
        assert_eq!(monster_sprite_path("Poring"), "data/sprite/몬스터/Poring");
    }
}
