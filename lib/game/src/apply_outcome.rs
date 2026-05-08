use crate::entity::EntityType;

pub enum ApplyOutcome {
    LoadEntitySprite {
        gid: u32,
        entity_type: EntityType,
        job: u16,
        sex: u8,
        head: u16,
        weapon: u16,
        shield: u16,
        head_top: u16,
        head_mid: u16,
        head_bottom: u16,
        hair_color: u16,
        direction: u8,
    },
    LoadPlayerSprite {
        gid: u32,
        job: u16,
        sex: u8,
        head: u16,
        hair_color: u16,
        cloth_color: u16,
        weapon: u16,
        head_top: u16,
        head_mid: u16,
        head_bottom: u16,
        shield: u16,
    },
    ReloadPlayerSprite {
        gid: u32,
    },
    PreloadItemIcons(Vec<String>),
    LoadMap(String),
    PositionCamera {
        x: f32,
        y: f32,
    },
    EmitDamageNumber {
        target_gid: u32,
        damage: i32,
        damage_type: crate::damage_number::DamageNumberType,
        direction: u8,
    },
    ScheduleCasterReplay {
        src_gid: u32,
        hit_time: f32,
        skill_id: u16,
    },
}
