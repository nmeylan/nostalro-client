use models::enums::effect_id::EffectId;

#[derive(Clone, Copy, Debug)]
pub struct SprDef {
    pub sprite: &'static str,
    pub size_scale: f32,
    pub anim_speed: f32,
    pub repeat: bool,
    pub tint: [f32; 4],
    pub pos_y: f32,
    pub action: usize,
    pub no_depth: bool,
    pub clip_offset: [i32; 2],
}

impl SprDef {
    const fn new(sprite: &'static str) -> Self {
        Self {
            sprite,
            size_scale: 1.0,
            anim_speed: 4.0,
            repeat: true,
            tint: [1.0, 1.0, 1.0, 1.0],
            pos_y: 0.0,
            action: 0,
            no_depth: false,
            clip_offset: [0, 0],
        }
    }
    const fn with_size(mut self, size_scale: f32) -> Self {
        self.size_scale = size_scale;
        self
    }
    const fn with_anim_speed(mut self, anim_speed: f32) -> Self {
        self.anim_speed = anim_speed;
        self
    }
    const fn one_shot(mut self) -> Self {
        self.repeat = false;
        self
    }
    const fn with_tint(mut self, tint: [f32; 4]) -> Self {
        self.tint = tint;
        self
    }
    const fn with_pos_y(mut self, pos_y: f32) -> Self {
        self.pos_y = pos_y;
        self
    }
    const fn with_action(mut self, action: usize) -> Self {
        self.action = action;
        self
    }
    const fn no_depth(mut self) -> Self {
        self.no_depth = true;
        self
    }
    const fn with_clip_offset(mut self, x: i32, y: i32) -> Self {
        self.clip_offset = [x, y];
        self
    }
}

pub fn spr_def(id: EffectId) -> Option<SprDef> {
    Some(match id {
        EffectId::Torch => {
            SprDef::new(ragnarok_resources::sprite::effect::TORCH_01).with_anim_speed(1.0)
        }
        EffectId::Aqua => SprDef::new(ragnarok_resources::sprite::effect::AQUA_BENEDICTA)
            .with_anim_speed(2.0)
            .one_shot()
            .with_pos_y(-20.0),
        EffectId::Vallentine => SprDef::new(ragnarok_resources::sprite::effect::VALLENTINE)
            .with_anim_speed(2.0)
            .one_shot(),
        EffectId::Vallentine2 => SprDef::new(ragnarok_resources::sprite::effect::VALLENTINE)
            .with_anim_speed(2.0)
            .one_shot()
            .with_action(1),
        EffectId::Itemfast => SprDef::new(ragnarok_resources::sprite::effect::FAST)
            .with_anim_speed(4.0)
            .one_shot(),
        EffectId::Blessing => SprDef::new(ragnarok_resources::sprite::effect::BLESSING).one_shot(),
        EffectId::Demonstration => SprDef::new(ragnarok_resources::sprite::effect::DEMONSTRATION)
            .with_size(1.2)
            .with_pos_y(-1.0),
        EffectId::NpcStop => SprDef::new(ragnarok_resources::sprite::effect::STOP).with_pos_y(-5.0),
        EffectId::NpcStop2 => SprDef::new(ragnarok_resources::sprite::effect::CCONFINE)
            .with_anim_speed(12.0)
            .one_shot()
            .with_tint([1.0, 1.0, 1.0, 100.0 / 255.0]),
        EffectId::Hamicastle => SprDef::new(ragnarok_resources::sprite::effect::CASTLING)
            .with_anim_speed(2.0)
            .one_shot()
            .no_depth(),
        EffectId::ItemThunder => SprDef::new(ragnarok_resources::sprite::effect::ITEM_THUNDER)
            .with_anim_speed(2.0)
            .one_shot()
            .no_depth(),
        EffectId::ItemCloud => SprDef::new(ragnarok_resources::sprite::effect::ITEM_CLOUD)
            .with_anim_speed(2.0)
            .one_shot()
            .no_depth(),
        EffectId::ItemCurse => SprDef::new(ragnarok_resources::sprite::effect::ITEM_CURSE)
            .with_anim_speed(2.0)
            .one_shot()
            .no_depth(),
        EffectId::ItemZzz => SprDef::new(ragnarok_resources::sprite::effect::ITEM_ZZZ)
            .with_anim_speed(2.0)
            .one_shot()
            .no_depth(),
        EffectId::ItemRain => SprDef::new(ragnarok_resources::sprite::effect::ITEM_RAIN)
            .with_anim_speed(2.0)
            .one_shot()
            .no_depth(),
        EffectId::Hamiblood => SprDef::new(ragnarok_resources::sprite::effect::BLOODLUST)
            .with_anim_speed(2.0)
            .one_shot()
            .no_depth(),
        EffectId::Kirikage => SprDef::new(ragnarok_resources::sprite::effect::KIRIKAGE)
            .one_shot()
            .no_depth(),
        EffectId::Tatami => SprDef::new(ragnarok_resources::sprite::effect::TATAMI_FLIP)
            .with_anim_speed(6.0)
            .one_shot()
            .with_pos_y(-6.0)
            .with_clip_offset(45, 0),
        EffectId::Kasumikiri => SprDef::new(ragnarok_resources::sprite::effect::KASUMIKIRI)
            .one_shot()
            .no_depth(),
        EffectId::Issen => SprDef::new(ragnarok_resources::sprite::effect::ISSEN)
            .with_anim_speed(2.0)
            .one_shot()
            .with_pos_y(-6.0)
            .no_depth(),
        EffectId::Kaen => SprDef::new(ragnarok_resources::sprite::effect::KAEN)
            .with_anim_speed(5.0)
            .with_pos_y(-1.0),
        EffectId::Desperado => SprDef::new(ragnarok_resources::sprite::effect::DESPERADO)
            .one_shot()
            .no_depth(),
        EffectId::LightningS => SprDef::new(ragnarok_resources::sprite::item::LIGHTNING_SPEAR)
            .with_anim_speed(2.0)
            .with_pos_y(-1.0),
        EffectId::BlindS => SprDef::new(ragnarok_resources::sprite::item::BLIND_SPEAR)
            .with_anim_speed(2.0)
            .with_pos_y(-1.0),
        EffectId::PoisonS => SprDef::new(ragnarok_resources::sprite::item::POISON_SPEAR)
            .with_anim_speed(2.0)
            .with_pos_y(-1.0),
        EffectId::FreezingS => SprDef::new(ragnarok_resources::sprite::item::FREEZING_SPEAR)
            .with_anim_speed(2.0)
            .with_pos_y(-1.0),
        EffectId::FlareS => SprDef::new(ragnarok_resources::sprite::item::FLARE_SPEAR)
            .with_anim_speed(2.0)
            .with_pos_y(-1.0),
        EffectId::Rapidshower => SprDef::new(ragnarok_resources::sprite::effect::RAPID_SHOWER)
            .one_shot()
            .no_depth(),
        EffectId::Magicalbullet => SprDef::new(ragnarok_resources::sprite::effect::MAGICAL_BULLET)
            .with_anim_speed(2.0)
            .one_shot()
            .no_depth(),
        EffectId::Spreadattack => SprDef::new(ragnarok_resources::sprite::effect::SPREAD_ATTACK)
            .one_shot()
            .no_depth(),
        EffectId::Tracking => SprDef::new(ragnarok_resources::sprite::effect::TRACKING)
            .with_anim_speed(2.0)
            .one_shot()
            .no_depth(),
        EffectId::Tripleaction => SprDef::new(ragnarok_resources::sprite::effect::TRIPLE_ACTION)
            .with_anim_speed(2.0)
            .one_shot()
            .no_depth(),
        EffectId::NpcEarthquake => SprDef::new(ragnarok_resources::sprite::effect::EARTHQUAKE)
            .one_shot()
            .no_depth(),
        EffectId::PokLove => {
            SprDef::new(ragnarok_resources::sprite::effect::FIREWORK_LOVE).one_shot()
        }
        EffectId::PokBirth => {
            SprDef::new(ragnarok_resources::sprite::effect::FIREWORK_BIRTHDAY).one_shot()
        }
        EffectId::PokChristmas => {
            SprDef::new(ragnarok_resources::sprite::effect::FIREWORK_CHRISTMAS).one_shot()
        }
        EffectId::PokWhite => {
            SprDef::new(ragnarok_resources::sprite::effect::FIREWORK_WHITE_DAY).one_shot()
        }
        EffectId::PokValen => {
            SprDef::new(ragnarok_resources::sprite::effect::FIREWORK_VALENTINE).one_shot()
        }
        EffectId::Poisonhit => SprDef::new(ragnarok_resources::sprite::effect::POISONHIT)
            .with_size(1.5)
            .with_anim_speed(2.0)
            .one_shot(),
        EffectId::Darkbreath => SprDef::new(ragnarok_resources::sprite::effect::DARKBREATH)
            .with_size(0.8)
            .with_anim_speed(1.0)
            .with_pos_y(-20.0)
            .with_tint([1.0, 0.0, 0.0, 1.0]),
        EffectId::M01 => SprDef::new(ragnarok_resources::sprite::effect::M_EF01)
            .with_anim_speed(3.0)
            .one_shot()
            .with_tint([1.0, 1.0, 1.0, 220.0 / 255.0])
            .no_depth(),
        EffectId::M03 => SprDef::new(ragnarok_resources::sprite::effect::M_EF03)
            .one_shot()
            .no_depth(),
        EffectId::M04 => SprDef::new(ragnarok_resources::sprite::effect::M_EF04).no_depth(),
        EffectId::M05 => SprDef::new(ragnarok_resources::sprite::effect::M_EF05)
            .one_shot()
            .no_depth(),
        EffectId::M06 => SprDef::new(ragnarok_resources::sprite::effect::M_EF06)
            .one_shot()
            .no_depth(),
        EffectId::M07 => SprDef::new(ragnarok_resources::sprite::effect::M_EF07)
            .one_shot()
            .no_depth(),
        _ => return None,
    })
}
