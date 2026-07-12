use std::collections::{HashSet, VecDeque};

use crate::context::{ActorView, AiContext, AiIntent, Motion};

mod target;

const TICK_INTERVAL_MS: u32 = 140;
const OUT_OF_SIGHT_DISTANCE: i32 = 20;
const FOLLOW_DISTANCE: i32 = 3;
const MOVE_SPLIT_DISTANCE: i32 = 15;
const MOVE_ABORT_DISTANCE: i32 = 24;
const RESERVED_QUEUE_CAP: usize = 10;
const DEFAULT_ATTACK_DELAY_MS: u32 = 1000;
const CHASE_GIVEUP_LIMIT: u32 = 8;
const TANK_HIT_INTERVAL_MS: u32 = 1500;

const HFLI_MOON: u16 = 8009;
const HVAN_CAPRICE: u16 = 8013;

/// The offensive auto-attack skill for a 1st-gen homunculus type, matching the
/// reference `GetAtkSkill`: Vanilmirth casts Caprice, Filir casts Moonlight;
/// Lif and Amistr have none (they melee). Homunculus-S are excluded (their
/// combo/minion skills are not modelled) and mercenaries return none here.
fn main_attack_skill(companion_type: u16, is_mercenary: bool) -> Option<u16> {
    if is_mercenary || companion_type == 0 || companion_type >= 17 {
        return None;
    }
    match companion_type % 4 {
        0 => Some(HVAN_CAPRICE),
        3 => Some(HFLI_MOON),
        _ => None,
    }
}

/// Per-skill reuse cooldown (ms) from the reference skill table; server enforces
/// the same, so tracking it avoids spamming casts the server would bounce.
fn reuse_delay_ms(skill_id: u16, level: u8) -> u32 {
    match skill_id {
        HFLI_MOON => 2000,
        HVAN_CAPRICE => 2000 + level as u32 * 200,
        _ => 0,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiState {
    Idle,
    Follow,
    Chase,
    Attack,
    TankChase,
    Tank,
    MoveCmd,
    StopCmd,
    AttackObjectCmd,
    AttackAreaCmd,
    PatrolCmd,
    HoldCmd,
    SkillObjectCmd,
    SkillAreaCmd,
    FollowCmd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandKind {
    Move,
    Stop,
    AttackObject,
    AttackArea,
    Patrol,
    Hold,
    SkillObject,
    SkillArea,
    Follow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OwnerCommand {
    pub kind: CommandKind,
    pub x: i32,
    pub y: i32,
    pub skill_id: u16,
    pub skill_level: u8,
    pub target_gid: u32,
}

impl OwnerCommand {
    pub fn move_to(x: i32, y: i32) -> Self {
        Self::with(CommandKind::Move, x, y, 0, 0, 0)
    }
    pub fn attack(target_gid: u32) -> Self {
        Self::with(CommandKind::AttackObject, 0, 0, 0, 0, target_gid)
    }
    pub fn stop() -> Self {
        Self::with(CommandKind::Stop, 0, 0, 0, 0, 0)
    }
    pub fn hold() -> Self {
        Self::with(CommandKind::Hold, 0, 0, 0, 0, 0)
    }
    pub fn follow() -> Self {
        Self::with(CommandKind::Follow, 0, 0, 0, 0, 0)
    }
    pub fn patrol(x: i32, y: i32) -> Self {
        Self::with(CommandKind::Patrol, x, y, 0, 0, 0)
    }
    pub fn skill_object(skill_id: u16, level: u8, target_gid: u32) -> Self {
        Self::with(CommandKind::SkillObject, 0, 0, skill_id, level, target_gid)
    }
    pub fn skill_area(skill_id: u16, level: u8, x: i32, y: i32) -> Self {
        Self::with(CommandKind::SkillArea, x, y, skill_id, level, 0)
    }

    fn with(kind: CommandKind, x: i32, y: i32, skill_id: u16, skill_level: u8, target_gid: u32) -> Self {
        Self {
            kind,
            x,
            y,
            skill_id,
            skill_level,
            target_gid,
        }
    }
}

pub struct CompanionAi {
    state: AiState,
    enemy: u32,
    dest_x: i32,
    dest_y: i32,
    patrol_x: i32,
    patrol_y: i32,
    skill: u16,
    skill_level: u8,
    is_mercenary: bool,
    msg: Option<OwnerCommand>,
    res_msg: Option<OwnerCommand>,
    res_cmd_list: VecDeque<OwnerCommand>,
    tick_accum_ms: u32,
    clock_ms: u32,
    next_attack_ms: u32,
    standby: bool,
    chase_giveup: u32,
    tank_hit_ms: u32,
    unreachable: HashSet<u32>,
    my_skill_used_count: u32,
    skill_count_enemy: u32,
    auto_skill_ready_ms: u32,
    skill_cooldown: Option<(u16, u32)>,
}

impl CompanionAi {
    pub fn new(is_mercenary: bool) -> Self {
        Self {
            state: AiState::Idle,
            enemy: 0,
            dest_x: 0,
            dest_y: 0,
            patrol_x: 0,
            patrol_y: 0,
            skill: 0,
            skill_level: 0,
            is_mercenary,
            msg: None,
            res_msg: None,
            res_cmd_list: VecDeque::new(),
            tick_accum_ms: 0,
            clock_ms: 0,
            next_attack_ms: 0,
            standby: false,
            chase_giveup: 0,
            tank_hit_ms: 0,
            unreachable: HashSet::new(),
            my_skill_used_count: 0,
            skill_count_enemy: 0,
            auto_skill_ready_ms: 0,
            skill_cooldown: None,
        }
    }

    pub fn state(&self) -> AiState {
        self.state
    }

    pub fn set_standby(&mut self, standby: bool) {
        self.standby = standby;
    }

    pub(crate) fn is_unreachable(&self, gid: u32) -> bool {
        self.unreachable.contains(&gid)
    }

    fn aggro_flag(&self, ctx: &AiContext) -> i32 {
        if ctx.hp_pct() > ctx.params.aggro_hp
            && ctx.sp_pct() > ctx.params.aggro_sp
            && !self.standby
        {
            1
        } else {
            0
        }
    }

    pub fn has_pending_command(&self) -> bool {
        self.msg.is_some()
    }

    /// Direct command (Alt+click): clears any queued reserved commands.
    pub fn push_command(&mut self, cmd: OwnerCommand) {
        self.msg = Some(cmd);
    }

    /// Reserved command (Shift+Alt+click): queued behind the current action.
    pub fn push_reserved(&mut self, cmd: OwnerCommand) {
        self.res_msg = Some(cmd);
    }

    pub fn tick(&mut self, dt: f32, ctx: &AiContext) -> Vec<AiIntent> {
        let mut out = Vec::new();
        self.tick_accum_ms += (dt * 1000.0) as u32;
        // Cap catch-up so a long stall can't spin the state machine hundreds of times.
        let mut steps = 0;
        while self.tick_accum_ms >= TICK_INTERVAL_MS && steps < 4 {
            self.tick_accum_ms -= TICK_INTERVAL_MS;
            self.clock_ms = self.clock_ms.wrapping_add(TICK_INTERVAL_MS);
            self.step(ctx, &mut out);
            steps += 1;
        }
        if self.tick_accum_ms >= TICK_INTERVAL_MS {
            self.tick_accum_ms = 0;
        }
        out
    }

    fn step(&mut self, ctx: &AiContext, out: &mut Vec<AiIntent>) {
        let msg = self.msg.take();
        let rmsg = self.res_msg.take();
        match msg {
            None => {
                if let Some(rmsg) = rmsg {
                    if self.res_cmd_list.len() < RESERVED_QUEUE_CAP {
                        self.res_cmd_list.push_back(rmsg);
                    }
                }
            }
            Some(cmd) => {
                self.res_cmd_list.clear();
                self.process_command(cmd, ctx, out);
            }
        }

        match self.state {
            AiState::Idle => self.on_idle(ctx, out),
            AiState::Chase => self.on_chase(ctx, out),
            AiState::Attack => self.on_attack(ctx, out),
            AiState::TankChase => self.on_tank_chase(ctx, out),
            AiState::Tank => self.on_tank(ctx, out),
            AiState::Follow => self.on_follow(ctx, out),
            AiState::MoveCmd => self.on_move_cmd(ctx),
            AiState::AttackAreaCmd => self.on_attack_area_cmd(ctx, out),
            AiState::PatrolCmd => self.on_patrol_cmd(ctx, out),
            AiState::HoldCmd => self.on_hold_cmd(ctx, out),
            AiState::SkillAreaCmd => self.on_skill_area_cmd(ctx, out),
            AiState::FollowCmd => self.on_follow_cmd(ctx, out),
            AiState::StopCmd | AiState::AttackObjectCmd | AiState::SkillObjectCmd => {}
        }
    }

    // ---- command processing --------------------------------------------

    fn process_command(&mut self, cmd: OwnerCommand, ctx: &AiContext, out: &mut Vec<AiIntent>) {
        self.standby = false;
        match cmd.kind {
            CommandKind::Move => self.on_move_command(cmd.x, cmd.y, ctx, out),
            CommandKind::Stop => self.on_stop_command(ctx, out),
            CommandKind::AttackObject => {
                self.skill = 0;
                self.enemy = cmd.target_gid;
                self.state = AiState::Chase;
            }
            CommandKind::AttackArea => {
                if cmd.x != self.dest_x || cmd.y != self.dest_y || ctx.my_motion != Motion::Move {
                    self.emit_move(cmd.x, cmd.y, ctx, out);
                }
                self.dest_x = cmd.x;
                self.dest_y = cmd.y;
                self.enemy = 0;
                self.state = AiState::AttackAreaCmd;
            }
            CommandKind::Patrol => {
                self.patrol_x = ctx.my_x;
                self.patrol_y = ctx.my_y;
                self.dest_x = cmd.x;
                self.dest_y = cmd.y;
                self.emit_move(cmd.x, cmd.y, ctx, out);
                self.state = AiState::PatrolCmd;
            }
            CommandKind::Hold => {
                self.dest_x = 0;
                self.dest_y = 0;
                self.enemy = 0;
                self.state = AiState::HoldCmd;
            }
            CommandKind::SkillObject => {
                self.skill_level = cmd.skill_level;
                self.skill = cmd.skill_id;
                self.enemy = cmd.target_gid;
                self.state = AiState::Chase;
            }
            CommandKind::SkillArea => {
                self.emit_move(cmd.x, cmd.y, ctx, out);
                self.dest_x = cmd.x;
                self.dest_y = cmd.y;
                self.skill_level = cmd.skill_level;
                self.skill = cmd.skill_id;
                self.state = AiState::SkillAreaCmd;
            }
            CommandKind::Follow => self.on_follow_command(ctx, out),
        }
    }

    fn on_move_command(&mut self, mut x: i32, mut y: i32, ctx: &AiContext, out: &mut Vec<AiIntent>) {
        if x == self.dest_x && y == self.dest_y && ctx.my_motion == Motion::Move {
            return;
        }
        if (x - ctx.my_x).abs() + (y - ctx.my_y).abs() > MOVE_SPLIT_DISTANCE {
            self.res_cmd_list.push_front(OwnerCommand::move_to(x, y));
            x = (x + ctx.my_x) / 2;
            y = (y + ctx.my_y) / 2;
        }
        self.emit_move(x, y, ctx, out);
        self.state = AiState::MoveCmd;
        self.dest_x = x;
        self.dest_y = y;
        self.enemy = 0;
        self.skill = 0;
    }

    fn on_stop_command(&mut self, ctx: &AiContext, out: &mut Vec<AiIntent>) {
        if ctx.my_motion != Motion::Stand {
            out.push(AiIntent::MoveTo {
                x: ctx.my_x,
                y: ctx.my_y,
            });
        }
        self.state = AiState::Idle;
        self.dest_x = 0;
        self.dest_y = 0;
        self.enemy = 0;
        self.skill = 0;
    }

    fn on_follow_command(&mut self, ctx: &AiContext, out: &mut Vec<AiIntent>) {
        if self.state != AiState::FollowCmd {
            out.push(AiIntent::MoveToOwner);
            self.state = AiState::FollowCmd;
            self.standby = true;
            if let Some((ox, oy)) = ctx.owner_pos {
                self.dest_x = ox;
                self.dest_y = oy;
            }
            self.enemy = 0;
            self.skill = 0;
        } else {
            self.state = AiState::Idle;
            self.standby = false;
            self.enemy = 0;
            self.skill = 0;
        }
    }

    // ---- state handlers -------------------------------------------------

    fn on_idle(&mut self, ctx: &AiContext, out: &mut Vec<AiIntent>) {
        if let Some(cmd) = self.res_cmd_list.pop_front() {
            self.process_command(cmd, ctx, out);
            return;
        }
        self.enemy = 0;
        self.skill = 0;
        if !ctx.params.super_passive {
            let object = self.select_enemy(ctx, &self.friend_targets(ctx), None);
            if object != 0 {
                self.state = AiState::Chase;
                self.enemy = object;
                return;
            }
            let aggro = self.aggro_flag(ctx);
            let object = self.select_enemy(ctx, &self.enemy_list(ctx, aggro), None);
            if object != 0 {
                self.state = AiState::Chase;
                self.enemy = object;
                return;
            }
            if aggro == 1 && self.tank_count(ctx) < ctx.params.tank_monster_limit {
                let object = self.select_enemy(ctx, &self.enemy_list(ctx, -1), None);
                if object != 0 {
                    self.state = AiState::TankChase;
                    self.enemy = object;
                    return;
                }
            }
        }
        let distance = self.distance_from_owner(ctx);
        if distance > FOLLOW_DISTANCE || distance == -1 {
            self.state = AiState::Follow;
        }
    }

    fn on_follow(&mut self, ctx: &AiContext, out: &mut Vec<AiIntent>) {
        let d = self.distance_from_owner(ctx);
        if d != -1 && d <= FOLLOW_DISTANCE {
            self.state = AiState::Idle;
        } else if ctx.my_motion == Motion::Stand {
            out.push(AiIntent::MoveToOwner);
        }
    }

    fn on_chase(&mut self, ctx: &AiContext, out: &mut Vec<AiIntent>) {
        if self.is_out_of_sight(ctx, self.enemy) || !self.is_not_ks(ctx, self.enemy) {
            self.drop_enemy_to_idle();
            return;
        }
        if ctx.my_motion != Motion::Move {
            if self.chase_giveup > CHASE_GIVEUP_LIMIT {
                self.unreachable.insert(self.enemy);
                self.chase_giveup = 0;
                self.drop_enemy_to_idle();
                return;
            }
            self.chase_giveup += 1;
        }
        if ctx.params.opportunistic && self.skill == 0 && !ctx.params.super_passive {
            let aggro = self.aggro_flag(ctx);
            let object =
                self.select_enemy(ctx, &self.enemy_list(ctx, aggro), Some(self.enemy));
            if object != 0 {
                self.enemy = object;
            }
        }
        if self.is_in_attack_sight(ctx, self.enemy) {
            self.state = AiState::Attack;
            self.chase_giveup = 0;
            return;
        }
        if self.chase_blocked(ctx, self.enemy) {
            return;
        }
        if let Some((ex, ey)) = self.actor_pos(ctx, self.enemy) {
            if self.dest_x != ex || self.dest_y != ey {
                self.dest_x = ex;
                self.dest_y = ey;
                self.emit_move(ex, ey, ctx, out);
            }
        }
    }

    fn on_tank_chase(&mut self, ctx: &AiContext, out: &mut Vec<AiIntent>) {
        if self.is_out_of_sight(ctx, self.enemy)
            || (self.chase_giveup > CHASE_GIVEUP_LIMIT && ctx.my_motion != Motion::Move)
        {
            if self.chase_giveup > CHASE_GIVEUP_LIMIT {
                self.unreachable.insert(self.enemy);
            }
            self.chase_giveup = 0;
            self.drop_enemy_to_idle();
            return;
        }
        if self.is_in_attack_sight(ctx, self.enemy) {
            self.chase_giveup = 0;
            if self.is_not_ks(ctx, self.enemy) {
                self.state = AiState::Tank;
                self.on_tank(ctx, out);
            } else {
                self.drop_enemy_to_idle();
            }
            return;
        }
        self.chase_giveup += 1;
        if !self.chase_blocked(ctx, self.enemy) {
            if let Some((ex, ey)) = self.actor_pos(ctx, self.enemy) {
                self.dest_x = ex;
                self.dest_y = ey;
                self.emit_move(ex, ey, ctx, out);
            }
        }
    }

    fn on_tank(&mut self, ctx: &AiContext, out: &mut Vec<AiIntent>) {
        if self.actor_motion(ctx, self.enemy) == Some(Motion::Dead)
            || self.is_out_of_sight(ctx, self.enemy)
        {
            self.drop_enemy_to_idle();
            return;
        }
        let targets_me = self.actor(ctx, self.enemy).and_then(|a| a.target_gid) == Some(ctx.my_gid);
        if !targets_me {
            if self.clock_ms >= self.tank_hit_ms.wrapping_add(TANK_HIT_INTERVAL_MS) {
                self.tank_hit_ms = self.clock_ms;
                if self.is_in_attack_sight(ctx, self.enemy) {
                    self.emit_attack(ctx, out);
                } else {
                    self.state = AiState::TankChase;
                }
            }
            return;
        }
        self.state = AiState::Idle;
    }

    fn drop_enemy_to_idle(&mut self) {
        self.state = AiState::Idle;
        self.enemy = 0;
        self.dest_x = 0;
        self.dest_y = 0;
    }

    /// Resolves the chase tactic column: `true` = do not close on this target.
    fn chase_blocked(&self, ctx: &AiContext, gid: u32) -> bool {
        use crate::consts::ChaseTactic;
        let Some(a) = self.actor(ctx, gid) else {
            return false;
        };
        match ctx.tactics.resolve(a.class_id as u32).chase {
            ChaseTactic::Always => false,
            ChaseTactic::Never => true,
            _ => ctx.params.do_not_chase,
        }
    }

    fn on_attack(&mut self, ctx: &AiContext, out: &mut Vec<AiIntent>) {
        if self.is_out_of_sight(ctx, self.enemy) {
            self.state = AiState::Idle;
            return;
        }
        if self.actor_motion(ctx, self.enemy) == Some(Motion::Dead) {
            self.state = AiState::Idle;
            return;
        }
        if !self.is_in_attack_sight(ctx, self.enemy) {
            self.state = AiState::Chase;
            if let Some((ex, ey)) = self.actor_pos(ctx, self.enemy) {
                self.dest_x = ex;
                self.dest_y = ey;
                self.emit_move(ex, ey, ctx, out);
            }
            return;
        }
        if self.enemy != self.skill_count_enemy {
            self.my_skill_used_count = 0;
            self.skill_count_enemy = self.enemy;
        }
        if self.skill != 0 {
            let target = self.enemy;
            out.push(AiIntent::SkillObject {
                skill_id: self.skill,
                level: self.skill_level,
                target_gid: target,
            });
            self.skill = 0;
            if self.is_mercenary || target == ctx.my_gid {
                self.drop_enemy_to_idle();
            }
        } else if let Some((skill_id, level)) = self.select_attack_skill(ctx) {
            out.push(AiIntent::SkillObject {
                skill_id,
                level,
                target_gid: self.enemy,
            });
            self.my_skill_used_count += 1;
            self.auto_skill_ready_ms = self
                .clock_ms
                .wrapping_add(ctx.params.auto_skill_delay.max(0) as u32);
            self.skill_cooldown =
                Some((skill_id, self.clock_ms.wrapping_add(reuse_delay_ms(skill_id, level))));
        } else {
            self.emit_attack(ctx, out);
        }
    }

    /// Picks the offensive attack skill to cast this tick, or `None` to melee,
    /// applying the reference gates: global skill delay, per-enemy cast count
    /// (tactic `skill` column), per-skill reuse cooldown, SP reserve and range.
    fn select_attack_skill(&self, ctx: &AiContext) -> Option<(u16, u8)> {
        use crate::tactics::SkillUse;
        if !ctx.params.use_attack_skill || self.clock_ms < self.auto_skill_ready_ms {
            return None;
        }
        let skill_id = main_attack_skill(ctx.companion_type, self.is_mercenary)?;
        let known = ctx.skills.iter().find(|s| s.id == skill_id && s.level > 0)?;

        let tactic = self
            .actor(ctx, self.enemy)
            .map(|a| ctx.tactics.resolve(a.class_id as u32))
            .unwrap_or_else(|| ctx.tactics.resolve(0));

        let level = match tactic.skill {
            SkillUse::Never => return None,
            SkillUse::Always => known.level,
            SkillUse::Times(n) => {
                if self.my_skill_used_count >= n as u32 {
                    return None;
                }
                known.level
            }
            SkillUse::OnceAtLevel(l) => {
                if self.my_skill_used_count >= 1 {
                    return None;
                }
                l.min(known.level)
            }
        };

        if let Some((cd_id, until)) = self.skill_cooldown {
            if cd_id == skill_id && self.clock_ms < until {
                return None;
            }
        }
        if known.range < self.distance_to(ctx, self.enemy) {
            return None;
        }
        let reserve = if tactic.sp == -1 {
            ctx.params.attack_skill_reserve_sp
        } else {
            tactic.sp
        };
        if (ctx.my_sp as i32) - reserve < known.sp_cost as i32 {
            return None;
        }
        Some((skill_id, level))
    }

    fn on_move_cmd(&mut self, ctx: &AiContext) {
        if ctx.my_x == self.dest_x && ctx.my_y == self.dest_y {
            self.state = AiState::Idle;
        }
    }

    fn on_attack_area_cmd(&mut self, ctx: &AiContext, out: &mut Vec<AiIntent>) {
        let object = self.command_target(ctx);
        if object != 0 {
            self.state = AiState::Chase;
            self.enemy = object;
            return;
        }
        if ctx.my_x == self.dest_x && ctx.my_y == self.dest_y {
            self.state = AiState::Idle;
        }
        let _ = out;
    }

    fn on_patrol_cmd(&mut self, ctx: &AiContext, out: &mut Vec<AiIntent>) {
        let object = self.command_target(ctx);
        if object != 0 {
            self.state = AiState::Chase;
            self.enemy = object;
            return;
        }
        if ctx.my_x == self.dest_x && ctx.my_y == self.dest_y {
            let new_dest = (self.patrol_x, self.patrol_y);
            self.patrol_x = ctx.my_x;
            self.patrol_y = ctx.my_y;
            self.dest_x = new_dest.0;
            self.dest_y = new_dest.1;
            self.emit_move(self.dest_x, self.dest_y, ctx, out);
        }
    }

    fn on_hold_cmd(&mut self, ctx: &AiContext, out: &mut Vec<AiIntent>) {
        if self.enemy != 0 {
            let d = self.distance_to(ctx, self.enemy);
            if d != -1 && d <= ctx.attack_range {
                self.emit_attack(ctx, out);
            } else {
                self.enemy = 0;
            }
            return;
        }
        let object = self.command_target(ctx);
        if object == 0 {
            return;
        }
        self.enemy = object;
    }

    fn on_skill_area_cmd(&mut self, ctx: &AiContext, out: &mut Vec<AiIntent>) {
        let range = (ctx.skill_range)(self.skill);
        if get_distance(ctx.my_x, ctx.my_y, self.dest_x, self.dest_y) <= range {
            out.push(AiIntent::SkillGround {
                skill_id: self.skill,
                level: self.skill_level,
                x: self.dest_x,
                y: self.dest_y,
            });
            self.state = AiState::Idle;
            self.skill = 0;
        }
    }

    fn on_follow_cmd(&mut self, ctx: &AiContext, out: &mut Vec<AiIntent>) {
        let Some((ox, oy)) = ctx.owner_pos else {
            return;
        };
        let d = get_distance(ox, oy, ctx.my_x, ctx.my_y);
        if d <= FOLLOW_DISTANCE {
            return;
        }
        if ctx.my_motion == Motion::Move {
            let d = get_distance(ox, oy, self.dest_x, self.dest_y);
            if d > FOLLOW_DISTANCE {
                out.push(AiIntent::MoveToOwner);
                self.dest_x = ox;
                self.dest_y = oy;
            }
        } else {
            out.push(AiIntent::MoveToOwner);
            self.dest_x = ox;
            self.dest_y = oy;
        }
    }

    // ---- target selection ----------------------------------------------

    /// Command states pick a target with the full tactics pipeline: a friend's
    /// attacker first, then the aggro/react enemy list.
    fn command_target(&self, ctx: &AiContext) -> u32 {
        let object = self.select_enemy(ctx, &self.friend_targets(ctx), None);
        if object != 0 {
            return object;
        }
        self.select_enemy(ctx, &self.enemy_list(ctx, self.aggro_flag(ctx)), None)
    }

    // ---- helpers --------------------------------------------------------

    fn emit_move(&self, x: i32, y: i32, ctx: &AiContext, out: &mut Vec<AiIntent>) {
        if get_distance(ctx.my_x, ctx.my_y, x, y) > MOVE_ABORT_DISTANCE {
            return;
        }
        out.push(AiIntent::MoveTo { x, y });
    }

    fn emit_attack(&mut self, ctx: &AiContext, out: &mut Vec<AiIntent>) {
        if self.enemy == 0 || self.enemy == ctx.my_gid {
            self.drop_enemy_to_idle();
            return;
        }
        if self.clock_ms < self.next_attack_ms {
            return;
        }
        out.push(AiIntent::Attack {
            target_gid: self.enemy,
        });
        let delay = if ctx.aspd_ms == 0 {
            DEFAULT_ATTACK_DELAY_MS
        } else {
            ctx.aspd_ms
        };
        self.next_attack_ms = self.clock_ms.wrapping_add(delay);
    }

    fn actor<'c>(&self, ctx: &'c AiContext, gid: u32) -> Option<&'c ActorView> {
        ctx.actors.iter().find(|a| a.gid == gid)
    }

    fn actor_pos(&self, ctx: &AiContext, gid: u32) -> Option<(i32, i32)> {
        self.actor(ctx, gid).map(|a| (a.x, a.y))
    }

    fn actor_motion(&self, ctx: &AiContext, gid: u32) -> Option<Motion> {
        self.actor(ctx, gid).map(|a| a.motion)
    }

    fn distance_to(&self, ctx: &AiContext, gid: u32) -> i32 {
        match self.actor_pos(ctx, gid) {
            Some((x, y)) => get_distance(ctx.my_x, ctx.my_y, x, y),
            None => -1,
        }
    }

    fn distance_from_owner(&self, ctx: &AiContext) -> i32 {
        match ctx.owner_pos {
            Some((ox, oy)) => get_distance(ctx.my_x, ctx.my_y, ox, oy),
            None => -1,
        }
    }

    fn is_out_of_sight(&self, ctx: &AiContext, gid: u32) -> bool {
        match self.actor_pos(ctx, gid) {
            Some((x, y)) => get_distance(ctx.my_x, ctx.my_y, x, y) > OUT_OF_SIGHT_DISTANCE,
            None => true,
        }
    }

    fn is_in_attack_sight(&self, ctx: &AiContext, gid: u32) -> bool {
        let Some((x, y)) = self.actor_pos(ctx, gid) else {
            return false;
        };
        let d = get_distance(ctx.my_x, ctx.my_y, x, y);
        let a = if self.skill == 0 {
            ctx.attack_range
        } else {
            (ctx.skill_range)(self.skill)
        };
        a >= d
    }
}

fn get_distance(x1: i32, y1: i32, x2: i32, y2: i32) -> i32 {
    let dx = (x1 - x2) as f64;
    let dy = (y1 - y2) as f64;
    (dx * dx + dy * dy).sqrt().floor() as i32
}

fn cheb_distance(x1: i32, y1: i32, x2: i32, y2: i32) -> i32 {
    (x1 - x2).abs().max((y1 - y2).abs())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consts::{BasicTactic, FriendClass};
    use crate::context::AiParams;
    use crate::tactics::{Tactic, TacticTable};

    const OWNER: u32 = 1;
    const ME: u32 = 100;

    fn neutral(_: u32) -> FriendClass {
        FriendClass::Neutral
    }

    struct Fixture {
        tactics: TacticTable,
        skills: Vec<crate::context::CompanionSkill>,
        params: AiParams,
    }

    impl Fixture {
        fn new() -> Self {
            Fixture {
                tactics: TacticTable::default(),
                skills: Vec::new(),
                params: AiParams::default(),
            }
        }

        fn with_tactic(mut self, class_id: u32, basic: BasicTactic) -> Self {
            let mut t = Tactic::default_row();
            t.id = class_id;
            t.basic = basic;
            self.tactics = TacticTable::from_rows(&[Tactic::default_row(), Tactic::treasure_row(), t]);
            self
        }

        fn with_skill(mut self, id: u16, level: u8, sp_cost: u16, range: i32) -> Self {
            self.skills.push(crate::context::CompanionSkill { id, level, sp_cost, range });
            self
        }

        fn ctx<'a>(
            &'a self,
            my: (i32, i32),
            motion: Motion,
            owner: Option<(i32, i32)>,
            actors: &'a [ActorView],
            skill_range: &'a dyn Fn(u16) -> i32,
        ) -> AiContext<'a> {
            AiContext {
                my_gid: ME,
                my_x: my.0,
                my_y: my.1,
                my_motion: motion,
                my_hp: 100,
                my_max_hp: 100,
                my_sp: 100,
                my_max_sp: 100,
                attack_range: 1,
                aspd_ms: 500,
                companion_type: 1,
                owner_gid: OWNER,
                owner_pos: owner,
                owner_motion: Motion::Stand,
                spheres: 0,
                now_ms: 0,
                actors,
                skills: &self.skills,
                skill_range,
                params: self.params,
                tactics: &self.tactics,
                friend_class: &neutral,
            }
        }
    }

    fn monster(gid: u32, class_id: u16, x: i32, y: i32, target: Option<u32>) -> ActorView {
        ActorView {
            gid,
            x,
            y,
            is_monster: true,
            is_player: false,
            class_id,
            motion: Motion::Stand,
            target_gid: target,
        }
    }

    fn player(gid: u32, x: i32, y: i32, target: Option<u32>) -> ActorView {
        ActorView {
            gid,
            x,
            y,
            is_monster: false,
            is_player: true,
            class_id: 0,
            motion: Motion::Attack,
            target_gid: target,
        }
    }

    fn one_step(ai: &mut CompanionAi, c: &AiContext) -> Vec<AiIntent> {
        ai.tick(0.15, c)
    }

    #[test]
    fn idle_follows_owner_then_settles_when_close() {
        let noskill = |_: u16| 1;
        let fx = Fixture::new();
        let mut ai = CompanionAi::new(false);

        let c = fx.ctx((100, 100), Motion::Stand, Some((110, 100)), &[], &noskill);
        one_step(&mut ai, &c);
        assert_eq!(ai.state(), AiState::Follow);

        let out = one_step(&mut ai, &c);
        assert!(out.contains(&AiIntent::MoveToOwner));

        let c = fx.ctx((108, 100), Motion::Stand, Some((110, 100)), &[], &noskill);
        one_step(&mut ai, &c);
        assert_eq!(ai.state(), AiState::Idle);
    }

    #[test]
    fn aggresses_default_tactic_monster_then_attacks_gated_by_aspd() {
        let noskill = |_: u16| 1;
        let fx = Fixture::new();
        let mut ai = CompanionAi::new(false);

        // Default tactic (Attack) → a free monster in aggro range is chased.
        let actors = [monster(500, 1002, 105, 100, None)];
        let c = fx.ctx((100, 100), Motion::Stand, Some((100, 100)), &actors, &noskill);
        one_step(&mut ai, &c);
        assert_eq!(ai.state(), AiState::Chase);
        assert_eq!(ai.enemy, 500);

        let actors = [monster(500, 1002, 101, 100, None)];
        let c = fx.ctx((100, 100), Motion::Stand, Some((100, 100)), &actors, &noskill);
        one_step(&mut ai, &c);
        assert_eq!(ai.state(), AiState::Attack);

        let out = one_step(&mut ai, &c);
        assert!(out.contains(&AiIntent::Attack { target_gid: 500 }));
        let out = one_step(&mut ai, &c);
        assert!(!out.contains(&AiIntent::Attack { target_gid: 500 }));
    }

    #[test]
    fn react_only_tactic_ignores_free_monster_but_defends_when_attacked() {
        let noskill = |_: u16| 1;
        let fx = Fixture::new().with_tactic(2000, BasicTactic::ReactLow);
        let mut ai = CompanionAi::new(false);

        // Free React-Low monster is left alone → falls through to following.
        let actors = [monster(500, 2000, 103, 100, None)];
        let c = fx.ctx((100, 100), Motion::Stand, Some((100, 100)), &actors, &noskill);
        one_step(&mut ai, &c);
        assert_ne!(ai.state(), AiState::Chase);

        // The same monster attacking us is defended against.
        let mut atk = monster(500, 2000, 103, 100, Some(ME));
        atk.motion = Motion::Attack;
        let actors = [atk];
        let c = fx.ctx((100, 100), Motion::Stand, Some((100, 100)), &actors, &noskill);
        one_step(&mut ai, &c);
        assert_eq!(ai.state(), AiState::Chase);
        assert_eq!(ai.enemy, 500);
    }

    #[test]
    fn ks_protection_skips_monster_fighting_another_player() {
        let noskill = |_: u16| 1;
        let fx = Fixture::new();
        let mut ai = CompanionAi::new(false);

        // A monster already engaged with a non-friend player is not stolen.
        let mut mob = monster(500, 1002, 103, 100, Some(9)); // targeting player 9
        mob.motion = Motion::Attack;
        let actors = [mob, player(9, 104, 100, Some(500))];
        let c = fx.ctx((100, 100), Motion::Stand, Some((100, 100)), &actors, &noskill);
        one_step(&mut ai, &c);
        assert_ne!(ai.state(), AiState::Chase);
    }

    #[test]
    fn filir_auto_casts_moonlight_then_melees_during_cooldown() {
        let noskill = |_: u16| 1;
        // Filir (type 3) knows Moonlight (8009) lvl 5, sp 20, range 1.
        let fx = Fixture::new().with_skill(HFLI_MOON, 5, 20, 1);
        let mut ai = CompanionAi::new(false);

        let actors = [monster(500, 1002, 101, 100, None)];
        let mut c = fx.ctx((100, 100), Motion::Stand, Some((100, 100)), &actors, &noskill);
        c.companion_type = 3;

        // Chase → Attack (in melee range).
        one_step(&mut ai, &c);
        one_step(&mut ai, &c);
        assert_eq!(ai.state(), AiState::Attack);

        // First attack tick casts Moonlight, not a melee.
        let out = one_step(&mut ai, &c);
        assert!(out.contains(&AiIntent::SkillObject { skill_id: HFLI_MOON, level: 5, target_gid: 500 }));
        assert!(!out.contains(&AiIntent::Attack { target_gid: 500 }));

        // Immediately after, the reuse cooldown gates the skill → it melees.
        let mut melee_seen = false;
        for _ in 0..4 {
            let out = one_step(&mut ai, &c);
            assert!(!out.iter().any(|i| matches!(i, AiIntent::SkillObject { .. })));
            if out.contains(&AiIntent::Attack { target_gid: 500 }) {
                melee_seen = true;
            }
        }
        assert!(melee_seen);
    }

    #[test]
    fn lif_has_no_attack_skill_and_only_melees() {
        let noskill = |_: u16| 1;
        // Lif (type 1) knows Healing (8001) — a support skill, never auto-cast.
        let fx = Fixture::new().with_skill(8001, 5, 25, 0);
        let mut ai = CompanionAi::new(false);

        let actors = [monster(500, 1002, 101, 100, None)];
        let mut c = fx.ctx((100, 100), Motion::Stand, Some((100, 100)), &actors, &noskill);
        c.companion_type = 1;

        one_step(&mut ai, &c);
        one_step(&mut ai, &c);
        assert_eq!(ai.state(), AiState::Attack);
        let out = one_step(&mut ai, &c);
        assert!(out.contains(&AiIntent::Attack { target_gid: 500 }));
        assert!(!out.iter().any(|i| matches!(i, AiIntent::SkillObject { .. })));
    }

    #[test]
    fn self_cast_skill_fires_once_then_returns_to_idle() {
        let selfbuff_range = |_: u16| 9;
        let fx = Fixture::new();
        let mut ai = CompanionAi::new(false);

        // Self-buff: target gid is the caster's own gid.
        ai.push_command(OwnerCommand::skill_object(8010, 5, ME));
        let me_actor = ActorView {
            gid: ME,
            x: 100,
            y: 100,
            is_monster: false,
            is_player: false,
            class_id: 0,
            motion: Motion::Stand,
            target_gid: None,
        };
        let actors = [me_actor];
        let c = fx.ctx((100, 100), Motion::Stand, Some((100, 100)), &actors, &selfbuff_range);

        let mut fired = 0;
        for _ in 0..6 {
            let out = one_step(&mut ai, &c);
            for it in &out {
                if let AiIntent::SkillObject { target_gid, .. } = it {
                    if *target_gid == ME {
                        fired += 1;
                    }
                }
                // Must never melee itself.
                assert!(!matches!(it, AiIntent::Attack { target_gid } if *target_gid == ME));
            }
        }
        assert_eq!(fired, 1);
        assert_eq!(ai.state(), AiState::Idle);
        assert_eq!(ai.enemy, 0);
    }

    #[test]
    fn move_command_interrupts_and_reserved_queue_drains_in_idle() {
        let noskill = |_: u16| 1;
        let fx = Fixture::new();
        let mut ai = CompanionAi::new(false);

        ai.push_command(OwnerCommand::move_to(105, 105));
        let c = fx.ctx((100, 100), Motion::Stand, Some((100, 100)), &[], &noskill);
        let out = one_step(&mut ai, &c);
        assert_eq!(ai.state(), AiState::MoveCmd);
        assert!(out.contains(&AiIntent::MoveTo { x: 105, y: 105 }));

        ai.push_reserved(OwnerCommand::move_to(106, 106));
        let c = fx.ctx((105, 105), Motion::Stand, Some((100, 100)), &[], &noskill);
        one_step(&mut ai, &c);
        let c = fx.ctx((105, 105), Motion::Stand, Some((100, 100)), &[], &noskill);
        let out = one_step(&mut ai, &c);
        assert!(out.contains(&AiIntent::MoveTo { x: 106, y: 106 }));

        let mut ai = CompanionAi::new(false);
        for i in 0..15 {
            ai.push_reserved(OwnerCommand::move_to(i, i));
            let c = fx.ctx((0, 0), Motion::Stand, Some((0, 0)), &[], &noskill);
            one_step(&mut ai, &c);
        }
        assert!(ai.res_cmd_list.len() <= RESERVED_QUEUE_CAP);
    }
}
