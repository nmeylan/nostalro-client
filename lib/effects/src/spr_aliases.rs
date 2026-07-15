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
}

pub fn spr_def(id: EffectId) -> Option<SprDef> {
    Some(match id {
        EffectId::Torch => SprDef::new("data/sprite/이팩트/torch_01").with_anim_speed(1.0),
        EffectId::Maple => SprDef::new("data/sprite/이팩트/단풍"),
        EffectId::Aqua => SprDef::new("data/sprite/이팩트/성수뜨기")
            .with_anim_speed(2.0)
            .one_shot()
            .with_pos_y(-20.0),
        EffectId::Vallentine => SprDef::new("data/sprite/이팩트/vallentine")
            .with_anim_speed(2.0)
            .one_shot(),
        EffectId::Vallentine2 => SprDef::new("data/sprite/이팩트/vallentine")
            .with_anim_speed(2.0)
            .one_shot()
            .with_action(1),
        EffectId::Itemfast => SprDef::new("data/sprite/이팩트/fast")
            .with_anim_speed(4.0)
            .one_shot(),
        EffectId::Blessing => SprDef::new("data/sprite/이팩트/축복").one_shot(),
        EffectId::Demonstration => SprDef::new("data/sprite/이팩트/데몬스트레이션")
            .with_size(1.2)
            .with_pos_y(-1.0),
        EffectId::NpcStop => SprDef::new("data/sprite/이팩트/스톱").with_pos_y(-5.0),
        EffectId::NpcStop2 => SprDef::new("data/sprite/이팩트/cconfine")
            .with_anim_speed(12.0)
            .one_shot()
            .with_tint([1.0, 1.0, 1.0, 100.0 / 255.0]),
        EffectId::Hamicastle => SprDef::new("data/sprite/이팩트/캐슬링")
            .with_anim_speed(2.0)
            .one_shot(),
        EffectId::ItemThunder => SprDef::new("data/sprite/이팩트/item_thunder")
            .with_anim_speed(2.0)
            .one_shot(),
        EffectId::ItemCloud => SprDef::new("data/sprite/이팩트/item_cloud")
            .with_anim_speed(2.0)
            .one_shot(),
        EffectId::ItemCurse => SprDef::new("data/sprite/이팩트/item_curse")
            .with_anim_speed(2.0)
            .one_shot(),
        EffectId::ItemZzz => SprDef::new("data/sprite/이팩트/item_zzz")
            .with_anim_speed(2.0)
            .one_shot(),
        EffectId::ItemRain => SprDef::new("data/sprite/이팩트/item_rain")
            .with_anim_speed(2.0)
            .one_shot(),
        EffectId::Hamiblood => SprDef::new("data/sprite/이팩트/블러드러스트").one_shot(),
        EffectId::Kirikage => SprDef::new("data/sprite/이팩트/그림자베기").one_shot(),
        EffectId::Tatami => SprDef::new("data/sprite/이팩트/다다미 뒤집기").one_shot(),
        EffectId::Kasumikiri => SprDef::new("data/sprite/이팩트/안개베기").one_shot(),
        EffectId::Issen => SprDef::new("data/sprite/이팩트/일섬").one_shot(),
        EffectId::Kaen => SprDef::new("data/sprite/이팩트/화염진"),
        EffectId::Desperado => SprDef::new("data/sprite/이팩트/데스페라도").one_shot(),
        EffectId::LightningS => SprDef::new("data/sprite/아이템/라이트닝스피어").one_shot(),
        EffectId::BlindS => SprDef::new("data/sprite/아이템/블라인드스피어").one_shot(),
        EffectId::PoisonS => SprDef::new("data/sprite/아이템/포이즌스피어").one_shot(),
        EffectId::FreezingS => SprDef::new("data/sprite/아이템/프리징스피어").one_shot(),
        EffectId::FlareS => SprDef::new("data/sprite/아이템/플레어스피어").one_shot(),
        EffectId::Rapidshower => SprDef::new("data/sprite/이팩트/래피드샤워").one_shot(),
        EffectId::Magicalbullet => SprDef::new("data/sprite/이팩트/매지컬불릿").one_shot(),
        EffectId::Spreadattack => SprDef::new("data/sprite/이팩트/스프레드").one_shot(),
        EffectId::Tracking => SprDef::new("data/sprite/이팩트/트래킹").one_shot(),
        EffectId::Tripleaction => SprDef::new("data/sprite/이팩트/트리플액션").one_shot(),
        EffectId::NpcEarthquake => SprDef::new("data/sprite/이팩트/어스퀘이크").one_shot(),
        EffectId::PokLove => SprDef::new("data/sprite/이팩트/폭죽_러브").one_shot(),
        EffectId::PokBirth => SprDef::new("data/sprite/이팩트/폭죽_생일").one_shot(),
        EffectId::PokChristmas => SprDef::new("data/sprite/이팩트/폭죽_크리스마스").one_shot(),
        EffectId::PokWhite => SprDef::new("data/sprite/이팩트/폭죽_화이트데이").one_shot(),
        EffectId::PokValen => SprDef::new("data/sprite/이팩트/폭죽_발렌타인").one_shot(),
        EffectId::Poisonhit => SprDef::new("data/sprite/이팩트/poisonhit")
            .with_size(1.5)
            .with_anim_speed(2.0)
            .one_shot(),
        EffectId::Darkbreath => SprDef::new("data/sprite/이팩트/darkbreath")
            .with_size(0.8)
            .with_anim_speed(1.0)
            .with_pos_y(-20.0)
            .with_tint([1.0, 0.0, 0.0, 1.0]),
        EffectId::M01 => SprDef::new("data/sprite/이팩트/m_ef01")
            .with_anim_speed(3.0)
            .one_shot()
            .with_tint([1.0, 1.0, 1.0, 220.0 / 255.0]),
        EffectId::M03 => SprDef::new("data/sprite/이팩트/m_ef03").one_shot(),
        EffectId::M04 => SprDef::new("data/sprite/이팩트/m_ef04"),
        EffectId::M05 => SprDef::new("data/sprite/이팩트/m_ef05").one_shot(),
        EffectId::M06 => SprDef::new("data/sprite/이팩트/m_ef06").one_shot(),
        EffectId::M07 => SprDef::new("data/sprite/이팩트/m_ef07").one_shot(),
        _ => return None,
    })
}
