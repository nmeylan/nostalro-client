use std::collections::VecDeque;

use crate::context::{ActorView, AiContext, AiIntent, Motion};

const TICK_INTERVAL_MS: u32 = 140;
const OUT_OF_SIGHT_DISTANCE: i32 = 20;
const FOLLOW_DISTANCE: i32 = 3;
const MOVE_SPLIT_DISTANCE: i32 = 15;
const MOVE_ABORT_DISTANCE: i32 = 24;
const RESERVED_QUEUE_CAP: usize = 10;
const NEAREST_SCAN_CAP: i32 = 100;
const DEFAULT_ATTACK_DELAY_MS: u32 = 1000;

/// Player-selectable target disposition, overriding the per-family default.
/// Mirrors the four companion attack modes of the original game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiMode {
    /// Aggress the nearest monster on sight.
    Aggressive,
    /// Attack the owner's attacker and retaliate, but don't seek monsters out.
    Assist,
    /// Only retaliate against monsters attacking the companion.
    Passive,
    /// Never attack; follow only.
    FollowOnly,
}

impl AiMode {
    pub fn label(self) -> &'static str {
        match self {
            AiMode::Aggressive => "Aggressive",
            AiMode::Assist => "Assist",
            AiMode::Passive => "Passive",
            AiMode::FollowOnly => "Follow",
        }
    }

    pub fn next(self) -> Self {
        match self {
            AiMode::Aggressive => AiMode::Assist,
            AiMode::Assist => AiMode::Passive,
            AiMode::Passive => AiMode::FollowOnly,
            AiMode::FollowOnly => AiMode::Aggressive,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiState {
    Idle,
    Follow,
    Chase,
    Attack,
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
    mode: Option<AiMode>,
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
            mode: None,
        }
    }

    pub fn state(&self) -> AiState {
        self.state
    }

    /// The active attack mode, defaulting to Assist when unset (the per-family
    /// default still applies while `None`; this is only the label shown in UI).
    pub fn mode(&self) -> AiMode {
        self.mode.unwrap_or(AiMode::Assist)
    }

    pub fn set_mode(&mut self, mode: AiMode) {
        self.mode = Some(mode);
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
            if let Some((ox, oy)) = ctx.owner_pos {
                self.dest_x = ox;
                self.dest_y = oy;
            }
            self.enemy = 0;
            self.skill = 0;
        } else {
            self.state = AiState::Idle;
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
        let assist_owner = !matches!(self.mode, Some(AiMode::Passive) | Some(AiMode::FollowOnly));
        if assist_owner {
            let object = self.get_owner_enemy(ctx);
            if object != 0 {
                self.state = AiState::Chase;
                self.enemy = object;
                return;
            }
        }
        let object = self.get_my_enemy(ctx);
        if object != 0 {
            self.state = AiState::Chase;
            self.enemy = object;
            return;
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
        if self.is_out_of_sight(ctx, self.enemy) {
            self.state = AiState::Idle;
            self.enemy = 0;
            self.dest_x = 0;
            self.dest_y = 0;
            return;
        }
        if self.is_in_attack_sight(ctx, self.enemy) {
            self.state = AiState::Attack;
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
        if self.skill == 0 {
            self.emit_attack(ctx, out);
        } else {
            out.push(AiIntent::SkillObject {
                skill_id: self.skill,
                level: self.skill_level,
                target_gid: self.enemy,
            });
            if self.is_mercenary {
                self.enemy = 0;
            }
            self.skill = 0;
        }
    }

    fn on_move_cmd(&mut self, ctx: &AiContext) {
        if ctx.my_x == self.dest_x && ctx.my_y == self.dest_y {
            self.state = AiState::Idle;
        }
    }

    fn on_attack_area_cmd(&mut self, ctx: &AiContext, out: &mut Vec<AiIntent>) {
        let mut object = self.get_owner_enemy(ctx);
        if object == 0 {
            object = self.get_my_enemy(ctx);
        }
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
        let mut object = self.get_owner_enemy(ctx);
        if object == 0 {
            object = self.get_my_enemy(ctx);
        }
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
        let mut object = self.get_owner_enemy(ctx);
        if object == 0 {
            object = self.get_my_enemy(ctx);
            if object == 0 {
                return;
            }
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

    fn get_owner_enemy(&self, ctx: &AiContext) -> u32 {
        let mut result = 0;
        let mut min_dis = NEAREST_SCAN_CAP;
        for actor in ctx.actors {
            if actor.gid == ctx.owner_gid || actor.gid == ctx.my_gid {
                continue;
            }
            if actor.target_gid != Some(ctx.owner_gid) {
                continue;
            }
            let is_enemy = actor.is_monster || actor.motion == Motion::Attack;
            if !is_enemy {
                continue;
            }
            let dis = get_distance(ctx.my_x, ctx.my_y, actor.x, actor.y);
            if dis < min_dis {
                result = actor.gid;
                min_dis = dis;
            }
        }
        result
    }

    fn get_my_enemy(&self, ctx: &AiContext) -> u32 {
        match self.mode {
            Some(AiMode::Aggressive) => return self.get_my_enemy_b(ctx),
            Some(AiMode::Assist) | Some(AiMode::Passive) => return self.get_my_enemy_a(ctx),
            Some(AiMode::FollowOnly) => return 0,
            None => {}
        }
        if self.is_mercenary {
            return self.get_my_enemy_a(ctx);
        }
        if is_melee_homunculus(ctx.companion_type) {
            self.get_my_enemy_a(ctx)
        } else if is_ranged_homunculus(ctx.companion_type) {
            self.get_my_enemy_b(ctx)
        } else {
            0
        }
    }

    /// Non-aggressive: only actors already targeting me.
    fn get_my_enemy_a(&self, ctx: &AiContext) -> u32 {
        let mut result = 0;
        let mut min_dis = NEAREST_SCAN_CAP;
        for actor in ctx.actors {
            if actor.gid == ctx.owner_gid || actor.gid == ctx.my_gid {
                continue;
            }
            if actor.target_gid != Some(ctx.my_gid) {
                continue;
            }
            let dis = get_distance(ctx.my_x, ctx.my_y, actor.x, actor.y);
            if dis < min_dis {
                result = actor.gid;
                min_dis = dis;
            }
        }
        result
    }

    /// Aggressive: nearest monster.
    fn get_my_enemy_b(&self, ctx: &AiContext) -> u32 {
        let mut result = 0;
        let mut min_dis = NEAREST_SCAN_CAP;
        for actor in ctx.actors {
            if actor.gid == ctx.owner_gid || actor.gid == ctx.my_gid {
                continue;
            }
            if !actor.is_monster {
                continue;
            }
            let dis = get_distance(ctx.my_x, ctx.my_y, actor.x, actor.y);
            if dis < min_dis {
                result = actor.gid;
                min_dis = dis;
            }
        }
        result
    }

    // ---- helpers --------------------------------------------------------

    fn emit_move(&self, x: i32, y: i32, ctx: &AiContext, out: &mut Vec<AiIntent>) {
        if get_distance(ctx.my_x, ctx.my_y, x, y) > MOVE_ABORT_DISTANCE {
            return;
        }
        out.push(AiIntent::MoveTo { x, y });
    }

    fn emit_attack(&mut self, ctx: &AiContext, out: &mut Vec<AiIntent>) {
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

/// LIF / AMISTR families (incl. `_H` and `2` variants) — counterattack only.
fn is_melee_homunculus(t: u16) -> bool {
    matches!(t, 1 | 2 | 5 | 6 | 9 | 10 | 13 | 14)
}

/// FILIR / VANILMIRTH families — aggress nearby monsters.
fn is_ranged_homunculus(t: u16) -> bool {
    matches!(t, 3 | 4 | 7 | 8 | 11 | 12 | 15 | 16)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx<'a>(
        my: (i32, i32),
        motion: Motion,
        owner: Option<(i32, i32)>,
        actors: &'a [ActorView],
        skill_range: &'a dyn Fn(u16) -> i32,
    ) -> AiContext<'a> {
        AiContext {
            my_gid: 100,
            my_x: my.0,
            my_y: my.1,
            my_motion: motion,
            my_hp: 100,
            my_max_hp: 100,
            my_sp: 100,
            my_max_sp: 100,
            attack_range: 1,
            aspd_ms: 500,
            companion_type: 1, // Lif (melee)
            owner_gid: 1,
            owner_pos: owner,
            owner_motion: Motion::Stand,
            spheres: 0,
            now_ms: 0,
            actors,
            skill_range,
        }
    }

    fn monster(gid: u32, x: i32, y: i32, target: Option<u32>) -> ActorView {
        ActorView {
            gid,
            x,
            y,
            is_monster: true,
            is_player: false,
            class_id: 1002,
            motion: Motion::Stand,
            target_gid: target,
        }
    }

    fn one_step(ai: &mut CompanionAi, c: &AiContext) -> Vec<AiIntent> {
        ai.tick(0.15, c)
    }

    #[test]
    fn idle_follows_owner_then_settles_when_close() {
        let noskill = |_: u16| 1;
        let mut ai = CompanionAi::new(false);

        let c = ctx((100, 100), Motion::Stand, Some((110, 100)), &[], &noskill);
        one_step(&mut ai, &c);
        assert_eq!(ai.state(), AiState::Follow);

        let out = one_step(&mut ai, &c);
        assert!(out.contains(&AiIntent::MoveToOwner));

        let c = ctx((100, 100), Motion::Move, Some((110, 100)), &[], &noskill);
        let out = one_step(&mut ai, &c);
        assert!(!out.contains(&AiIntent::MoveToOwner));

        let c = ctx((108, 100), Motion::Stand, Some((110, 100)), &[], &noskill);
        one_step(&mut ai, &c);
        assert_eq!(ai.state(), AiState::Idle);
    }

    #[test]
    fn chases_attacker_of_owner_then_attacks_gated_by_aspd() {
        let noskill = |_: u16| 1;
        let mut ai = CompanionAi::new(false);

        let actors = [monster(500, 105, 100, Some(1))];
        let c = ctx((100, 100), Motion::Stand, Some((100, 100)), &actors, &noskill);
        one_step(&mut ai, &c);
        assert_eq!(ai.state(), AiState::Chase);

        let out = one_step(&mut ai, &c);
        assert!(matches!(out.first(), Some(AiIntent::MoveTo { .. })));

        let actors = [monster(500, 101, 100, Some(1))];
        let c = ctx((100, 100), Motion::Stand, Some((100, 100)), &actors, &noskill);
        one_step(&mut ai, &c);
        assert_eq!(ai.state(), AiState::Attack);

        let out = one_step(&mut ai, &c);
        assert!(out.contains(&AiIntent::Attack { target_gid: 500 }));

        let out = one_step(&mut ai, &c);
        assert!(!out.contains(&AiIntent::Attack { target_gid: 500 }));

        let mut dead = monster(500, 101, 100, Some(1));
        dead.motion = Motion::Dead;
        let actors = [dead];
        let c = ctx((100, 100), Motion::Stand, Some((100, 100)), &actors, &noskill);
        one_step(&mut ai, &c);
        assert_eq!(ai.state(), AiState::Idle);
    }

    #[test]
    fn move_command_interrupts_and_reserved_queue_drains_in_idle() {
        let noskill = |_: u16| 1;
        let mut ai = CompanionAi::new(false);

        ai.push_command(OwnerCommand::move_to(105, 105));
        let c = ctx((100, 100), Motion::Stand, Some((100, 100)), &[], &noskill);
        let out = one_step(&mut ai, &c);
        assert_eq!(ai.state(), AiState::MoveCmd);
        assert!(out.contains(&AiIntent::MoveTo { x: 105, y: 105 }));

        ai.push_reserved(OwnerCommand::move_to(106, 106));
        let c = ctx((105, 105), Motion::Stand, Some((100, 100)), &[], &noskill);
        one_step(&mut ai, &c);
        let c = ctx((105, 105), Motion::Stand, Some((100, 100)), &[], &noskill);
        let out = one_step(&mut ai, &c);
        assert!(out.contains(&AiIntent::MoveTo { x: 106, y: 106 }));

        let mut ai = CompanionAi::new(false);
        for i in 0..15 {
            ai.push_reserved(OwnerCommand::move_to(i, i));
            let c = ctx((0, 0), Motion::Stand, Some((0, 0)), &[], &noskill);
            one_step(&mut ai, &c);
        }
        assert!(ai.res_cmd_list.len() <= RESERVED_QUEUE_CAP);
    }

    #[test]
    fn ai_mode_overrides_family_targeting() {
        let noskill = |_: u16| 1;

        let mut ai = CompanionAi::new(false);
        ai.set_mode(AiMode::Aggressive);
        let actors = [monster(500, 103, 100, None)];
        let c = ctx((100, 100), Motion::Stand, Some((100, 100)), &actors, &noskill);
        one_step(&mut ai, &c);
        assert_eq!(ai.state(), AiState::Chase);

        let mut ai = CompanionAi::new(false);
        ai.set_mode(AiMode::FollowOnly);
        let actors = [monster(500, 101, 100, Some(100))];
        let c = ctx((100, 100), Motion::Stand, Some((100, 100)), &actors, &noskill);
        one_step(&mut ai, &c);
        assert_eq!(ai.state(), AiState::Idle);
    }
}
