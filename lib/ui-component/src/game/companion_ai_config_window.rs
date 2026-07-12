use crate::Window;
use crate::helper::colors;
use crate::helper::scrollbar::{self, SCROLLBAR_W, ScrollbarIds};
use crate::helper::window_chrome::{
    TITLEBAR_TEX, draw_container, draw_sys_button, draw_titlebar, text_color,
};
use ragnarok_ai::config::{CompanionAiConfig, HomunConfig, MercConfig};
use ragnarok_ai::consts::{
    BasicTactic, CastTactic, ChaseTactic, KiteTactic, KsTactic, PushbackTactic, RescueTactic,
    SkillClass, SnipeTactic,
};
use ragnarok_ai::tactics::{SkillUse, Tactic};
use ragnarok_game::event::GameEvent;
use ragnarok_ui::draw::{self, DrawCall, TextureRef};
use ragnarok_ui::frame::{UiFrame, WidgetId};
use ragnarok_ui::rect::Rect;

pub const COMPANION_AI_CONFIG_WINDOW_ID: WidgetId = WidgetId(3200);
const CLOSE_BTN_ID: WidgetId = WidgetId(3201);
const TAB_BASE_ID: WidgetId = WidgetId(3210);
const APPLY_BTN_ID: WidgetId = WidgetId(3220);
const REVERT_BTN_ID: WidgetId = WidgetId(3221);
const RESET_BTN_ID: WidgetId = WidgetId(3222);
const SCROLL_UP_ID: WidgetId = WidgetId(3223);
const SCROLL_DOWN_ID: WidgetId = WidgetId(3224);
const SCROLL_THUMB_ID: WidgetId = WidgetId(3225);
const TACT_PREV_ID: WidgetId = WidgetId(3226);
const TACT_NEXT_ID: WidgetId = WidgetId(3227);
const TACT_ADD_ID: WidgetId = WidgetId(3228);
const TACT_DEL_ID: WidgetId = WidgetId(3229);
const ROW_WIDGET_BASE: u32 = 3230;

const BASIC_OPTS: &[(i32, &str)] = &[
    (-2, "Tank Mob"),
    (-1, "Tank"),
    (0, "Ignore"),
    (2, "Attack Low"),
    (3, "Attack Med"),
    (4, "Attack High"),
    (5, "React Low"),
    (7, "React Med"),
    (8, "React High"),
    (9, "React Self"),
    (10, "Snipe Low"),
    (11, "Snipe Med"),
    (12, "Snipe High"),
];
const KITE_OPTS: &[(i32, &str)] = &[(0, "Never"), (1, "React"), (2, "Always")];
const CAST_OPTS: &[(i32, &str)] = &[(0, "Passive"), (1, "React")];
const PUSH_OPTS: &[(i32, &str)] = &[(0, "Never"), (1, "Self"), (2, "Friend")];
const CLASS_OPTS: &[(i32, &str)] = &[
    (-1, "Both"),
    (0, "Old"),
    (1, "S"),
    (2, "Mob"),
    (3, "Combo 1"),
    (4, "Combo 2"),
    (5, "Minion"),
    (6, "Grapple"),
];
const RESCUE_OPTS: &[(i32, &str)] = &[
    (0, "Never"),
    (1, "Friend"),
    (2, "Retainer"),
    (3, "Self"),
    (4, "Owner"),
    (5, "All"),
];
const SNIPE_OPTS: &[(i32, &str)] = &[(0, "Disable"), (1, "OK")];
const KS_OPTS: &[(i32, &str)] = &[(-1, "Polite"), (0, "Never"), (1, "Always")];
const CHASE_OPTS: &[(i32, &str)] = &[(-1, "Normal"), (0, "Always"), (1, "Never"), (2, "Clever")];

fn tactic_cols() -> Vec<FieldSpec<Tactic>> {
    use Widget::*;
    vec![
        FieldSpec { label: "Basic", category: "", widget: Enum(BASIC_OPTS), tip: "",
            get: |t| i32::from(t.basic), set: |t, v| t.basic = BasicTactic::from(v) },
        FieldSpec { label: "Skill Use", category: "", widget: Int { min: -20, max: 100, step: 1 }, tip: "0 never, 100 always, N up to N casts, -N once at level N",
            get: |t| i32::from(t.skill), set: |t, v| t.skill = SkillUse::from(v) },
        FieldSpec { label: "Kite", category: "", widget: Enum(KITE_OPTS), tip: "",
            get: |t| i32::from(t.kite), set: |t, v| t.kite = KiteTactic::from(v) },
        FieldSpec { label: "Cast React", category: "", widget: Enum(CAST_OPTS), tip: "",
            get: |t| i32::from(t.cast), set: |t, v| t.cast = CastTactic::from(v) },
        FieldSpec { label: "Pushback", category: "", widget: Enum(PUSH_OPTS), tip: "",
            get: |t| i32::from(t.pushback), set: |t, v| t.pushback = PushbackTactic::from(v) },
        FieldSpec { label: "Debuff Skill", category: "", widget: Int { min: -9000, max: 9000, step: 1 }, tip: "Debuff skill id, or a negative status code",
            get: |t| t.debuff, set: |t, v| t.debuff = v },
        FieldSpec { label: "Skill Class", category: "", widget: Enum(CLASS_OPTS), tip: "",
            get: |t| i32::from(t.skill_class), set: |t, v| t.skill_class = SkillClass::from(v) },
        FieldSpec { label: "Rescue", category: "", widget: Enum(RESCUE_OPTS), tip: "",
            get: |t| i32::from(t.rescue), set: |t, v| t.rescue = RescueTactic::from(v) },
        FieldSpec { label: "SP Reserve", category: "", widget: Int { min: -1, max: 100, step: 1 }, tip: "-1 uses Attack Skill Reserve SP",
            get: |t| t.sp, set: |t, v| t.sp = v },
        FieldSpec { label: "Snipe", category: "", widget: Enum(SNIPE_OPTS), tip: "",
            get: |t| i32::from(t.snipe), set: |t, v| t.snipe = SnipeTactic::from(v) },
        FieldSpec { label: "KS", category: "", widget: Enum(KS_OPTS), tip: "",
            get: |t| i32::from(t.ks), set: |t, v| t.ks = KsTactic::from(v) },
        FieldSpec { label: "Weight x10", category: "", widget: Int { min: 0, max: 30, step: 1 }, tip: "Aggro/mob-count weight, in tenths",
            get: |t| (t.weight * 10.0).round() as i32, set: |t, v| t.weight = v as f32 / 10.0 },
        FieldSpec { label: "Chase", category: "", widget: Enum(CHASE_OPTS), tip: "",
            get: |t| i32::from(t.chase), set: |t, v| t.chase = ChaseTactic::from(v) },
    ]
}

const CLOSE_OFF_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_close_off.bmp";
const CLOSE_ON_TEX: &str = "data/texture/유저인터페이스/basic_interface/sys_close_on.bmp";

const WIN_W: f32 = 384.0;
const WIN_H: f32 = 340.0;
const TITLE_H: f32 = 17.0;
const TAB_H: f32 = 18.0;
const FOOTER_H: f32 = 24.0;
const ROW_H: f32 = 18.0;
const PAD: f32 = 6.0;
const BASELINE: f32 = 12.0;

const TABS: [&str; 7] = [
    "Homun",
    "Merc",
    "H.Tactics",
    "M.Tactics",
    "H.PVP",
    "M.PVP",
    "Extra",
];

#[derive(Clone, Copy)]
enum Widget {
    Bool,
    Int { min: i32, max: i32, step: i32 },
    Enum(&'static [(i32, &'static str)]),
}

struct FieldSpec<C: 'static> {
    label: &'static str,
    category: &'static str,
    widget: Widget,
    tip: &'static str,
    get: fn(&C) -> i32,
    set: fn(&mut C, i32),
}

const USE_SKILL_ONLY_OPTS: &[(i32, &str)] =
    &[(0, "Attacking"), (-1, "Chasing"), (1, "Skill Only")];
const AUTO_MOB_OPTS: &[(i32, &str)] = &[(0, "Disabled"), (1, "Aggressive"), (2, "All")];
const AUTO_COMBO_OPTS: &[(i32, &str)] = &[(0, "Never"), (1, "Tactics"), (2, "Always")];
const IDLE_WALK_OPTS: &[(i32, &str)] = &[
    (0, "None"),
    (1, "Circle"),
    (2, "Cross"),
    (3, "Square"),
    (4, "Random"),
    (5, "Route Linear"),
    (6, "Route Loop"),
];
const STICKY_STANDBY_OPTS: &[(i32, &str)] =
    &[(0, "Disabled"), (1, "Enabled"), (2, "Enabled+Relog")];
const AUTO_HEAL_OPTS: &[(i32, &str)] =
    &[(0, "Never"), (1, "Always"), (2, "Idle"), (3, "Idle Low")];
const PUSHBACK_OPTS: &[(i32, &str)] = &[(0, "Off"), (1, "Self"), (2, "All")];
const OLD_HOMUN_OPTS: &[(i32, &str)] = &[(1, "Lif"), (2, "Amistr"), (3, "Filir")];
const BUFF_WHEN_OPTS: &[(i32, &str)] = &[
    (-2, "Idle Low"),
    (-1, "Chase"),
    (0, "Never"),
    (1, "Idle"),
    (2, "Berserk"),
    (3, "ASAP"),
];
const GROUND_BUFF_OPTS: &[(i32, &str)] = &[
    (-2, "Idle Low"),
    (-1, "Chase"),
    (0, "Attack"),
    (1, "Idle"),
    (2, "Berserk"),
];

macro_rules! spec {
    ($label:expr, $cat:expr, $field:ident, $w:expr, $tip:expr) => {
        FieldSpec {
            label: $label,
            category: $cat,
            widget: $w,
            tip: $tip,
            get: |c| c.$field,
            set: |c, v| c.$field = v,
        }
    };
}

fn homun_specs() -> Vec<FieldSpec<HomunConfig>> {
    use Widget::*;
    vec![
        spec!("Aggro HP %", "Basic", AggroHP, Int { min: 0, max: 100, step: 5 }, "Only aggress while HP is above this percent."),
        spec!("Aggro SP %", "Basic", AggroSP, Int { min: 0, max: 100, step: 5 }, "Only aggress while SP is above this percent."),
        spec!("Old Homun Type", "Basic", OldHomunType, Enum(OLD_HOMUN_OPTS), "Pre-evolution family used for skill selection."),
        spec!("Use Skill Only", "Basic", UseSkillOnly, Enum(USE_SKILL_ONLY_OPTS), "When attack skills may be used."),
        spec!("Use Attack Skill", "Basic", UseAttackSkill, Bool, "Enable attack-skill selection."),
        spec!("Opportunistic Target", "Basic", OpportunisticTargeting, Bool, "Re-target a closer/weaker monster mid-fight."),
        spec!("Do Not Chase", "Basic", DoNotChase, Bool, "Never leave attack range to chase."),
        spec!("Use Dance Attack", "Basic", UseDanceAttack, Bool, "Use the move-cancel dance attack."),
        spec!("Super Passive", "Basic", SuperPassive, Bool, "Never attack unless commanded."),
        spec!("Rescue Owner Low HP", "Basic", RescueOwnerLowHP, Bool, "Prioritize the owner's attacker when the owner is low."),
        spec!("Tank Monster Limit", "Basic", TankMonsterLimit, Int { min: 0, max: 12, step: 1 }, "Max monsters to tank at once."),
        spec!("Stationary Aggro Dist", "Basic", StationaryAggroDist, Int { min: 1, max: 14, step: 1 }, "Aggro radius while the owner is still."),
        spec!("Mobile Aggro Dist", "Basic", MobileAggroDist, Int { min: 1, max: 14, step: 1 }, "Aggro radius while the owner moves."),
        spec!("Use Avoid", "Basic", UseAvoid, Bool, "Emergency disconnect on severe danger."),

        spec!("Attack Skill Reserve SP", "AutoSkill", AttackSkillReserveSP, Int { min: 0, max: 100, step: 5 }, "SP to keep in reserve for attack skills."),
        spec!("Auto Mob Mode", "AutoSkill", AutoMobMode, Enum(AUTO_MOB_OPTS), "When to switch to AoE mob attacks."),
        spec!("Auto Mob Count", "AutoSkill", AutoMobCount, Int { min: 1, max: 12, step: 1 }, "Monsters needed to trigger AoE."),
        spec!("Auto Combo Mode", "AutoSkill", AutoComboMode, Enum(AUTO_COMBO_OPTS), "When to use combo skills."),
        spec!("Auto Combo Spheres", "AutoSkill", AutoComboSpheres, Int { min: 0, max: 15, step: 1 }, "Spheres to keep for combos."),
        spec!("Homun-S Skill Chase", "AutoSkill", UseHomunSSkillChase, Bool, "Use Homun-S skills while chasing."),
        spec!("Homun-S Skill Attack", "AutoSkill", UseHomunSSkillAttack, Bool, "Use Homun-S skills while attacking."),
        spec!("Auto Skill Delay", "AutoSkill", AutoSkillDelay, Int { min: 100, max: 2000, step: 50 }, "Delay between auto skill casts (ms)."),
        spec!("AoE Maximize Targets", "AutoSkill", AoEMaximizeTargets, Bool, "Position AoE to hit the most targets."),
        spec!("AoE Reserve SP", "AutoSkill", AoEReserveSP, Bool, "Reserve SP for AoE skills."),
        spec!("Use Auto Pushback", "AutoSkill", UseAutoPushback, Enum(PUSHBACK_OPTS), "Knockback skill usage scope."),
        spec!("Attack Time Limit", "AutoSkill", AttackTimeLimit, Int { min: 1000, max: 30000, step: 500 }, "Give up an unreachable target after (ms)."),

        spec!("Follow Stay Back", "Walk", FollowStayBack, Int { min: 0, max: 6, step: 1 }, "Cells to keep behind the owner."),
        spec!("Rest X Offset", "Walk", RestXOff, Int { min: -6, max: 6, step: 1 }, "Rest cell X offset from owner."),
        spec!("Rest Y Offset", "Walk", RestYOff, Int { min: -6, max: 6, step: 1 }, "Rest cell Y offset from owner."),
        spec!("Do Not Use Rest", "Walk", DoNotUseRest, Bool, "Never sit to rest at the owner."),
        spec!("Spawn Delay", "Walk", SpawnDelay, Int { min: 0, max: 5000, step: 100 }, "Startup delay before acting (ms)."),
        spec!("Move Sticky", "Walk", MoveSticky, Bool, "Hold position tightly when idle."),
        spec!("Use Idle Walk", "Walk", UseIdleWalk, Enum(IDLE_WALK_OPTS), "Idle wander / route pattern."),
        spec!("Idle Walk SP %", "Walk", IdleWalkSP, Int { min: 0, max: 100, step: 5 }, "Only idle-walk above this SP percent."),
        spec!("Idle Walk Distance", "Walk", IdleWalkDistance, Int { min: 1, max: 10, step: 1 }, "Idle-walk radius."),
        spec!("Chase SP Pause", "Walk", ChaseSPPause, Bool, "Pause chasing when SP is low."),

        spec!("Use Offensive Buff", "Autobuff", UseOffensiveBuff, Enum(BUFF_WHEN_OPTS), "When to cast offensive buffs."),
        spec!("Use Defensive Buff", "Autobuff", UseDefensiveBuff, Enum(BUFF_WHEN_OPTS), "When to cast defensive buffs."),
        spec!("Def Buff Owner Mobbed", "Autobuff", DefensiveBuffOwnerMobbed, Int { min: 0, max: 12, step: 1 }, "Mob count that triggers owner defensive buffs."),
        spec!("Heal Owner HP %", "Autobuff", HealOwnerHP, Int { min: 0, max: 100, step: 5 }, "Heal the owner below this HP percent."),
        spec!("Heal Self HP %", "Autobuff", HealSelfHP, Int { min: 0, max: 100, step: 5 }, "Heal self below this HP percent."),
        spec!("Use Auto Heal", "Autobuff", UseAutoHeal, Enum(AUTO_HEAL_OPTS), "When to auto-heal."),
        spec!("Lava Slide Mode", "Autobuff", LavaSlideMode, Enum(GROUND_BUFF_OPTS), "When to lay Lava Slide."),
        spec!("Poison Mist Mode", "Autobuff", PoisonMistMode, Enum(GROUND_BUFF_OPTS), "When to lay Poison Mist."),

        spec!("Kite Monsters", "Kiting", KiteMonsters, Bool, "Kite monsters flagged for kiting."),
        spec!("Kite Step", "Kiting", KiteStep, Int { min: 1, max: 10, step: 1 }, "Cells to retreat per kite step."),
        spec!("Kite Threshold", "Kiting", KiteThreshold, Int { min: 1, max: 10, step: 1 }, "Distance that triggers a kite step."),
        spec!("Kite Bounds", "Kiting", KiteBounds, Int { min: 1, max: 14, step: 1 }, "Max distance to kite from the owner."),
        spec!("Kite Paranoid", "Kiting", KiteParanoid, Bool, "Kite more aggressively."),
        spec!("Force Kite", "Kiting", ForceKite, Bool, "Always kite regardless of tactics."),
        spec!("Flee HP %", "Kiting", FleeHP, Int { min: 0, max: 100, step: 5 }, "Flee below this HP percent."),

        spec!("Standby Friending", "Standby", StandbyFriending, Bool, "Enable friend gestures while in standby."),
        spec!("Defend Standby", "Standby", DefendStandby, Bool, "Defend self/owner while in standby."),
        spec!("Sticky Standby", "Standby", StickyStandby, Enum(STICKY_STANDBY_OPTS), "Persist standby across relog."),

        spec!("Use Berserk Mobbed", "Berserk", UseBerserkMobbed, Bool, "Enter berserk when mobbed."),
        spec!("Use Berserk Skill", "Berserk", UseBerserkSkill, Bool, "Enter berserk to use skills."),
        spec!("Use Berserk Attack", "Berserk", UseBerserkAttack, Bool, "Enter berserk to attack."),

        spec!("PVP Mode", "PVP", PVPmode, Bool, "Consult PVP tactics against players."),
    ]
}

fn merc_specs() -> Vec<FieldSpec<MercConfig>> {
    use Widget::*;
    vec![
        spec!("Aggro HP %", "Basic", AggroHP, Int { min: 0, max: 100, step: 5 }, "Only aggress while HP is above this percent."),
        spec!("Aggro SP %", "Basic", AggroSP, Int { min: 0, max: 100, step: 5 }, "Only aggress while SP is above this percent."),
        spec!("Use Skill Only", "Basic", UseSkillOnly, Enum(USE_SKILL_ONLY_OPTS), "When attack skills may be used."),
        spec!("Use Attack Skill", "Basic", UseAttackSkill, Bool, "Enable attack-skill selection."),
        spec!("Opportunistic Target", "Basic", OpportunisticTargeting, Bool, "Re-target a closer/weaker monster mid-fight."),
        spec!("Do Not Chase", "Basic", DoNotChase, Bool, "Never leave attack range to chase."),
        spec!("Super Passive", "Basic", SuperPassive, Bool, "Never attack unless commanded."),
        spec!("Auto Detect Plant", "Basic", AutoDetectPlant, Bool, "Skip immobile plant-type monsters."),
        spec!("Tank Monster Limit", "Basic", TankMonsterLimit, Int { min: 0, max: 12, step: 1 }, "Max monsters to tank at once."),
        spec!("Stationary Aggro Dist", "Basic", StationaryAggroDist, Int { min: 1, max: 14, step: 1 }, "Aggro radius while the owner is still."),
        spec!("Mobile Aggro Dist", "Basic", MobileAggroDist, Int { min: 1, max: 14, step: 1 }, "Aggro radius while the owner moves."),

        spec!("Auto Mob Mode", "AutoSkill", AutoMobMode, Enum(AUTO_MOB_OPTS), "When to switch to AoE mob attacks."),
        spec!("Auto Mob Count", "AutoSkill", AutoMobCount, Int { min: 1, max: 12, step: 1 }, "Monsters needed to trigger AoE."),
        spec!("Auto Skill Delay", "AutoSkill", AutoSkillDelay, Int { min: 100, max: 750, step: 50 }, "Delay between auto skill casts (ms)."),
        spec!("Use Auto Pushback", "AutoSkill", UseAutoPushback, Enum(PUSHBACK_OPTS), "Knockback skill usage scope."),
        spec!("Auto Pushback Threshold", "AutoSkill", AutoPushbackThreshold, Int { min: 1, max: 6, step: 1 }, "Distance that triggers pushback."),
        spec!("Attack Time Limit", "AutoSkill", AttackTimeLimit, Int { min: 1000, max: 30000, step: 500 }, "Give up an unreachable target after (ms)."),

        spec!("Use Offensive Buff", "Autobuff", UseOffensiveBuff, Enum(BUFF_WHEN_OPTS), "When to cast offensive buffs."),
        spec!("Use Defensive Buff", "Autobuff", UseDefensiveBuff, Enum(BUFF_WHEN_OPTS), "When to cast defensive buffs."),
        spec!("Use Provoke Owner", "Autobuff", UseProvokeOwner, Enum(BUFF_WHEN_OPTS), "Provoke the owner's attacker."),
        spec!("Use Provoke Self", "Autobuff", UseProvokeSelf, Enum(BUFF_WHEN_OPTS), "Provoke own attacker."),
        spec!("Use Sacrifice Owner", "Autobuff", UseSacrificeOwner, Enum(BUFF_WHEN_OPTS), "Cast Sacrifice on the owner."),
        spec!("Use Auto Mag", "Autobuff", UseAutoMag, Enum(BUFF_WHEN_OPTS), "Cast Magnificat."),
        spec!("Use Auto Sight", "Autobuff", UseAutoSight, Enum(BUFF_WHEN_OPTS), "Cast Sight to reveal hidden foes."),
        spec!("Use Blessing Owner", "Autobuff", UseBlessingOwner, Enum(BUFF_WHEN_OPTS), "Bless the owner."),
        spec!("Use Blessing Self", "Autobuff", UseBlessingSelf, Enum(BUFF_WHEN_OPTS), "Bless self."),
        spec!("Use IncAgi Owner", "Autobuff", UseIncAgiOwner, Enum(BUFF_WHEN_OPTS), "Increase Agility on the owner."),
        spec!("Use IncAgi Self", "Autobuff", UseIncAgiSelf, Enum(BUFF_WHEN_OPTS), "Increase Agility on self."),
        spec!("Use Kyrie Owner", "Autobuff", UseKyrieOwner, Enum(BUFF_WHEN_OPTS), "Kyrie Eleison on the owner."),
        spec!("Use Kyrie Self", "Autobuff", UseKyrieSelf, Enum(BUFF_WHEN_OPTS), "Kyrie Eleison on self."),

        spec!("Follow Stay Back", "Walk", FollowStayBack, Int { min: 0, max: 6, step: 1 }, "Cells to keep behind the owner."),
        spec!("Do Not Use Rest", "Walk", DoNotUseRest, Bool, "Never sit to rest at the owner."),
        spec!("Use Idle Walk", "Walk", UseIdleWalk, Enum(IDLE_WALK_OPTS), "Idle wander / route pattern."),
        spec!("Idle Walk Distance", "Walk", IdleWalkDistance, Int { min: 1, max: 10, step: 1 }, "Idle-walk radius."),

        spec!("Kite Monsters", "Kiting", KiteMonsters, Bool, "Kite monsters flagged for kiting."),
        spec!("Kite Bounds", "Kiting", KiteBounds, Int { min: 1, max: 14, step: 1 }, "Max distance to kite from the owner."),
        spec!("Force Kite", "Kiting", ForceKite, Bool, "Always kite regardless of tactics."),
        spec!("Flee HP %", "Kiting", FleeHP, Int { min: 0, max: 100, step: 5 }, "Flee below this HP percent."),

        spec!("Sticky Standby", "Standby", StickyStandby, Enum(STICKY_STANDBY_OPTS), "Persist standby across relog."),
        spec!("PVP Mode", "PVP", PVPmode, Bool, "Consult PVP tactics against players."),
    ]
}

enum Row<'a> {
    Header(&'a str),
    Field(usize),
}

fn build_rows<'a, C>(specs: &'a [FieldSpec<C>]) -> Vec<Row<'a>> {
    let mut rows = Vec::new();
    let mut cur = "";
    for (i, s) in specs.iter().enumerate() {
        if s.category != cur {
            cur = s.category;
            rows.push(Row::Header(s.category));
        }
        rows.push(Row::Field(i));
    }
    rows
}

pub struct CompanionAiConfigWindow {
    pub has_grf_textures: bool,
    visible: bool,
    tab: usize,
    scroll_offset: usize,
    tactic_sel: usize,
}

impl Default for CompanionAiConfigWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl CompanionAiConfigWindow {
    pub fn new() -> Self {
        Self {
            has_grf_textures: false,
            visible: false,
            tab: 0,
            scroll_offset: 0,
            tactic_sel: 0,
        }
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }
    pub fn is_visible(&self) -> bool {
        self.visible
    }
    pub fn set_visible(&mut self, value: bool) {
        self.visible = value;
    }

    pub fn build(
        &mut self,
        ui: &mut UiFrame,
        config: &mut CompanionAiConfig,
    ) -> Vec<GameEvent> {
        if !self.visible {
            return Vec::new();
        }
        let prev_grf = ui.has_grf_textures;
        ui.has_grf_textures = self.has_grf_textures;
        let grf = self.has_grf_textures;
        let mut events = Vec::new();
        let tc = text_color(grf);

        let win = ui.window_at(COMPANION_AI_CONFIG_WINDOW_ID, WIN_W, WIN_H, TITLE_H, 160.0, 90.0);
        let (x, y) = (win.x, win.y);
        ui.interact(COMPANION_AI_CONFIG_WINDOW_ID, Rect::new(x, y, WIN_W, WIN_H));

        draw_titlebar(ui, x, y, WIN_W, TITLE_H, grf);
        ui.text(x + 16.0, y + 13.0, "Companion AI Settings", tc);

        let sys_w = 11.0;
        let close_rect = Rect::new(x + WIN_W - 3.0 - sys_w, y + 3.0, sys_w, sys_w);
        let close_resp = ui.interact(CLOSE_BTN_ID, close_rect);
        if close_resp.hovered() {
            ui.any_interactive_hovered = true;
        }
        if close_resp.clicked() {
            self.visible = false;
        }
        draw_sys_button(
            ui,
            close_rect,
            (sys_w, sys_w),
            close_resp.hovered(),
            grf,
            CLOSE_ON_TEX,
            CLOSE_OFF_TEX,
            Some('x'),
        );

        draw_container(ui, x, y + TITLE_H, WIN_W, WIN_H - TITLE_H, grf);

        // Tab bar.
        let tab_w = WIN_W / TABS.len() as f32;
        let tab_y = y + TITLE_H;
        for (i, name) in TABS.iter().enumerate() {
            let tab_rect = Rect::new(x + i as f32 * tab_w, tab_y, tab_w, TAB_H);
            let resp = ui.interact(WidgetId(TAB_BASE_ID.0 + i as u32), tab_rect);
            if resp.hovered() {
                ui.any_interactive_hovered = true;
            }
            if resp.clicked() && self.tab != i {
                self.tab = i;
                self.scroll_offset = 0;
            }
            let bg = if i == self.tab {
                [0.72, 0.79, 0.93, 1.0]
            } else if grf {
                [0.70, 0.70, 0.74, 1.0]
            } else {
                [0.95, 0.95, 0.96, 1.0]
            };
            push_quad(ui, tab_rect.x, tab_rect.y, tab_w - 1.0, TAB_H - 1.0, bg);
            ui.text_centered(tab_rect.x, tab_rect.y + BASELINE, tab_w, name, tc);
        }

        let content_y = tab_y + TAB_H + 2.0;
        let footer_y = y + WIN_H - FOOTER_H;
        let content_h = footer_y - content_y - 2.0;

        match self.tab {
            0 => {
                let specs = homun_specs();
                self.render_fields(ui, &mut config.homunculus, &specs, x, content_y, content_h, tc, grf);
            }
            1 => {
                let specs = merc_specs();
                self.render_fields(ui, &mut config.mercenary, &specs, x, content_y, content_h, tc, grf);
            }
            2 => {
                self.render_tactics(ui, &mut config.homunculus_tactics, x, content_y, tc, grf);
            }
            3 => {
                self.render_tactics(ui, &mut config.mercenary_tactics, x, content_y, tc, grf);
            }
            _ => {
                ui.text(x + PAD, content_y + 24.0, "This tab arrives with a later tier.", colors::YELLOW);
            }
        }

        // Footer buttons.
        let bw = 84.0;
        let by = footer_y + 3.0;
        if ui.text_button(APPLY_BTN_ID, Rect::new(x + PAD, by, bw, 18.0), "Apply").clicked() {
            events.push(GameEvent::SaveCompanionAiConfig);
        }
        if ui.text_button(REVERT_BTN_ID, Rect::new(x + PAD + bw + 6.0, by, bw, 18.0), "Revert").clicked() {
            events.push(GameEvent::RevertCompanionAiConfig);
        }
        if ui.text_button(RESET_BTN_ID, Rect::new(x + PAD + (bw + 6.0) * 2.0, by, bw, 18.0), "Defaults").clicked() {
            events.push(GameEvent::ResetCompanionAiConfig);
        }

        ui.has_grf_textures = prev_grf;
        events
    }

    #[allow(clippy::too_many_arguments)]
    fn render_fields<C>(
        &mut self,
        ui: &mut UiFrame,
        cfg: &mut C,
        specs: &[FieldSpec<C>],
        x: f32,
        y: f32,
        h: f32,
        tc: [f32; 4],
        grf: bool,
    ) {
        let rows = build_rows(specs);
        let visible_rows = (h / ROW_H) as usize;
        let max_scroll = rows.len().saturating_sub(visible_rows);
        let content_rect = Rect::new(x, y, WIN_W - SCROLLBAR_W - 2.0, h);
        self.scroll_offset = scrollbar::scrollbar(
            ui,
            ScrollbarIds { up: SCROLL_UP_ID, down: SCROLL_DOWN_ID, thumb: SCROLL_THUMB_ID },
            self.scroll_offset,
            visible_rows,
            max_scroll,
            content_rect,
            x + WIN_W - SCROLLBAR_W - 1.0,
            y,
            h,
        );

        let label_x = x + PAD + 8.0;
        let widget_x = x + WIN_W - SCROLLBAR_W - 6.0 - 120.0;
        let widget_w = 120.0;
        let mut ry = y;
        for (slot, row) in rows.iter().skip(self.scroll_offset).take(visible_rows).enumerate() {
            match row {
                Row::Header(name) => {
                    push_quad(ui, x + 2.0, ry, WIN_W - SCROLLBAR_W - 4.0, ROW_H - 1.0,
                        [0.51, 0.58, 0.78, 1.0]);
                    ui.text(x + PAD, ry + BASELINE, name, [1.0, 1.0, 1.0, 1.0]);
                }
                Row::Field(i) => {
                    let s = &specs[*i];
                    ui.text(label_x, ry + BASELINE, s.label, tc);
                    let wrect = Rect::new(widget_x, ry + 1.0, widget_w, ROW_H - 3.0);
                    let cur = (s.get)(cfg);
                    let new = self.render_widget(ui, slot, s, cur, wrect, tc, grf);
                    if new != cur {
                        (s.set)(cfg, new);
                    }
                    let hover_rect = Rect::new(label_x, ry, widget_x - label_x, ROW_H);
                    if hover_rect.contains(ui.ctx.mouse_x, ui.ctx.mouse_y) && !s.tip.is_empty() {
                        ui.tooltip(ui.ctx.mouse_x, ui.ctx.mouse_y, s.tip);
                    }
                }
            }
            ry += ROW_H;
        }
    }

    fn render_tactics(
        &mut self,
        ui: &mut UiFrame,
        rows: &mut Vec<Tactic>,
        x: f32,
        y: f32,
        tc: [f32; 4],
        grf: bool,
    ) {
        if rows.is_empty() {
            rows.push(Tactic::default_row());
        }
        self.tactic_sel = self.tactic_sel.min(rows.len() - 1);

        // Selector row: ◀ id:name ▶  Add  Del
        let bw = 22.0;
        let mut cx = x + PAD;
        if ui.text_button(TACT_PREV_ID, Rect::new(cx, y, bw, 16.0), "<").clicked() && self.tactic_sel > 0 {
            self.tactic_sel -= 1;
        }
        cx += bw + 2.0;
        let sel = &rows[self.tactic_sel];
        let label = format!("#{}  {}", sel.id, if sel.name.is_empty() { "(unnamed)" } else { &sel.name });
        let name_w = WIN_W - PAD * 2.0 - bw * 2.0 - 4.0 - 120.0;
        push_quad(ui, cx, y, name_w, 16.0, [0.95, 0.95, 0.96, 1.0]);
        ui.text(cx + 4.0, y + BASELINE, &label, tc);
        cx += name_w + 2.0;
        if ui.text_button(TACT_NEXT_ID, Rect::new(cx, y, bw, 16.0), ">").clicked()
            && self.tactic_sel + 1 < rows.len()
        {
            self.tactic_sel += 1;
        }
        cx += bw + 4.0;
        if ui.text_button(TACT_ADD_ID, Rect::new(cx, y, 56.0, 16.0), "Add").clicked() {
            let mut t = Tactic::default_row();
            t.id = next_free_class_id(rows);
            t.name = "New".to_string();
            rows.push(t);
            self.tactic_sel = rows.len() - 1;
        }
        cx += 58.0;
        if ui.text_button(TACT_DEL_ID, Rect::new(cx, y, 56.0, 16.0), "Del").clicked()
            && rows[self.tactic_sel].id != 0
            && rows[self.tactic_sel].id != 13
        {
            rows.remove(self.tactic_sel);
            self.tactic_sel = self.tactic_sel.saturating_sub(1);
        }

        let cols = tactic_cols();
        let sel = &mut rows[self.tactic_sel];
        let mut ry = y + 22.0;
        for (slot, spec) in cols.iter().enumerate() {
            ui.text(x + PAD + 8.0, ry + BASELINE, spec.label, tc);
            let widget_x = x + WIN_W - SCROLLBAR_W - 6.0 - 130.0;
            let wrect = Rect::new(widget_x, ry + 1.0, 130.0, ROW_H - 3.0);
            let cur = (spec.get)(sel);
            let new = self.render_widget(ui, slot, spec, cur, wrect, tc, grf);
            if new != cur {
                (spec.set)(sel, new);
            }
            ry += ROW_H;
        }
    }

    fn render_widget<C>(
        &self,
        ui: &mut UiFrame,
        slot: usize,
        spec: &FieldSpec<C>,
        cur: i32,
        rect: Rect,
        tc: [f32; 4],
        grf: bool,
    ) -> i32 {
        let base = ROW_WIDGET_BASE + slot as u32 * 3;
        match spec.widget {
            Widget::Bool => {
                let sz = 12.0;
                let cb = Rect::new(rect.x + rect.w - sz, rect.y + 1.0, sz, sz);
                let resp = ui.interact(WidgetId(base), cb);
                if resp.hovered() {
                    ui.any_interactive_hovered = true;
                }
                push_quad(ui, cb.x, cb.y, sz, sz, [0.85, 0.85, 0.85, 1.0]);
                if cur != 0 {
                    push_quad(ui, cb.x + 3.0, cb.y + 3.0, sz - 6.0, sz - 6.0, colors::GREEN);
                }
                if resp.clicked() {
                    return if cur != 0 { 0 } else { 1 };
                }
                cur
            }
            Widget::Int { min, max, step } => {
                let bw = 16.0;
                let minus = Rect::new(rect.x, rect.y, bw, rect.h);
                let plus = Rect::new(rect.x + rect.w - bw, rect.y, bw, rect.h);
                let m = ui.interact(WidgetId(base), minus);
                let p = ui.interact(WidgetId(base + 1), plus);
                if m.hovered() || p.hovered() {
                    ui.any_interactive_hovered = true;
                }
                push_quad(ui, minus.x, minus.y, bw, rect.h, [0.35, 0.35, 0.40, 1.0]);
                push_quad(ui, plus.x, plus.y, bw, rect.h, [0.35, 0.35, 0.40, 1.0]);
                ui.text_centered(minus.x, rect.y + BASELINE - 2.0, bw, "-", tc);
                ui.text_centered(plus.x, rect.y + BASELINE - 2.0, bw, "+", tc);
                ui.text_centered(rect.x + bw, rect.y + BASELINE - 2.0, rect.w - 2.0 * bw, &cur.to_string(), tc);
                let mut v = cur;
                if m.clicked() {
                    v = (v - step).max(min);
                }
                if p.clicked() {
                    v = (v + step).min(max);
                }
                let _ = grf;
                v
            }
            Widget::Enum(opts) => {
                let resp = ui.interact(WidgetId(base), rect);
                if resp.hovered() {
                    ui.any_interactive_hovered = true;
                }
                push_quad(ui, rect.x, rect.y, rect.w, rect.h, [0.30, 0.30, 0.40, 1.0]);
                let label = opts.iter().find(|(v, _)| *v == cur).map(|(_, l)| *l).unwrap_or("?");
                ui.text_centered(rect.x, rect.y + BASELINE - 2.0, rect.w, label, tc);
                if resp.clicked() {
                    let idx = opts.iter().position(|(v, _)| *v == cur).unwrap_or(0);
                    let next = (idx + 1) % opts.len();
                    return opts[next].0;
                }
                cur
            }
        }
    }
}

fn next_free_class_id(rows: &[Tactic]) -> u32 {
    let mut id = 1001;
    while rows.iter().any(|t| t.id == id) {
        id += 1;
    }
    id
}

fn push_quad(ui: &mut UiFrame, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) {
    let (v, i) = draw::quad_vertices(x, y, w, h, color);
    ui.draw_calls.push(DrawCall {
        vertices: v.to_vec(),
        indices: i.to_vec(),
        texture: TextureRef::White,
    });
}

impl Window for CompanionAiConfigWindow {
    fn has_grf_textures(&self) -> bool {
        self.has_grf_textures
    }
    fn set_has_grf_textures(&mut self, value: bool) {
        self.has_grf_textures = value;
    }
    fn grf_texture_paths() -> Vec<&'static str> {
        let mut paths = vec![TITLEBAR_TEX, CLOSE_OFF_TEX, CLOSE_ON_TEX];
        paths.extend(scrollbar::grf_texture_paths());
        paths
    }
}
