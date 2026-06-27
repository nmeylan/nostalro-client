pub use models::enums::weapon::WeaponType;

use crate::entity::EntityType;
use models::enums::class::JobName;
use models::enums::EnumWithNumberValue;
use crate::data_table::name_table::NameTable;

/// View class of the visible warp portal NPC. The original game renders it with
/// no body — its whole visual is the launched warp-zone effect.
pub const JT_WARPNPC: u16 = 45;

pub fn entity_type_from_job(job: u16) -> EntityType {
    match job {
        0..=44 | 4001..=5999 => EntityType::Player,
        JT_WARPNPC => EntityType::Npc,
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
 dd    if job == JT_WARPNPC {
        return None;
    }
    match entity_type_from_job(job) {
        EntityType::Npc => Some(npc_sprite_path(name)),
        EntityType::Monster => Some(monster_sprite_path(name)),
        EntityType::Player => None,
    }
}

pub const OPTION_FALCON: i32 = 0x10;
pub const OPTION_RIDING: i32 = 0x20;

/// `true` while `effect_state` carries the falcon OPTION bit — the hunter/sniper
/// rented falcon companion. Set/cleared server-side (`setoption`) and delivered
/// in the same option-change packet as cart/riding.
pub fn has_falcon(effect_state: i32) -> bool {
    (effect_state & OPTION_FALCON) != 0
}

/// GRF base path (no extension) for the hunter/sniper falcon companion sprite,
/// selected per job like the original game: the base falcon (`매`) for Hunter,
/// the advanced variant (`매2`) for Sniper. The sprites live in the effect
/// directory beside the pushcart.
pub fn falcon_sprite_path(job: u16) -> &'static str {
    match job {
        // Sniper gets the advanced falcon.
        4012 => "data/sprite/이팩트/매2",
        _ => "data/sprite/이팩트/매",
    }
}
pub const OPTION_CART_MASK: i32 = 0x08 | 0x80 | 0x100 | 0x200 | 0x400;
pub const OPTION_REMOVABLE_MASK: i32 = OPTION_FALCON | OPTION_RIDING | OPTION_CART_MASK;

pub const OPTION_HIDE: i32 = 0x02;
pub const OPTION_CLOAK: i32 = 0x04;
pub const OPTION_CHASEWALK: i32 = 0x4000;
pub const OPTION_HIDDEN_MASK: i32 = OPTION_HIDE | OPTION_CLOAK | OPTION_CHASEWALK;

/// Cloak body opacity — the original game's `SetArgb(50, …)`. Faint, distinctly
/// more transparent than Hide so a cloaking unit reads differently.
pub const CLOAK_BODY_ALPHA: f32 = 50.0 / 255.0;
/// Hide / Chase Walk body opacity for the *local* player — the original applies
/// `EF_ACTOR_COLOR` (`SetArgb(100, 255, 255, 255)`: white = no tint, just the
/// reduced alpha). Other units in these states are not drawn at all.
pub const HIDE_BODY_ALPHA: f32 = 100.0 / 255.0;

/// How an actor's body draws while a visibility OPTION is set.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HiddenRender {
    /// Drawn at full opacity (no visibility option active).
    Visible,
    /// Drawn translucent at this alpha; the shadow is dropped.
    Alpha(f32),
    /// Not drawn at all (a hiding/chase-walking unit seen by others).
    Skip,
}

/// `true` while `effect_state` carries Hide, Cloak, or Chase Walk.
pub fn is_hidden(effect_state: i32) -> bool {
    (effect_state & OPTION_HIDDEN_MASK) != 0
}

/// Per-state body visibility, matching the original client: cloak is a faint
/// translucent body for everyone; hide / chase walk keep the body faintly
/// visible only for the local player (`is_self`) and hide it entirely from
/// everyone else. Cloak takes precedence when several bits are set.
pub fn hidden_render(effect_state: i32, is_self: bool) -> HiddenRender {
    if effect_state & OPTION_CLOAK != 0 {
        HiddenRender::Alpha(CLOAK_BODY_ALPHA)
    } else if effect_state & (OPTION_HIDE | OPTION_CHASEWALK) != 0 {
        if is_self {
            HiddenRender::Alpha(HIDE_BODY_ALPHA)
        } else {
            HiddenRender::Skip
        }
    } else {
        HiddenRender::Visible
    }
}

pub fn mounted_job(job: u16) -> Option<u16> {
    match job {
        7 => Some(13),
        14 => Some(21),
        4008 => Some(4014),
        4015 => Some(4022),
        _ => None,
    }
}

pub fn visual_job(job: u16, effect_state: i32) -> u16 {
    if (effect_state & OPTION_RIDING) != 0 {
        mounted_job(job).unwrap_or(job)
    } else {
        job
    }
}

/// The cart design index (1..=5) carried by the cart OPTION bits, or `None`
/// when no cart bit is set. Each bit in `OPTION_CART_MASK` selects one of the
/// five pushcart sprite designs.
pub fn cart_design_from_option(effect_state: i32) -> Option<u8> {
    match effect_state & OPTION_CART_MASK {
        0x08 => Some(1),
        0x80 => Some(2),
        0x100 => Some(3),
        0x200 => Some(4),
        0x400 => Some(5),
        _ => None,
    }
}

/// GRF base path (no extension) for a pushcart sprite of the given design
/// index. The cart sprites live under the effect sprite directory with Korean
/// names: design 1 is the base handcart, higher designs append the variant
/// index, and design 0 is the Super Novice handcart.
pub fn cart_sprite_path(design: u8) -> String {
    const BASE: &str = "data/sprite/이팩트";
    match design {
        0 => format!("{BASE}/슈노손수레"),
        1 => format!("{BASE}/손수레"),
        n => format!("{BASE}/손수레{}", n - 1),
    }
}

pub fn unmounted_job(job: u16) -> Option<u16> {
    match job {
        13 => Some(7),
        21 => Some(14),
        4014 => Some(4008),
        4022 => Some(4015),
        _ => None,
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
        13 => "페코페코_기사",
        14 => "크루세이더",
        15 => "몽크",
        16 => "세이지",
        17 => "로그",
        18 => "연금술사",
        19 => "바드",
        20 => "무희",
        21 => "신페코크루세이더",
        23 => "슈퍼노비스",
        24 => "건너",
        25 => "닌자",
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
        4014 => "로드페코",
        4015 => "팔라딘",
        4016 => "챔피온",
        4017 => "프로페서",
        4018 => "스토커",
        4019 => "크리에이터",
        4020 => "클라운",
        4021 => "집시",
        4022 => "페코팔라딘",
        4046 => "태권소년",
        4047 => "권성",
        4048 => "권성융합",
        4049 => "소울링커",
        _ => "초보자",
    }
}

fn sex_kr(sex: u8) -> &'static str {
    if sex == 0 {
        "여"
    } else {
        "남"
    }
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
    if (1..=4).contains(&id) {
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

/// Numeric path format used by some GRFs (e.g. original game-style)
pub fn shield_sprite_path_numeric(view_id: u16, job_class: u16, sex: u8) -> String {
    let job = job_name_kr(job_class);
    let sex_str = sex_kr(sex);
    format!("data/sprite/방패/{job}/{job}_{sex_str}_{view_id}")
}

pub fn transcendent_to_base_class(job_class: u16) -> Option<JobName> {
    let job = JobName::try_from_value(job_class as usize).ok()?;
    match job {
        JobName::LordKnight => Some(JobName::Knight),
        JobName::HighPriest => Some(JobName::Priest),
        JobName::HighWizard => Some(JobName::Wizard),
        JobName::Whitesmith => Some(JobName::Blacksmith),
        JobName::Sniper => Some(JobName::Hunter),
        JobName::AssassinCross => Some(JobName::Assassin),
        JobName::Paladin => Some(JobName::Crusader),
        JobName::Champion => Some(JobName::Monk),
        JobName::Professor => Some(JobName::Sage),
        JobName::Stalker => Some(JobName::Rogue),
        JobName::Creator => Some(JobName::Alchemist),
        JobName::Clown => Some(JobName::Bard),
        JobName::Gypsy => Some(JobName::Dancer),
        _ => None,
    }
}

pub fn weapon_sprite_path(job_class: u16, sex: u8, weapon_type: WeaponType) -> String {
    let job = job_name_kr(job_class);
    let sex_str = sex_kr(sex);
    let suffix = weapon_suffix(weapon_type);
    format!("data/sprite/인간족/{job}/{job}_{sex_str}{suffix}")
}

/// The per-weapon swing-trail sprite (`검광`, "sword-light") — the weapon sprite
/// path with a `_검광` suffix. Shown during attacks under the Quicken family.
pub fn weapon_trail_sprite_path(job_class: u16, sex: u8, weapon_type: WeaponType) -> String {
    format!("{}_검광", weapon_sprite_path(job_class, sex, weapon_type))
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
    fn hidden_detects_hide_cloak_and_chasewalk() {
        assert!(is_hidden(OPTION_HIDE));
        assert!(is_hidden(OPTION_CLOAK));
        assert!(is_hidden(OPTION_CHASEWALK));
        assert!(is_hidden(OPTION_RIDING | OPTION_CLOAK), "set among other options");
        assert!(!is_hidden(0));
        assert!(!is_hidden(OPTION_RIDING), "mount alone is not hidden");
    }

    #[test]
    fn hidden_render_is_per_state_and_self_aware() {
        use HiddenRender::*;
        // No visibility option: always fully drawn.
        assert_eq!(hidden_render(0, true), Visible);
        assert_eq!(hidden_render(OPTION_RIDING, false), Visible);
        // Cloak: faint for everyone, cloak alpha wins over hide.
        assert_eq!(hidden_render(OPTION_CLOAK, false), Alpha(CLOAK_BODY_ALPHA));
        assert_eq!(hidden_render(OPTION_CLOAK, true), Alpha(CLOAK_BODY_ALPHA));
        assert_eq!(hidden_render(OPTION_CLOAK | OPTION_HIDE, false), Alpha(CLOAK_BODY_ALPHA));
        // Hide / Chase Walk: faintly visible to self, invisible to others.
        assert_eq!(hidden_render(OPTION_HIDE, true), Alpha(HIDE_BODY_ALPHA));
        assert_eq!(hidden_render(OPTION_HIDE, false), Skip);
        assert_eq!(hidden_render(OPTION_CHASEWALK, true), Alpha(HIDE_BODY_ALPHA));
        assert_eq!(hidden_render(OPTION_CHASEWALK, false), Skip);
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

    #[test]
    fn visual_job_with_riding() {
        assert_eq!(visual_job(7, OPTION_RIDING), 13);
        assert_eq!(visual_job(14, OPTION_RIDING), 21);
        assert_eq!(visual_job(4008, OPTION_RIDING), 4014);
        assert_eq!(visual_job(4015, OPTION_RIDING), 4022);
        // Non-mountable class returns original job
        assert_eq!(visual_job(0, OPTION_RIDING), 0);
        assert_eq!(visual_job(12, OPTION_RIDING), 12);
        // No riding flag returns original job
        assert_eq!(visual_job(7, 0), 7);
        assert_eq!(visual_job(14, 0), 14);
        // Other flags don't trigger mount
        assert_eq!(visual_job(7, 0x01), 7);
    }

    #[test]
    fn cart_design_from_option_maps_each_bit() {
        assert_eq!(cart_design_from_option(0), None);
        assert_eq!(cart_design_from_option(OPTION_RIDING), None);
        assert_eq!(cart_design_from_option(0x08), Some(1));
        assert_eq!(cart_design_from_option(0x80), Some(2));
        assert_eq!(cart_design_from_option(0x100), Some(3));
        assert_eq!(cart_design_from_option(0x200), Some(4));
        assert_eq!(cart_design_from_option(0x400), Some(5));
        // Cart bit set among unrelated options still resolves.
        assert_eq!(cart_design_from_option(OPTION_RIDING | 0x100), Some(3));
    }

    #[test]
    fn falcon_bit_and_sprite_path() {
        assert!(has_falcon(OPTION_FALCON));
        assert!(has_falcon(OPTION_FALCON | OPTION_RIDING));
        assert!(!has_falcon(0));
        assert!(!has_falcon(OPTION_RIDING));
        assert_eq!(falcon_sprite_path(11), "data/sprite/이팩트/매");
        assert_eq!(falcon_sprite_path(4012), "data/sprite/이팩트/매2");
    }

    #[test]
    fn mounted_job_sprite_paths() {
        assert_eq!(
            body_sprite_path(13, 1),
            "data/sprite/인간족/몸통/남/페코페코_기사_남"
        );
        assert_eq!(
            body_sprite_path(21, 0),
            "data/sprite/인간족/몸통/여/신페코크루세이더_여"
        );
        assert_eq!(
            body_sprite_path(4014, 1),
            "data/sprite/인간족/몸통/남/로드페코_남"
        );
        assert_eq!(
            body_sprite_path(4022, 1),
            "data/sprite/인간족/몸통/남/페코팔라딘_남"
        );
    }
}
