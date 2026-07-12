use crate::consts::{FriendClass, KsTactic};
use crate::context::{AiContext, Motion};
use crate::engine::{CompanionAi, cheb_distance, get_distance};

const CAST_REACT: i32 = 1;
const TACT_TANKMOB: i32 = -2;
const TACT_TANK: i32 = -1;

pub(crate) struct Candidate {
    pub gid: u32,
    pub motion_class: i32,
    pub v2: i32,
    pub tact: i32,
    pub casttact: i32,
}

fn motion_class(m: Motion) -> i32 {
    match m {
        Motion::Attack => 3,
        Motion::Move => 1,
        Motion::Dead => -1,
        _ => 0,
    }
}

impl CompanionAi {
    fn tact_basic(&self, ctx: &AiContext, class_id: u16) -> i32 {
        i32::from(ctx.tactics.resolve(class_id as u32).basic)
    }

    fn tact_cast(&self, ctx: &AiContext, class_id: u16) -> i32 {
        i32::from(ctx.tactics.resolve(class_id as u32).cast)
    }

    fn weight(&self, ctx: &AiContext, class_id: u16) -> f32 {
        ctx.tactics.resolve(class_id as u32).weight
    }

    fn is_friend(&self, ctx: &AiContext, id: u32) -> bool {
        id == ctx.owner_gid
            || matches!(
                (ctx.friend_class)(id),
                FriendClass::Friend | FriendClass::Retainer
            )
    }

    fn is_friend_or_self(&self, ctx: &AiContext, id: u32) -> bool {
        id == ctx.owner_gid
            || id == ctx.my_gid
            || matches!(
                (ctx.friend_class)(id),
                FriendClass::Friend | FriendClass::Retainer | FriendClass::Ally
            )
    }

    /// Reference `GetTargetClass`: what the actor `id` is relative to us.
    fn target_class(&self, ctx: &AiContext, id: u32) -> i32 {
        if id == ctx.my_gid {
            return 1;
        }
        if id == 0 {
            return 0;
        }
        match self.actor(ctx, id) {
            Some(a) if a.is_player => {
                if self.is_friend_or_self(ctx, id) {
                    2
                } else {
                    -2
                }
            }
            Some(a) if a.is_monster => -1,
            _ if id == ctx.owner_gid => 2,
            _ => 0,
        }
    }

    pub(crate) fn is_not_ks(&self, ctx: &AiContext, target: u32) -> bool {
        let Some(a) = self.actor(ctx, target) else {
            return true;
        };
        let tt = a.target_gid.unwrap_or(0);
        let motion = a.motion;
        let ks = ctx.tactics.resolve(a.class_id as u32).ks;
        if a.is_player {
            return true;
        }
        if (ks == KsTactic::Polite || ctx.params.do_not_attack_moving) && motion == Motion::Move {
            return false;
        }
        if self.is_friend(ctx, tt) || tt == ctx.my_gid {
            return true;
        }
        if ks == KsTactic::Always {
            return true;
        }
        // Enemy is attacking another player → that player's kill.
        if tt > 0 {
            if let Some(tta) = self.actor(ctx, tt) {
                if tta.is_player {
                    return false;
                }
            }
        }
        // Otherwise KS only if some other non-friend player is already on it.
        !ctx.actors.iter().any(|o| {
            o.is_player && !self.is_friend_or_self(ctx, o.gid) && o.target_gid == Some(target)
        })
    }

    pub(crate) fn aggro_count(&self, ctx: &AiContext) -> f32 {
        let Some((ox, oy)) = ctx.owner_pos else {
            return 0.0;
        };
        let bounds = ctx.move_bounds();
        let mut count = 0.0;
        for a in ctx.actors {
            if !a.is_monster {
                continue;
            }
            if self.target_class(ctx, a.target_gid.unwrap_or(0)) > 0
                && cheb_distance(ox, oy, a.x, a.y) <= bounds
            {
                count += self.weight(ctx, a.class_id);
            }
        }
        count
    }

    pub(crate) fn tank_count(&self, ctx: &AiContext) -> i32 {
        ctx.actors
            .iter()
            .filter(|a| {
                a.is_monster
                    && self.tact_basic(ctx, a.class_id) == TACT_TANK
                    && a.target_gid == Some(ctx.my_gid)
            })
            .count() as i32
    }

    fn is_rescue_target(&self, ctx: &AiContext, id: u32) -> bool {
        use crate::consts::RescueTactic;
        let Some(a) = self.actor(ctx, id) else {
            return false;
        };
        let rescue = ctx.tactics.resolve(a.class_id as u32).rescue;
        let target = a.target_gid.unwrap_or(0);
        match rescue {
            RescueTactic::Never => false,
            RescueTactic::Owner => target == ctx.owner_gid,
            RescueTactic::SelfOnly => target == ctx.my_gid,
            RescueTactic::All => self.is_friend_or_self(ctx, target) && target != ctx.my_gid,
            RescueTactic::Friend => (ctx.friend_class)(target) == FriendClass::Friend,
            RescueTactic::Retainer => (ctx.friend_class)(target) == FriendClass::Retainer,
        }
    }

    /// Reference `GetEnemyList`: candidate monsters for the given aggro level
    /// (1 aggressive, 0 react, -1 tank, -2 rescue).
    pub(crate) fn enemy_list(&self, ctx: &AiContext, aggro: i32) -> Vec<Candidate> {
        let Some((ox, oy)) = ctx.owner_pos else {
            return Vec::new();
        };
        let move_bounds = ctx.move_bounds();
        let aggro_dist = ctx.aggro_dist();
        let sp_pct = ctx.sp_pct();
        let mut out = Vec::new();
        for a in ctx.actors {
            if !a.is_monster || a.gid == ctx.my_gid {
                continue;
            }
            let mc = motion_class(a.motion);
            if mc == -1 {
                continue;
            }
            let mut tact = self.tact_basic(ctx, a.class_id);
            let casttact = self.tact_cast(ctx, a.class_id);
            if tact == TACT_TANKMOB {
                tact = if self.aggro_count(ctx) > ctx.params.auto_mob_count as f32 {
                    3
                } else {
                    TACT_TANK
                };
            }
            let v2 = self.target_class(ctx, a.target_gid.unwrap_or(0));

            let attack_family = 0 < tact
                && (tact < 5 || tact > 9)
                && aggro == 1
                && (!ctx.params.do_not_attack_moving || mc != 1)
                && (tact != 14 || !ctx.params.attack_last_full_sp || sp_pct == 100);
            let react = v2 > 0
                && tact > 0
                && (mc == 3 || casttact == CAST_REACT)
                && aggro != 2
                && (aggro > -1 || (aggro == -2 && self.is_rescue_target(ctx, a.gid)));
            let tank = tact == TACT_TANK && aggro == -1 && v2 != 1;
            if !(attack_family || react || tank) {
                continue;
            }
            if !self.is_not_ks(ctx, a.gid) {
                continue;
            }
            let within = if aggro == 0 || v2 > 0 {
                move_bounds >= cheb_distance(ox, oy, a.x, a.y)
            } else {
                aggro_dist >= get_distance(ox, oy, a.x, a.y)
            };
            if within {
                out.push(Candidate {
                    gid: a.gid,
                    motion_class: mc,
                    v2,
                    tact,
                    casttact,
                });
            }
        }
        out
    }

    /// Reference `GetFriendTargets`: monsters that friends are actively attacking.
    pub(crate) fn friend_targets(&self, ctx: &AiContext) -> Vec<Candidate> {
        let mut out = Vec::new();
        for a in ctx.actors {
            if !(a.gid == ctx.owner_gid || self.is_friend(ctx, a.gid)) {
                continue;
            }
            if a.motion != Motion::Attack {
                continue;
            }
            let Some(t) = a.target_gid else { continue };
            let Some(tgt) = self.actor(ctx, t) else {
                continue;
            };
            if !tgt.is_monster {
                continue;
            }
            let tact = self.tact_basic(ctx, tgt.class_id);
            if tact <= 0 {
                continue;
            }
            out.push(Candidate {
                gid: t,
                motion_class: motion_class(tgt.motion),
                v2: self.target_class(ctx, tgt.target_gid.unwrap_or(0)),
                tact,
                casttact: self.tact_cast(ctx, tgt.class_id),
            });
        }
        out
    }

    /// Reference `SelectEnemy`: highest-priority, then nearest. `cur` set for an
    /// opportunistic re-target (must beat the current target's priority/distance).
    pub(crate) fn select_enemy(
        &self,
        ctx: &AiContext,
        cands: &[Candidate],
        cur: Option<u32>,
    ) -> u32 {
        let mut min_priority = -1;
        let mut min_dis = 100;
        let mut result = 0;
        let mut max_reachable = 1;

        if let Some(cur_gid) = cur {
            let dist = get_distance(ctx.my_x, ctx.my_y, self.actor_x(ctx, cur_gid), self.actor_y(ctx, cur_gid));
            let cur_agr = self
                .actor(ctx, cur_gid)
                .map(|a| self.is_friend_or_self(ctx, a.target_gid.unwrap_or(0)))
                .unwrap_or(false);
            let base = self.tact_basic(ctx, self.actor(ctx, cur_gid).map(|a| a.class_id).unwrap_or(0));
            min_priority = conv_priority(base, cur_agr);
            if dist < 3 {
                return 0;
            }
            min_dis = dist - 3;
        }

        for c in cands {
            let agr = c.v2 > 0 && (c.motion_class == 3 || c.casttact == CAST_REACT);
            let priority = conv_priority(c.tact, agr);
            let dis = get_distance(ctx.my_x, ctx.my_y, self.actor_x(ctx, c.gid), self.actor_y(ctx, c.gid));
            let unreachable = if self.is_unreachable(c.gid) { 1 } else { 0 };
            if unreachable <= max_reachable
                && (priority > min_priority || (priority == min_priority && dis < min_dis))
            {
                result = c.gid;
                min_dis = dis;
                min_priority = priority;
                max_reachable = unreachable;
            }
        }
        result
    }

    fn actor_x(&self, ctx: &AiContext, gid: u32) -> i32 {
        self.actor(ctx, gid).map(|a| a.x).unwrap_or(ctx.my_x)
    }
    fn actor_y(&self, ctx: &AiContext, gid: u32) -> i32 {
        self.actor(ctx, gid).map(|a| a.y).unwrap_or(ctx.my_y)
    }
}

/// Reference `convpriority`: map a basic-tactic value + aggression flag to a
/// selection priority (higher wins).
fn conv_priority(base: i32, agr: bool) -> i32 {
    let mut base = base;
    if base > 9 && base < 13 {
        base -= 8;
    }
    if base == 13 {
        base = if agr { 7 } else { 2 };
    }
    if base == 14 {
        base = 1;
    }
    if base > 6 && agr {
        return base;
    }
    if base == 4 || base == 3 || base == 15 {
        let mut p = base + 1;
        if !agr {
            p -= 2;
        }
        return p;
    }
    if base == 2 {
        return 1;
    }
    0
}
