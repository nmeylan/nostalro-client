//! Every GRF resource path the client reads, in one place.
//!
//! Values are GRF entry names: forward slashes, Korean folder names spelled the
//! way the archive stores them. Paths built at runtime are functions here too,
//! so no `data/` literal needs to live anywhere else.

/// Directory prefixes, for listing or stripping — not resources themselves.
pub mod dir {
    pub const DATA: &str = "data/";
    pub const SPRITE_ACCESSORY: &str = "data/sprite/악세사리/";
    pub const SPRITE_MONSTER: &str = "data/sprite/몬스터/";
    pub const SPRITE_NPC: &str = "data/sprite/npc/";
    pub const SPRITE_PLAYER: &str = "data/sprite/인간족/";
    pub const SPRITE_SHIELD: &str = "data/sprite/방패/";
    pub const STR_EFFECT: &str = "data/texture/effect/";
}

/// TrueType fonts shipped in the archive.
pub mod font {
    pub const NANUM_BARUN_GOTHIC: &str = "data/Font/NanumBarunGothic.ttf";
    pub const NANUM_BARUN_GOTHIC_BOLD: &str = "data/Font/NanumBarunGothicBold.ttf";
}

/// GRF archives.
pub mod grf {
    pub const DEFAULT_ARCHIVE: &str = "data/data.grf";
}

/// Head-attachment offset tables, one per job.
pub mod imf {
    pub fn of(base: &str) -> String {
        format!("data/imf/{base}.imf")
    }

    pub fn for_job(job: &str, sex: &str) -> String {
        format!("data/imf/{job}_{sex}.imf")
    }
}

/// Lua/Lub identity tables. `.lua` and `.lub` are alternate encodings of
/// the same table; readers try one then the other.
pub mod lua {
    pub const ACCESSORY_ID_LUA: &str = "data/lua files/datainfo/accessoryid.lua";
    pub const ACCESSORY_ID_LUB: &str = "data/lua files/datainfo/accessoryid.lub";
    pub const ACCESSORY_NAME_LUA: &str = "data/lua files/datainfo/accname.lua";
    pub const ACCESSORY_NAME_LUB: &str = "data/lua files/datainfo/accname.lub";
    pub const JOB_IDENTITY_514_LUB: &str = "data/luafiles514/lua files/datainfo/jobidentity.lub";
    pub const JOB_IDENTITY_LUA: &str = "data/lua files/datainfo/jobidentity.lua";
    pub const JOB_IDENTITY_LUB: &str = "data/lua files/datainfo/jobidentity.lub";
    pub const NPC_IDENTITY_514_LUB: &str = "data/luafiles514/lua files/datainfo/npcidentity.lub";
    pub const NPC_IDENTITY_LUA: &str = "data/lua files/datainfo/npcidentity.lua";
    pub const NPC_IDENTITY_LUB: &str = "data/lua files/datainfo/npcidentity.lub";
}

/// Map geometry: one `.rsw`/`.gnd`/`.gat` triple per map.
pub mod map {
    pub fn rsw(map_name: &str) -> String {
        format!("data/{map_name}.rsw")
    }

    pub fn gnd(map_name: &str) -> String {
        format!("data/{map_name}.gnd")
    }

    pub fn gat(map_name: &str) -> String {
        format!("data/{map_name}.gat")
    }

    /// For names already carrying their extension, as RSW headers store them.
    pub fn file(file_name: &str) -> String {
        format!("data/{file_name}")
    }
}

/// 3D monster models and their animation clips.
pub mod model {
    pub fn mob(file_name: &str) -> String {
        format!("data/model/3dmob/{file_name}")
    }

    pub fn mob_animation(bone_type: u32, suffix: &str) -> String {
        format!("data/model/3dmob_bone/{bone_type}_{suffix}.gr2")
    }
}

/// Sprite recolour palettes.
pub mod palette {
    pub fn head(head_id: u16, sex: &str, palette_id: u16) -> String {
        format!("data/palette/머리/머리{head_id}_{sex}_{palette_id}.pal")
    }

    pub fn body(job: &str, sex: &str, palette_id: u16) -> String {
        format!("data/palette/몸/{job}_{sex}_{palette_id}.pal")
    }
}

/// BGM tracks and sound effects.
pub mod sound {
    pub fn bgm(track: &str) -> String {
        format!("data/wav/bgm/{track}")
    }

    pub fn sfx(name: &str) -> String {
        format!("data/wav/{name}")
    }
}

/// Sprites outside the actor trees.
pub mod sprite {
    pub const CURSORS_ACT: &str = "data/sprite/cursors.act";
    pub const CURSORS_SPR: &str = "data/sprite/cursors.spr";
    pub const SHADOW: &str = "data/sprite/shadow";
    pub const SHADOW_ACT: &str = "data/sprite/shadow.act";
    pub const SHADOW_SPR: &str = "data/sprite/shadow.spr";
    pub const SLOTMACHINE_ACT: &str = "data/sprite/slotmachine.act";
    pub const SLOTMACHINE_SPR: &str = "data/sprite/slotmachine.spr";

    /// Headgear sprites, drawn per sex.
    pub mod accessory {
        pub fn of(sex: &str, suffix: &str) -> String {
            format!("data/sprite/악세사리/{sex}/{sex}{suffix}")
        }
    }

    /// The `이팩트` tree — visual-effect sprites.
    pub mod effect {
        pub const ANGEL: &str = "data/sprite/이팩트/천사";
        pub const ANGEL_FEATHER: &str = "data/sprite/이팩트/천사날개깃털";
        pub const ANGEL_WINGS: &str = "data/sprite/이팩트/천사날개";
        pub const AQUA_BENEDICTA: &str = "data/sprite/이팩트/성수뜨기";
        pub const BAT: &str = "data/sprite/이팩트/박쥐";
        pub const BLESSING: &str = "data/sprite/이팩트/축복";
        pub const BLOODLUST: &str = "data/sprite/이팩트/블러드러스트";
        pub const CASTLING: &str = "data/sprite/이팩트/캐슬링";
        pub const CCONFINE: &str = "data/sprite/이팩트/cconfine";
        pub const CHIMNEY_SMOKE: &str = "data/sprite/이팩트/굴뚝연기";
        pub const CHRISTMAS: &str = "data/sprite/이팩트/크리스마스";
        pub const DARKBREATH: &str = "data/sprite/이팩트/darkbreath";
        pub const DEMONSTRATION: &str = "data/sprite/이팩트/데몬스트레이션";
        pub const DESPERADO: &str = "data/sprite/이팩트/데스페라도";
        pub const EARTHQUAKE: &str = "data/sprite/이팩트/어스퀘이크";
        pub const EF_SNOW: &str = "data/sprite/이팩트/ef_snow";
        pub const EMOTION_ACT: &str = "data/sprite/이팩트/emotion.act";
        pub const EMOTION_SPR: &str = "data/sprite/이팩트/emotion.spr";
        pub const FALCON: &str = "data/sprite/이팩트/매";
        pub const FALCON2: &str = "data/sprite/이팩트/매2";
        pub const FAST: &str = "data/sprite/이팩트/fast";
        pub const FIREBALL: &str = "data/sprite/이팩트/fireball";
        pub const FIREWORK_BIRTHDAY: &str = "data/sprite/이팩트/폭죽_생일";
        pub const FIREWORK_CHRISTMAS: &str = "data/sprite/이팩트/폭죽_크리스마스";
        pub const FIREWORK_LOVE: &str = "data/sprite/이팩트/폭죽_러브";
        pub const FIREWORK_VALENTINE: &str = "data/sprite/이팩트/폭죽_발렌타인";
        pub const FIREWORK_WHITE_DAY: &str = "data/sprite/이팩트/폭죽_화이트데이";
        pub const FVOICE: &str = "data/sprite/이팩트/fvoice";
        pub const GHOST: &str = "data/sprite/이팩트/유령";
        pub const HANBOK_ANGEL_BODY: &str = "data/sprite/이팩트/한복천사(본체)";
        pub const HANBOK_ANGEL_WINGS: &str = "data/sprite/이팩트/한복천사(날개)";
        pub const ICE_BLOCK: &str = "data/sprite/이팩트/얼음땡";
        pub const ISSEN: &str = "data/sprite/이팩트/일섬";
        pub const ITEM_CLOUD: &str = "data/sprite/이팩트/item_cloud";
        pub const ITEM_CURSE: &str = "data/sprite/이팩트/item_curse";
        pub const ITEM_RAIN: &str = "data/sprite/이팩트/item_rain";
        pub const ITEM_THUNDER: &str = "data/sprite/이팩트/item_thunder";
        pub const ITEM_ZZZ: &str = "data/sprite/이팩트/item_zzz";
        pub const KAEN: &str = "data/sprite/이팩트/화염진";
        pub const KASUMIKIRI: &str = "data/sprite/이팩트/안개베기";
        pub const KIRIKAGE: &str = "data/sprite/이팩트/그림자베기";
        pub const MAGICAL_BULLET: &str = "data/sprite/이팩트/매지컬불릿";
        pub const MAPLE_LEAF: &str = "data/sprite/이팩트/단풍";
        pub const MSG_ACT: &str = "data/sprite/이팩트/msg.act";
        pub const MSG_SPR: &str = "data/sprite/이팩트/msg.spr";
        pub const M_EF01: &str = "data/sprite/이팩트/m_ef01";
        pub const M_EF02: &str = "data/sprite/이팩트/m_ef02";
        pub const M_EF03: &str = "data/sprite/이팩트/m_ef03";
        pub const M_EF04: &str = "data/sprite/이팩트/m_ef04";
        pub const M_EF05: &str = "data/sprite/이팩트/m_ef05";
        pub const M_EF06: &str = "data/sprite/이팩트/m_ef06";
        pub const M_EF07: &str = "data/sprite/이팩트/m_ef07";
        pub const NUMBER_ACT: &str = "data/sprite/이팩트/숫자.act";
        pub const NUMBER_SPR: &str = "data/sprite/이팩트/숫자.spr";
        pub const ORCFACE: &str = "data/sprite/이팩트/orcface";
        pub const PARTICLE1: &str = "data/sprite/이팩트/particle1";
        pub const PARTICLE2: &str = "data/sprite/이팩트/particle2";
        pub const PARTICLE3: &str = "data/sprite/이팩트/particle3";
        pub const PARTICLE4: &str = "data/sprite/이팩트/particle4";
        pub const PARTICLE5: &str = "data/sprite/이팩트/particle5";
        pub const PARTICLE6: &str = "data/sprite/이팩트/particle6";
        pub const PARTICLE7: &str = "data/sprite/이팩트/particle7";
        pub const POISONHIT: &str = "data/sprite/이팩트/poisonhit";
        pub const RANKFONT_ACT: &str = "data/sprite/이팩트/rankfont.act";
        pub const RANKFONT_SPR: &str = "data/sprite/이팩트/rankfont.spr";
        pub const RAPID_SHOWER: &str = "data/sprite/이팩트/래피드샤워";
        pub const SAKURA01: &str = "data/sprite/이팩트/sakura01";
        pub const SIGHT: &str = "data/sprite/이팩트/sight";
        pub const SMOKE: &str = "data/sprite/이팩트/smoke";
        pub const SPEAR: &str = "data/sprite/이팩트/창";
        pub const SPREAD_ATTACK: &str = "data/sprite/이팩트/스프레드";
        pub const STATUS_CURSE: &str = "data/sprite/이팩트/status-curse";
        pub const STATUS_SLEEP: &str = "data/sprite/이팩트/status-sleep";
        pub const STATUS_STUN: &str = "data/sprite/이팩트/status-stun";
        pub const STOP: &str = "data/sprite/이팩트/스톱";
        pub const TATAMI_FLIP: &str = "data/sprite/이팩트/다다미 뒤집기";
        pub const TIMEFONT_ACT: &str = "data/sprite/이팩트/timefont.act";
        pub const TIMEFONT_SPR: &str = "data/sprite/이팩트/timefont.spr";
        pub const TORCH_01: &str = "data/sprite/이팩트/torch_01";
        pub const TRACKING: &str = "data/sprite/이팩트/트래킹";
        pub const TRIPLE_ACTION: &str = "data/sprite/이팩트/트리플액션";
        pub const VALLENTINE: &str = "data/sprite/이팩트/vallentine";
        pub const WATERBALL: &str = "data/sprite/이팩트/waterball";
        pub const WINK: &str = "data/sprite/이팩트/wink";

        /// Design 0 is the Merchant's own cart, 1.. the purchasable ones.
        pub fn cart(design: u8) -> String {
            match design {
                0 => "data/sprite/이팩트/슈노손수레".to_string(),
                1 => "data/sprite/이팩트/손수레".to_string(),
                n => format!("data/sprite/이팩트/손수레{}", n - 1),
            }
        }
    }

    /// Homunculus sprites.
    pub mod homun {
        pub fn of(name: &str) -> String {
            format!("data/sprite/homun/{name}")
        }
    }

    /// Dropped-item sprites.
    pub mod item {
        pub const BLIND_SPEAR: &str = "data/sprite/아이템/블라인드스피어";
        pub const BLUE_GEMSTONE: &str = "data/sprite/아이템/블루젬스톤";
        pub const FLARE_SPEAR: &str = "data/sprite/아이템/플레어스피어";
        pub const FREEZING_SPEAR: &str = "data/sprite/아이템/프리징스피어";
        pub const LIGHTNING_SPEAR: &str = "data/sprite/아이템/라이트닝스피어";
        pub const POISON_SPEAR: &str = "data/sprite/아이템/포이즌스피어";
        pub const RED_GEMSTONE: &str = "data/sprite/아이템/레드젬스톤";
        pub const YELLOW_GEMSTONE: &str = "data/sprite/아이템/옐로우젬스톤";

        pub fn of(name: &str) -> String {
            format!("data/sprite/아이템/{name}")
        }
    }

    /// Monster sprites.
    pub mod monster {
        pub const SKEL_ARCHER_ARROW: &str = "data/sprite/몬스터/skel_archer_arrow";

        pub fn of(name: &str) -> String {
            format!("data/sprite/몬스터/{name}")
        }
    }

    /// NPC sprites.
    pub mod npc {
        pub fn of(name: &str) -> String {
            format!("data/sprite/npc/{name}")
        }
    }

    /// Pet headgear animations, keyed by the pet mob.
    pub mod pet_accessory {
        pub const BACSOJIN: &str = "data/sprite/몬스터/BACSOJIN_동그란머리장식.act";
        pub const BAPHOMET: &str = "data/sprite/몬스터/BAPHOMET_뼉다구모자.act";
        pub const BON_GUN: &str = "data/sprite/몬스터/bon_gun_영환도사검.act";
        pub const CHOCHO: &str = "data/sprite/몬스터/chocho_방독면.act";
        pub const CIVIL_SERVANT: &str = "data/sprite/몬스터/CIVIL_SERVANT_금빛귀걸이.act";
        pub const DESERT_WOLF_B: &str = "data/sprite/몬스터/DESERT_WOLF_B_우주복머리.act";
        pub const DEVIRUCHI: &str = "data/sprite/몬스터/DEVIRUCHI_젖꼭지.act";
        pub const DOKEBI: &str = "data/sprite/몬스터/DOKEBI_아후로머리.act";
        pub const DULLAHAN: &str = "data/sprite/몬스터/DULLAHAN_죽음의고리.act";
        pub const GOBLIN_LEADER: &str = "data/sprite/몬스터/GOBLIN_LEADER_멋진휘장.act";
        pub const GOLEM: &str = "data/sprite/몬스터/GOLEM_태엽.act";
        pub const IMP: &str = "data/sprite/몬스터/IMP_뿔보호대.act";
        pub const INCUBUS: &str = "data/sprite/몬스터/INCUBUS_무도회가면.act";
        pub const ISIS: &str = "data/sprite/몬스터/isis_클레오파트라머리띠.act";
        pub const LEAF_CAT: &str = "data/sprite/몬스터/LEAF_CAT_초록복주머니.act";
        pub const LOLI_RURI: &str = "data/sprite/몬스터/LOLI_RURI_패션안경.act";
        pub const LUNATIC: &str = "data/sprite/몬스터/lunatic_리본.act";
        pub const MARIONETTE: &str = "data/sprite/몬스터/MARIONETTE_별모양머리띠.act";
        pub const MEDUSA: &str = "data/sprite/몬스터/MEDUSA_여왕의코로넷.act";
        pub const MIYABI_NINGYO: &str = "data/sprite/몬스터/MIYABI_NINGYO_여름부채.act";
        pub const MUNAK: &str = "data/sprite/몬스터/munak_요술봉.act";
        pub const NIGHTMARE_TERROR: &str = "data/sprite/몬스터/NIGHTMARE_TERROR_지옥의뿔.act";
        pub const ORK_WARRIOR: &str = "data/sprite/몬스터/ork_warrior_꽃.act";
        pub const PECOPECO: &str = "data/sprite/몬스터/pecopeco_냄비.act";
        pub const PETIT: &str = "data/sprite/몬스터/PETIT_별.act";
        pub const PICKY: &str = "data/sprite/몬스터/picky_알껍질.act";
        pub const PORING: &str = "data/sprite/몬스터/poring_책가방.act";
        pub const ROCKER: &str = "data/sprite/몬스터/rocker_메뚜기안경.act";
        pub const SAVAGE_BABE: &str = "data/sprite/몬스터/savage_babe_레이스.act";
        pub const SHINOBI: &str = "data/sprite/몬스터/SHINOBI_두루마기용술.act";
        pub const SMOKIE: &str = "data/sprite/몬스터/smokie_머플러.act";
        pub const SOHEE: &str = "data/sprite/몬스터/SOHEE_방울.act";
        pub const SPORE: &str = "data/sprite/몬스터/spore_원주민치마.act";
        pub const STONE_SHOOTER: &str = "data/sprite/몬스터/STONE_SHOOTER_아프로헤어.act";
        pub const SUCCUBUS: &str = "data/sprite/몬스터/SUCCUBUS_검은나비가면.act";
        pub const WHISPER: &str = "data/sprite/몬스터/WHISPER_영혼고리_.act";
        pub const WICKED_NYMPH: &str = "data/sprite/몬스터/WICKED_NYMPH_옥노리개.act";
        pub const YOYO: &str = "data/sprite/몬스터/yoyo_머리띠.act";
    }

    /// Player body, head and weapon sprites.
    pub mod player {
        pub const GRAND_PECO_CRUSADER_FEMALE: &str =
            "data/sprite/인간족/몸통/여/신페코크루세이더_여";
        pub const GRAND_PECO_CRUSADER_MALE: &str = "data/sprite/인간족/몸통/남/신페코크루세이더_남";
        pub const LORD_PECO_MALE: &str = "data/sprite/인간족/몸통/남/로드페코_남";
        pub const PECO_KNIGHT_MALE: &str = "data/sprite/인간족/몸통/남/페코페코_기사_남";
        pub const PECO_PALADIN_MALE: &str = "data/sprite/인간족/몸통/남/페코팔라딘_남";

        pub fn body(job: &str, sex: &str) -> String {
            format!("data/sprite/인간족/몸통/{sex}/{job}_{sex}")
        }

        pub fn head(head_id: u16, sex: &str) -> String {
            format!("data/sprite/인간족/머리통/{sex}/{head_id}_{sex}")
        }

        pub fn weapon(job: &str, sex: &str, suffix: &str) -> String {
            format!("data/sprite/인간족/{job}/{job}_{sex}{suffix}")
        }

        pub fn gm_body(sex: &str) -> String {
            format!("data/sprite/인간족/몸통/{sex}/운영자_{sex}")
        }

        pub fn gm_weapon(sex: &str) -> String {
            format!("data/sprite/인간족/운영자/운영자_{sex}_검")
        }

        /// The name table already carries the sex/type sub-path.
        pub fn mercenary_body(name: &str) -> String {
            format!("data/sprite/인간족/몸통/{name}")
        }

        pub fn mercenary_weapon(base: &str, weapon_char: char) -> String {
            format!("data/sprite/인간족/용병/{base}_{weapon_char}")
        }
    }

    /// Shield sprites, drawn per job and sex.
    pub mod shield {
        pub fn of(job: &str, sex: &str, shield: impl std::fmt::Display) -> String {
            format!("data/sprite/방패/{job}/{job}_{sex}_{shield}")
        }
    }
}

/// Flat `data/*.txt` lookup tables.
pub mod table {
    pub const CARD_ILLUSTRATION_NAME: &str = "data/num2cardillustnametable.txt";
    pub const CARD_POSTFIX_NAME: &str = "data/cardpostfixnametable.txt";
    pub const CARD_PREFIX_NAME: &str = "data/cardprefixnametable.txt";
    pub const FOG_PARAMETER: &str = "data/fogparametertable.txt";
    pub const IDENTIFIED_ITEM_DESC: &str = "data/idnum2itemdesctable.txt";
    pub const IDENTIFIED_ITEM_NAME: &str = "data/idnum2itemdisplaynametable.txt";
    pub const IDENTIFIED_ITEM_RESOURCE: &str = "data/idnum2itemresnametable.txt";
    pub const INDOOR_RSW: &str = "data/indoorrswtable.txt";
    pub const ITEM_SLOT_COUNT: &str = "data/itemslotcounttable.txt";
    pub const MAP_NAME: &str = "data/mapnametable.txt";
    pub const MAP_POSITION: &str = "data/mappostable.txt";
    pub const MP3_NAME: &str = "data/mp3nametable.txt";
    pub const MSG_STRING: &str = "data/msgstringtable.txt";
    pub const PET_TALK: &str = "data/pettalktable.xml";
    pub const QUEST_DISPLAY: &str = "data/questid2display.txt";
    pub const SKILL_DESC: &str = "data/skilldesctable.txt";
    pub const SKILL_NAME: &str = "data/skillnametable.txt";
    pub const SKILL_SP_AMOUNT: &str = "data/leveluseskillspamount.txt";
    pub const SKILL_TREE: &str = "data/skilltreeview.txt";
    pub const UNIDENTIFIED_ITEM_DESC: &str = "data/num2itemdesctable.txt";
    pub const UNIDENTIFIED_ITEM_NAME: &str = "data/num2itemdisplaynametable.txt";
    pub const UNIDENTIFIED_ITEM_RESOURCE: &str = "data/num2itemresnametable.txt";

    pub fn book(item_id: impl std::fmt::Display) -> String {
        format!("data/book/{item_id}.txt")
    }
}

/// Textures outside the UI tree.
pub mod texture {
    pub const GRID: &str = "data/texture/grid.tga";

    pub fn named(name: &str) -> String {
        format!("data/texture/{name}")
    }

    pub fn water(water_type: i32, frame: usize) -> String {
        format!("data/texture/워터/water{water_type}{frame:02}.jpg")
    }

    /// Effect textures and `.str` animation scripts.
    pub mod effect {
        pub const ALPHABET: &str = "data/texture/effect/alpabet.bmp";

        pub fn named(name: &str) -> String {
            format!("data/texture/effect/{name}")
        }

        pub fn str_file(name: &str) -> String {
            format!("data/texture/effect/{name}.str")
        }
    }
}

/// The `유저인터페이스` tree — every window, button and icon texture.
pub mod ui {
    pub const BTN_ADD: &str = "data/texture/유저인터페이스/btn_add.bmp";
    pub const BTN_ADD_A: &str = "data/texture/유저인터페이스/btn_add_a.bmp";
    pub const BTN_ADD_B: &str = "data/texture/유저인터페이스/btn_add_b.bmp";
    pub const BTN_APPLY: &str = "data/texture/유저인터페이스/btn_apply.bmp";
    pub const BTN_APPLY_A: &str = "data/texture/유저인터페이스/btn_apply_a.bmp";
    pub const BTN_APPLY_B: &str = "data/texture/유저인터페이스/btn_apply_b.bmp";
    pub const BTN_BACK: &str = "data/texture/유저인터페이스/btn_back.bmp";
    pub const BTN_BACK_A: &str = "data/texture/유저인터페이스/btn_back_a.bmp";
    pub const BTN_BACK_B: &str = "data/texture/유저인터페이스/btn_back_b.bmp";
    pub const BTN_BUY: &str = "data/texture/유저인터페이스/btn_buy.bmp";
    pub const BTN_BUY_A: &str = "data/texture/유저인터페이스/btn_buy_a.bmp";
    pub const BTN_BUY_B: &str = "data/texture/유저인터페이스/btn_buy_b.bmp";
    pub const BTN_CANCEL: &str = "data/texture/유저인터페이스/btn_cancel.bmp";
    pub const BTN_CANCEL_A: &str = "data/texture/유저인터페이스/btn_cancel_a.bmp";
    pub const BTN_CANCEL_B: &str = "data/texture/유저인터페이스/btn_cancel_b.bmp";
    pub const BTN_CLOSE: &str = "data/texture/유저인터페이스/btn_close.bmp";
    pub const BTN_CLOSE_A: &str = "data/texture/유저인터페이스/btn_close_a.bmp";
    pub const BTN_CLOSE_B: &str = "data/texture/유저인터페이스/btn_close_b.bmp";
    pub const BTN_DEL: &str = "data/texture/유저인터페이스/btn_del.bmp";
    pub const BTN_DEL_A: &str = "data/texture/유저인터페이스/btn_del_a.bmp";
    pub const BTN_DEL_B: &str = "data/texture/유저인터페이스/btn_del_b.bmp";
    pub const BTN_EDIT: &str = "data/texture/유저인터페이스/btn_edit.bmp";
    pub const BTN_EDIT_A: &str = "data/texture/유저인터페이스/btn_edit_a.bmp";
    pub const BTN_EDIT_B: &str = "data/texture/유저인터페이스/btn_edit_b.bmp";
    pub const BTN_EXCHANGE: &str = "data/texture/유저인터페이스/btn_exchange.bmp";
    pub const BTN_EXCHANGE_A: &str = "data/texture/유저인터페이스/btn_exchange_a.bmp";
    pub const BTN_EXCHANGE_B: &str = "data/texture/유저인터페이스/btn_exchange_b.bmp";
    pub const BTN_EXCHANGE_DIS: &str = "data/texture/유저인터페이스/btn_exchange_dis.bmp";
    pub const BTN_FEED: &str = "data/texture/유저인터페이스/btn_feed.bmp";
    pub const BTN_FEED_A: &str = "data/texture/유저인터페이스/btn_feed_a.bmp";
    pub const BTN_FEED_B: &str = "data/texture/유저인터페이스/btn_feed_b.bmp";
    pub const BTN_FIRED: &str = "data/texture/유저인터페이스/btn_fired.bmp";
    pub const BTN_FIRED_A: &str = "data/texture/유저인터페이스/btn_fired_a.bmp";
    pub const BTN_FIRED_B: &str = "data/texture/유저인터페이스/btn_fired_b.bmp";
    pub const BTN_MAKE: &str = "data/texture/유저인터페이스/btn_make.bmp";
    pub const BTN_MAKE_A: &str = "data/texture/유저인터페이스/btn_make_a.bmp";
    pub const BTN_MAKE_B: &str = "data/texture/유저인터페이스/btn_make_b.bmp";
    pub const BTN_NEXT: &str = "data/texture/유저인터페이스/btn_next.bmp";
    pub const BTN_NEXT_A: &str = "data/texture/유저인터페이스/btn_next_a.bmp";
    pub const BTN_NEXT_B: &str = "data/texture/유저인터페이스/btn_next_b.bmp";
    pub const BTN_OK: &str = "data/texture/유저인터페이스/btn_ok.bmp";
    pub const BTN_OK_A: &str = "data/texture/유저인터페이스/btn_ok_a.bmp";
    pub const BTN_OK_B: &str = "data/texture/유저인터페이스/btn_ok_b.bmp";
    pub const BTN_OK_DIS: &str = "data/texture/유저인터페이스/btn_ok_dis.bmp";
    pub const BTN_RESET: &str = "data/texture/유저인터페이스/btn_reset.bmp";
    pub const BTN_RESET_A: &str = "data/texture/유저인터페이스/btn_reset_a.bmp";
    pub const BTN_RESET_B: &str = "data/texture/유저인터페이스/btn_reset_b.bmp";
    pub const BTN_RESIZE: &str = "data/texture/유저인터페이스/btn_resize.bmp";
    pub const BTN_REWRITE: &str = "data/texture/유저인터페이스/btn_rewrite.bmp";
    pub const BTN_REWRITE_A: &str = "data/texture/유저인터페이스/btn_rewrite_a.bmp";
    pub const BTN_REWRITE_B: &str = "data/texture/유저인터페이스/btn_rewrite_b.bmp";
    pub const BTN_SELL: &str = "data/texture/유저인터페이스/btn_sell.bmp";
    pub const BTN_SELL_A: &str = "data/texture/유저인터페이스/btn_sell_a.bmp";
    pub const BTN_SELL_B: &str = "data/texture/유저인터페이스/btn_sell_b.bmp";
    pub const BTN_SKILL: &str = "data/texture/유저인터페이스/btn_skill.bmp";
    pub const BTN_SKILL_A: &str = "data/texture/유저인터페이스/btn_skill_a.bmp";
    pub const BTN_SKILL_B: &str = "data/texture/유저인터페이스/btn_skill_b.bmp";
    pub const BTN_USE: &str = "data/texture/유저인터페이스/btn_use.bmp";
    pub const BTN_USE_A: &str = "data/texture/유저인터페이스/btn_use_a.bmp";
    pub const BTN_USE_B: &str = "data/texture/유저인터페이스/btn_use_b.bmp";
    pub const BTN_VIEW: &str = "data/texture/유저인터페이스/btn_view.bmp";
    pub const BTN_VIEW_A: &str = "data/texture/유저인터페이스/btn_view_a.bmp";
    pub const BTN_VIEW_B: &str = "data/texture/유저인터페이스/btn_view_b.bmp";
    pub const CHAT_CLOSE: &str = "data/texture/유저인터페이스/chat_close.bmp";
    pub const CHAT_OPEN: &str = "data/texture/유저인터페이스/chat_open.bmp";
    pub const CHECKBOX_0: &str = "data/texture/유저인터페이스/checkbox_0.bmp";
    pub const CHECKBOX_1: &str = "data/texture/유저인터페이스/checkbox_1.bmp";
    pub const EMPTY_CARD_SLOT: &str = "data/texture/유저인터페이스/empty_card_slot.bmp";
    pub const ESC_01A: &str = "data/texture/유저인터페이스/esc_01a.bmp";
    pub const ESC_01B: &str = "data/texture/유저인터페이스/esc_01b.bmp";
    pub const ESC_01C: &str = "data/texture/유저인터페이스/esc_01c.bmp";
    pub const ESC_02A: &str = "data/texture/유저인터페이스/esc_02a.bmp";
    pub const ESC_02B: &str = "data/texture/유저인터페이스/esc_02b.bmp";
    pub const ESC_02C: &str = "data/texture/유저인터페이스/esc_02c.bmp";
    pub const ESC_03A: &str = "data/texture/유저인터페이스/esc_03a.bmp";
    pub const ESC_03B: &str = "data/texture/유저인터페이스/esc_03b.bmp";
    pub const ESC_03C: &str = "data/texture/유저인터페이스/esc_03c.bmp";
    pub const ESC_04A: &str = "data/texture/유저인터페이스/esc_04a.bmp";
    pub const ESC_04B: &str = "data/texture/유저인터페이스/esc_04b.bmp";
    pub const ESC_04C: &str = "data/texture/유저인터페이스/esc_04c.bmp";
    pub const ESC_05A: &str = "data/texture/유저인터페이스/esc_05a.bmp";
    pub const ESC_05B: &str = "data/texture/유저인터페이스/esc_05b.bmp";
    pub const ESC_05C: &str = "data/texture/유저인터페이스/esc_05c.bmp";
    pub const ESC_06A: &str = "data/texture/유저인터페이스/esc_06a.bmp";
    pub const ESC_06B: &str = "data/texture/유저인터페이스/esc_06b.bmp";
    pub const ESC_06C: &str = "data/texture/유저인터페이스/esc_06c.bmp";
    pub const ESC_07A: &str = "data/texture/유저인터페이스/esc_07a.bmp";
    pub const ESC_07B: &str = "data/texture/유저인터페이스/esc_07b.bmp";
    pub const ESC_07C: &str = "data/texture/유저인터페이스/esc_07c.bmp";
    pub const ESC_08A: &str = "data/texture/유저인터페이스/esc_08a.bmp";
    pub const ESC_08B: &str = "data/texture/유저인터페이스/esc_08b.bmp";
    pub const ESC_08C: &str = "data/texture/유저인터페이스/esc_08c.bmp";
    pub const RADIOBTN_OFF: &str = "data/texture/유저인터페이스/radiobtn_off.bmp";
    pub const RADIOBTN_ON: &str = "data/texture/유저인터페이스/radiobtn_on.bmp";
    pub const RAG_TITLE: &str = "data/texture/유저인터페이스/rag_title.bmp";
    pub const RAG_TITLE2: &str = "data/texture/유저인터페이스/rag_title2.bmp";
    pub const RAG_TITLE3: &str = "data/texture/유저인터페이스/rag_title3.bmp";
    pub const SCROLL0DOWN: &str = "data/texture/유저인터페이스/scroll0down.bmp";
    pub const SCROLL0UP: &str = "data/texture/유저인터페이스/scroll0up.bmp";
    pub const SCROLL1LEFT: &str = "data/texture/유저인터페이스/scroll1left.bmp";
    pub const SCROLL1RIGHT: &str = "data/texture/유저인터페이스/scroll1right.bmp";
    pub const SHOP: &str = "data/texture/유저인터페이스/shop.bmp";
    pub const SYSBOX_BG: &str = "data/texture/유저인터페이스/sysbox_bg.bmp";
    pub const SYSBOX_LD: &str = "data/texture/유저인터페이스/sysbox_ld.bmp";
    pub const SYSBOX_LM: &str = "data/texture/유저인터페이스/sysbox_lm.bmp";
    pub const SYSBOX_LU: &str = "data/texture/유저인터페이스/sysbox_lu.bmp";
    pub const SYSBOX_MD: &str = "data/texture/유저인터페이스/sysbox_md.bmp";
    pub const SYSBOX_MU: &str = "data/texture/유저인터페이스/sysbox_mu.bmp";
    pub const SYSBOX_RD: &str = "data/texture/유저인터페이스/sysbox_rd.bmp";
    pub const SYSBOX_RM: &str = "data/texture/유저인터페이스/sysbox_rm.bmp";
    pub const SYSBOX_RU: &str = "data/texture/유저인터페이스/sysbox_ru.bmp";
    pub const WIN_MSGBOX: &str = "data/texture/유저인터페이스/win_msgbox.bmp";
    pub const WORLDMAP: &str = "data/texture/유저인터페이스/worldmap.bmp";

    /// Shared window chrome and the in-game HUD.
    pub mod basic {
        pub const ARW_LEFT: &str = "data/texture/유저인터페이스/basic_interface/arw_left.bmp";
        pub const ARW_RIGHT: &str = "data/texture/유저인터페이스/basic_interface/arw_right.bmp";
        pub const ARW_RIGHT_ON: &str =
            "data/texture/유저인터페이스/basic_interface/arw_right_on.bmp";
        pub const BASEWIN_BG: &str = "data/texture/유저인터페이스/basic_interface/basewin_bg.bmp";
        pub const BASEWIN_MINI: &str =
            "data/texture/유저인터페이스/basic_interface/basewin_mini.bmp";
        pub const BTNBAR_MID2: &str = "data/texture/유저인터페이스/basic_interface/btnbar_mid2.bmp";
        pub const BTN_CANCEL2: &str = "data/texture/유저인터페이스/basic_interface/cancel2.bmp";
        pub const BTN_CANCEL2_A: &str = "data/texture/유저인터페이스/basic_interface/cancel2_a.bmp";
        pub const BTN_CLOSE2: &str = "data/texture/유저인터페이스/basic_interface/close2.bmp";
        pub const BTN_CLOSE2_A: &str = "data/texture/유저인터페이스/basic_interface/close2_a.bmp";
        pub const BTN_DEL: &str = "data/texture/유저인터페이스/basic_interface/del.bmp";
        pub const BTN_DEL_A: &str = "data/texture/유저인터페이스/basic_interface/del_a.bmp";
        pub const BTN_REMAIL: &str = "data/texture/유저인터페이스/basic_interface/remail.bmp";
        pub const BTN_REMAIL_A: &str = "data/texture/유저인터페이스/basic_interface/remail_a.bmp";
        pub const BTN_RETURN: &str = "data/texture/유저인터페이스/basic_interface/return.bmp";
        pub const BTN_RETURN_A: &str = "data/texture/유저인터페이스/basic_interface/return_a.bmp";
        pub const BTN_SEND: &str = "data/texture/유저인터페이스/basic_interface/send.bmp";
        pub const BTN_SEND_A: &str = "data/texture/유저인터페이스/basic_interface/send_a.bmp";
        pub const BTN_CLOSE: &str = "data/texture/유저인터페이스/basic_interface/btn_close.bmp";
        pub const BTN_CLOSE_A: &str = "data/texture/유저인터페이스/basic_interface/btn_close_a.bmp";
        pub const BTN_CLOSE_B: &str = "data/texture/유저인터페이스/basic_interface/btn_close_b.bmp";
        pub const BTN_DIALOG_OFF: &str =
            "data/texture/유저인터페이스/basic_interface/btn_dialog_off.bmp";
        pub const BTN_DIALOG_ON: &str =
            "data/texture/유저인터페이스/basic_interface/btn_dialog_on.bmp";
        pub const BTN_EQUIP_OFF: &str =
            "data/texture/유저인터페이스/basic_interface/btn_equip_off.bmp";
        pub const BTN_EQUIP_ON: &str =
            "data/texture/유저인터페이스/basic_interface/btn_equip_on.bmp";
        pub const BTN_FRIEND_OFF: &str =
            "data/texture/유저인터페이스/basic_interface/btn_friend_off.bmp";
        pub const BTN_FRIEND_ON: &str =
            "data/texture/유저인터페이스/basic_interface/btn_friend_on.bmp";
        pub const BTN_ITEMS_OFF: &str =
            "data/texture/유저인터페이스/basic_interface/btn_items_off.bmp";
        pub const BTN_ITEMS_ON: &str =
            "data/texture/유저인터페이스/basic_interface/btn_items_on.bmp";
        pub const BTN_MAP_OFF: &str = "data/texture/유저인터페이스/basic_interface/btn_map_off.bmp";
        pub const BTN_MAP_ON: &str = "data/texture/유저인터페이스/basic_interface/btn_map_on.bmp";
        pub const BTN_OFF: &str = "data/texture/유저인터페이스/basic_interface/btn_off.bmp";
        pub const BTN_OPTION_OFF: &str =
            "data/texture/유저인터페이스/basic_interface/btn_option_off.bmp";
        pub const BTN_OPTION_ON: &str =
            "data/texture/유저인터페이스/basic_interface/btn_option_on.bmp";
        pub const BTN_SKILL_OFF: &str =
            "data/texture/유저인터페이스/basic_interface/btn_skill_off.bmp";
        pub const BTN_SKILL_ON: &str =
            "data/texture/유저인터페이스/basic_interface/btn_skill_on.bmp";
        pub const BTN_STATUS_OFF: &str =
            "data/texture/유저인터페이스/basic_interface/btn_status_off.bmp";
        pub const BTN_STATUS_ON: &str =
            "data/texture/유저인터페이스/basic_interface/btn_status_on.bmp";
        pub const COLLECTION_BG: &str =
            "data/texture/유저인터페이스/basic_interface/collection_bg.bmp";
        pub const COPARISON_DISABLE_CARD_SLOT: &str =
            "data/texture/유저인터페이스/basic_interface/coparison_disable_card_slot.bmp";
        pub const DIALOG_BG: &str = "data/texture/유저인터페이스/basic_interface/dialog_bg.bmp";
        pub const DIALOG_BTN0: &str = "data/texture/유저인터페이스/basic_interface/dialog_btn0.bmp";
        pub const DIALOG_BTN1: &str = "data/texture/유저인터페이스/basic_interface/dialog_btn1.bmp";
        pub const DIALOG_BTN2: &str = "data/texture/유저인터페이스/basic_interface/dialog_btn2.bmp";
        pub const DIALSCR_DOWN: &str =
            "data/texture/유저인터페이스/basic_interface/dialscr_down.bmp";
        pub const DIALSCR_UP: &str = "data/texture/유저인터페이스/basic_interface/dialscr_up.bmp";
        pub const ENVELOP: &str = "data/texture/유저인터페이스/basic_interface/envelop.bmp";
        pub const EQUIPWIN_BG: &str = "data/texture/유저인터페이스/basic_interface/equipwin_bg.bmp";
        pub const EXCHANGE_BG2: &str =
            "data/texture/유저인터페이스/basic_interface/exchange_bg2.bmp";
        pub const GRP_LEADER: &str = "data/texture/유저인터페이스/basic_interface/grp_leader.bmp";
        pub const GRP_ONLINE: &str = "data/texture/유저인터페이스/basic_interface/grp_online.bmp";
        pub const GZEBLUE_LEFT: &str =
            "data/texture/유저인터페이스/basic_interface/gzeblue_left.bmp";
        pub const GZEBLUE_MID: &str = "data/texture/유저인터페이스/basic_interface/gzeblue_mid.bmp";
        pub const GZEBLUE_RIGHT: &str =
            "data/texture/유저인터페이스/basic_interface/gzeblue_right.bmp";
        pub const GZERED_LEFT: &str = "data/texture/유저인터페이스/basic_interface/gzered_left.bmp";
        pub const GZERED_MID: &str = "data/texture/유저인터페이스/basic_interface/gzered_mid.bmp";
        pub const GZERED_RIGHT: &str =
            "data/texture/유저인터페이스/basic_interface/gzered_right.bmp";
        pub const ITEMWIN_MID: &str = "data/texture/유저인터페이스/basic_interface/itemwin_mid.bmp";
        pub const ITEM_INVERT: &str = "data/texture/유저인터페이스/basic_interface/item_invert.bmp";
        pub const LV_UP_OFF: &str = "data/texture/유저인터페이스/basic_interface/lv_up_off.bmp";
        pub const LV_UP_ON: &str = "data/texture/유저인터페이스/basic_interface/lv_up_on.bmp";
        pub const MAILLIST1_BG: &str =
            "data/texture/유저인터페이스/basic_interface/maillist1_bg.bmp";
        pub const MAILLIST2_BG: &str =
            "data/texture/유저인터페이스/basic_interface/maillist2_bg.bmp";
        pub const MAILLIST3_BG: &str =
            "data/texture/유저인터페이스/basic_interface/maillist3_bg.bmp";
        pub const MESBTN_010: &str = "data/texture/유저인터페이스/basic_interface/mesbtn_010.bmp";
        pub const MESBTN_02: &str = "data/texture/유저인터페이스/basic_interface/mesbtn_02.bmp";
        pub const MESBTN_04: &str = "data/texture/유저인터페이스/basic_interface/mesbtn_04.bmp";
        pub const MESBTN_05: &str = "data/texture/유저인터페이스/basic_interface/mesbtn_05.bmp";
        pub const MESBTN_08: &str = "data/texture/유저인터페이스/basic_interface/mesbtn_08.bmp";
        pub const MESBTN_09: &str = "data/texture/유저인터페이스/basic_interface/mesbtn_09.bmp";
        pub const QUEST_WINDOW: &str =
            "data/texture/유저인터페이스/basic_interface/quest_window.bmp";
        pub const SHORTITEM_BG: &str =
            "data/texture/유저인터페이스/basic_interface/shortitem_bg.bmp";
        pub const SKILL_UP_A: &str = "data/texture/유저인터페이스/basic_interface/skill_up_a.bmp";
        pub const SKILL_UP_B: &str = "data/texture/유저인터페이스/basic_interface/skill_up_b.bmp";
        pub const SKILL_UP_C: &str = "data/texture/유저인터페이스/basic_interface/skill_up_c.bmp";
        pub const STATWIN0_BG: &str = "data/texture/유저인터페이스/basic_interface/statwin0_bg.bmp";
        pub const SYS_BASE_OFF: &str =
            "data/texture/유저인터페이스/basic_interface/sys_base_off.bmp";
        pub const SYS_BASE_ON: &str = "data/texture/유저인터페이스/basic_interface/sys_base_on.bmp";
        pub const SYS_CLOSE_OFF: &str =
            "data/texture/유저인터페이스/basic_interface/sys_close_off.bmp";
        pub const SYS_CLOSE_ON: &str =
            "data/texture/유저인터페이스/basic_interface/sys_close_on.bmp";
        pub const SYS_MINI_OFF: &str =
            "data/texture/유저인터페이스/basic_interface/sys_mini_off.bmp";
        pub const SYS_MINI_ON: &str = "data/texture/유저인터페이스/basic_interface/sys_mini_on.bmp";
        pub const TAB_ITM_01: &str = "data/texture/유저인터페이스/basic_interface/tab_itm_01.bmp";
        pub const TAB_ITM_02: &str = "data/texture/유저인터페이스/basic_interface/tab_itm_02.bmp";
        pub const TAB_ITM_03: &str = "data/texture/유저인터페이스/basic_interface/tab_itm_03.bmp";
        pub const TAB_QUE_01: &str = "data/texture/유저인터페이스/basic_interface/tab_que_01.bmp";
        pub const TAB_QUE_02: &str = "data/texture/유저인터페이스/basic_interface/tab_que_02.bmp";
        pub const TAB_QUE_03: &str = "data/texture/유저인터페이스/basic_interface/tab_que_03.bmp";
        pub const TITLEBAR_FIX: &str =
            "data/texture/유저인터페이스/basic_interface/titlebar_fix.bmp";
        pub const TITLEBAR_MID: &str =
            "data/texture/유저인터페이스/basic_interface/titlebar_mid.bmp";
        pub const TXTBOX_BTN_A: &str =
            "data/texture/유저인터페이스/basic_interface/txtbox_btn_a.bmp";
        pub const TXTBOX_BTN_B: &str =
            "data/texture/유저인터페이스/basic_interface/txtbox_btn_b.bmp";
        pub const TXTBOX_BTN_C: &str =
            "data/texture/유저인터페이스/basic_interface/txtbox_btn_c.bmp";

        /// Party-recruitment board buttons.
        pub mod seekparty {
            pub const BTN_CLEAR_A: &str =
                "data/texture/유저인터페이스/basic_interface/seekparty/btn_clear_a.bmp";
            pub const BTN_CLEAR_B: &str =
                "data/texture/유저인터페이스/basic_interface/seekparty/btn_clear_b.bmp";
            pub const BTN_CLEAR_C: &str =
                "data/texture/유저인터페이스/basic_interface/seekparty/btn_clear_c.bmp";
        }
    }

    /// Card illustrations.
    pub mod cardbmp {
        pub fn named(name: &str) -> String {
            format!("data/texture/유저인터페이스/cardbmp/{name}.bmp")
        }
    }

    /// Item collection illustrations.
    pub mod collection {
        pub fn named(name: &str) -> String {
            format!("data/texture/유저인터페이스/collection/{name}.bmp")
        }
    }

    /// Pet illustrations, keyed by the pet mob.
    pub mod illust {
        pub const PET_ALICE: &str = "data/texture/유저인터페이스/illust/펫_ALICE.bmp";
        pub const PET_BABY_DESERT_WOLF: &str =
            "data/texture/유저인터페이스/illust/펫_데져트울프새끼.bmp";
        pub const PET_BACSOJIN: &str = "data/texture/유저인터페이스/illust/펫_BACSOJIN.bmp";
        pub const PET_BAPHOMET_JR: &str = "data/texture/유저인터페이스/illust/펫_바포메트.bmp";
        pub const PET_BONGUN: &str = "data/texture/유저인터페이스/illust/펫_본건.bmp";
        pub const PET_CHONCHON: &str = "data/texture/유저인터페이스/illust/펫_촌촌.bmp";
        pub const PET_CHRISTMAS_SNOW_RABBIT: &str =
            "data/texture/유저인터페이스/illust/펫_크리스마스_눈토끼.bmp";
        pub const PET_CHUNG_E: &str = "data/texture/유저인터페이스/illust/펫_청이.bmp";
        pub const PET_CIVIL_SERVANT: &str =
            "data/texture/유저인터페이스/illust/펫_CIVIL_SERVANT.bmp";
        pub const PET_DELETER: &str = "data/texture/유저인터페이스/illust/펫_지상딜리터.bmp";
        pub const PET_DEVIRUCHI: &str = "data/texture/유저인터페이스/illust/펫_데비루치.bmp";
        pub const PET_DIABOLIC: &str = "data/texture/유저인터페이스/illust/펫_디아볼릭.bmp";
        pub const PET_DOKEBI: &str = "data/texture/유저인터페이스/illust/펫_도깨비.bmp";
        pub const PET_DROPS: &str = "data/texture/유저인터페이스/illust/펫_드롭프스.bmp";
        pub const PET_DULLAHAN: &str = "data/texture/유저인터페이스/illust/펫_DULLAHAN.bmp";
        pub const PET_GOBLIN_DAGGER: &str = "data/texture/유저인터페이스/illust/펫_고블린_단검.bmp";
        pub const PET_GOBLIN_EVENT: &str =
            "data/texture/유저인터페이스/illust/펫_고블린_이벤트.bmp";
        pub const PET_GOBLIN_FLAIL: &str =
            "data/texture/유저인터페이스/illust/펫_고블린_플레일.bmp";
        pub const PET_GOBLIN_HAMMER: &str = "data/texture/유저인터페이스/illust/펫_고블린_해머.bmp";
        pub const PET_GOBLIN_LEADER: &str =
            "data/texture/유저인터페이스/illust/펫_GOBLIN_LEADER.bmp";
        pub const PET_GOLEM: &str = "data/texture/유저인터페이스/illust/펫_GOLEM.bmp";
        pub const PET_HUNTER_FLY: &str = "data/texture/유저인터페이스/illust/펫_헌터플라이.bmp";
        pub const PET_IMP: &str = "data/texture/유저인터페이스/illust/펫_IMP.bmp";
        pub const PET_INCUBUS: &str = "data/texture/유저인터페이스/illust/펫_INCUBUS.bmp";
        pub const PET_ISIS: &str = "data/texture/유저인터페이스/illust/펫_이시스.bmp";
        pub const PET_J_TAINI: &str = "data/texture/유저인터페이스/illust/펫_j_taini.bmp";
        pub const PET_LEAF_CAT: &str = "data/texture/유저인터페이스/illust/펫_LEAF_CAT.bmp";
        pub const PET_LOLI_RURI: &str = "data/texture/유저인터페이스/illust/펫_LOLI_RURI.BMP";
        pub const PET_LUNATIC: &str = "data/texture/유저인터페이스/illust/펫_루나틱.bmp";
        pub const PET_MARIONETTE: &str = "data/texture/유저인터페이스/illust/펫_MARIONETTE.bmp";
        pub const PET_MEDUSA: &str = "data/texture/유저인터페이스/illust/펫_MEDUSA.bmp";
        pub const PET_MIYABI_NINGYO: &str =
            "data/texture/유저인터페이스/illust/펫_MIYABI_NINGYO.bmp";
        pub const PET_MUNAK: &str = "data/texture/유저인터페이스/illust/펫_무낙.bmp";
        pub const PET_NIGHTMARE_TERROR: &str =
            "data/texture/유저인터페이스/illust/펫_NIGHTMARE_TERROR.bmp";
        pub const PET_ORC_WARRIOR: &str = "data/texture/유저인터페이스/illust/펫_오크워리어.bmp";
        pub const PET_PECOPECO: &str = "data/texture/유저인터페이스/illust/펫_페코페코.bmp";
        pub const PET_PETITE: &str = "data/texture/유저인터페이스/illust/펫_쁘띠.bmp";
        pub const PET_PICKY: &str = "data/texture/유저인터페이스/illust/펫_픽키.bmp";
        pub const PET_POISON_SPORE: &str = "data/texture/유저인터페이스/illust/펫_포이즌스포아.bmp";
        pub const PET_POPORING: &str = "data/texture/유저인터페이스/illust/펫_포포링.bmp";
        pub const PET_PORING: &str = "data/texture/유저인터페이스/illust/펫_포링.bmp";
        pub const PET_RICE_CAKE: &str = "data/texture/유저인터페이스/illust/펫_떡_이벤트.bmp";
        pub const PET_ROCKER: &str = "data/texture/유저인터페이스/illust/펫_로커.bmp";
        pub const PET_SAVAGE_BABE: &str = "data/texture/유저인터페이스/illust/펫_세비지베베.bmp";
        pub const PET_SHINOBI: &str = "data/texture/유저인터페이스/illust/펫_SHINOBI.bmp";
        pub const PET_SMOKIE: &str = "data/texture/유저인터페이스/illust/펫_스모키.bmp";
        pub const PET_SOHEE: &str = "data/texture/유저인터페이스/illust/펫_소희.bmp";
        pub const PET_SPORE: &str = "data/texture/유저인터페이스/illust/펫_스포아.bmp";
        pub const PET_STEEL_CHONCHON: &str = "data/texture/유저인터페이스/illust/펫_스틸촌촌.bmp";
        pub const PET_STONE_SHOOTER: &str =
            "data/texture/유저인터페이스/illust/펫_STONE_SHOOTER.bmp";
        pub const PET_SUCCUBUS: &str = "data/texture/유저인터페이스/illust/펫_SUCCUBUS.bmp";
        pub const PET_WANDERER: &str = "data/texture/유저인터페이스/illust/펫_배회하는자.bmp";
        pub const PET_WHISPER: &str = "data/texture/유저인터페이스/illust/펫_WHISPER.BMP";
        pub const PET_WICKED_NYMPH: &str = "data/texture/유저인터페이스/illust/펫_WICKED_NYMPH.bmp";
        pub const PET_YOYO: &str = "data/texture/유저인터페이스/illust/펫_요요.bmp";
        pub const PET_ZHERLTHSH: &str = "data/texture/유저인터페이스/illust/펫_ZHERLTHSH.bmp";

        pub fn named(name: &str) -> String {
            format!("data/texture/유저인터페이스/illust/{name}.bmp")
        }
    }

    /// Inventory item icons.
    pub mod item {
        pub const CAT_PAW_HAIRPIN: &str = "data/texture/유저인터페이스/item/고양이발머리핀.bmp";

        pub fn icon(name: &str) -> String {
            format!("data/texture/유저인터페이스/item/{name}.bmp")
        }
    }

    /// Login, server-select and character-create screens.
    pub mod login {
        pub const ARW_AGI0: &str = "data/texture/유저인터페이스/login_interface/arw-agi0.bmp";
        pub const ARW_AGI1: &str = "data/texture/유저인터페이스/login_interface/arw-agi1.bmp";
        pub const ARW_DEX0: &str = "data/texture/유저인터페이스/login_interface/arw-dex0.bmp";
        pub const ARW_DEX1: &str = "data/texture/유저인터페이스/login_interface/arw-dex1.bmp";
        pub const ARW_INT0: &str = "data/texture/유저인터페이스/login_interface/arw-int0.bmp";
        pub const ARW_INT1: &str = "data/texture/유저인터페이스/login_interface/arw-int1.bmp";
        pub const ARW_LUK0: &str = "data/texture/유저인터페이스/login_interface/arw-luk0.bmp";
        pub const ARW_LUK1: &str = "data/texture/유저인터페이스/login_interface/arw-luk1.bmp";
        pub const ARW_STR0: &str = "data/texture/유저인터페이스/login_interface/arw-str0.bmp";
        pub const ARW_STR1: &str = "data/texture/유저인터페이스/login_interface/arw-str1.bmp";
        pub const ARW_VIT0: &str = "data/texture/유저인터페이스/login_interface/arw-vit0.bmp";
        pub const ARW_VIT1: &str = "data/texture/유저인터페이스/login_interface/arw-vit1.bmp";
        pub const BOX_SELECT: &str = "data/texture/유저인터페이스/login_interface/box_select.bmp";
        pub const BTN_CONNECT: &str = "data/texture/유저인터페이스/login_interface/btn_connect.bmp";
        pub const BTN_CONNECT_A: &str =
            "data/texture/유저인터페이스/login_interface/btn_connect_a.bmp";
        pub const BTN_CONNECT_B: &str =
            "data/texture/유저인터페이스/login_interface/btn_connect_b.bmp";
        pub const BTN_EXIT: &str = "data/texture/유저인터페이스/login_interface/btn_exit.bmp";
        pub const BTN_EXIT_A: &str = "data/texture/유저인터페이스/login_interface/btn_exit_a.bmp";
        pub const BTN_EXIT_B: &str = "data/texture/유저인터페이스/login_interface/btn_exit_b.bmp";
        pub const CHK_SAVEOFF: &str = "data/texture/유저인터페이스/login_interface/chk_saveoff.bmp";
        pub const CHK_SAVEON: &str = "data/texture/유저인터페이스/login_interface/chk_saveon.bmp";
        pub const NAME_EDIT: &str = "data/texture/유저인터페이스/login_interface/name-edit.bmp";
        pub const WIN_LOGIN: &str = "data/texture/유저인터페이스/login_interface/win_login.bmp";
        pub const WIN_MAKE: &str = "data/texture/유저인터페이스/login_interface/win_make.bmp";
        pub const WIN_MAKE2: &str = "data/texture/유저인터페이스/login_interface/win_make2.bmp";
        pub const WIN_SELECT: &str = "data/texture/유저인터페이스/login_interface/win_select.bmp";
        pub const WIN_SERVICE: &str = "data/texture/유저인터페이스/login_interface/win_service.bmp";
    }

    /// Minimap frame and per-map minimap images.
    pub mod minimap {
        pub const MAP_ARROW: &str = "data/texture/유저인터페이스/map/map_arrow.bmp";
        pub const MAP_MINUS0: &str = "data/texture/유저인터페이스/map/map_minus0.bmp";
        pub const MAP_MINUS1: &str = "data/texture/유저인터페이스/map/map_minus1.bmp";
        pub const MAP_PLUS0: &str = "data/texture/유저인터페이스/map/map_plus0.bmp";
        pub const MAP_PLUS1: &str = "data/texture/유저인터페이스/map/map_plus1.bmp";
        pub const PRONTERA: &str = "data/texture/유저인터페이스/map/prontera.bmp";

        pub fn of(map_name: &str) -> String {
            format!("data/texture/유저인터페이스/map/{map_name}.bmp")
        }
    }
}
