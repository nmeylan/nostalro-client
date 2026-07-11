use crate::App;
use ragnarok_game::companion::ai::{ActorView, AiContext, AiIntent, Motion};
use ragnarok_game::entity::EntityType;
use ragnarok_game::event::SkillInfo;
use ragnarok_game::sprite_path::{homunculus_type_index, mercenary_type_index};
use ragnarok_network::{
    build_companion_attack_packet, build_companion_move_packet,
    build_companion_move_to_owner_packet, build_use_skill_packet, build_use_skill_to_ground_packet,
};

impl App {
    pub(crate) fn update_companion_ai(&mut self, delta: f32) {
        if self.game.homunculus.is_none() && self.game.mercenary.is_none() {
            return;
        }
        let Some(owner_gid) = self.game.entities.player_id() else {
            return;
        };
        let owner_pos = self.game.entities.player().map(|p| {
            let (x, y) = p.movement.cell_position();
            (x as i32, y as i32)
        });

        // One actor snapshot shared by both companions (owned, so it doesn't hold
        // a borrow of `entities` across the mutable AI tick).
        let actors: Vec<ActorView> = self
            .game
            .entities
            .iter()
            .map(|e| {
                let (x, y) = e.movement.cell_position();
                ActorView {
                    gid: e.id,
                    x: x as i32,
                    y: y as i32,
                    is_monster: e.entity_type == EntityType::Monster,
                    motion: Motion::from_state(e.state),
                    target_gid: e.target_gid,
                }
            })
            .collect();

        if let Some((gid, intents)) = self.tick_homunculus(owner_gid, owner_pos, &actors, delta) {
            self.dispatch_companion_intents(gid, &intents);
        }
        if let Some((gid, intents)) = self.tick_mercenary(owner_gid, owner_pos, &actors, delta) {
            self.dispatch_companion_intents(gid, &intents);
        }
    }

    fn tick_homunculus(
        &mut self,
        owner_gid: u32,
        owner_pos: Option<(i32, i32)>,
        actors: &[ActorView],
        delta: f32,
    ) -> Option<(u32, Vec<AiIntent>)> {
        let homun = self.game.homunculus.as_mut()?;
        if homun.vaporized || homun.gid == 0 {
            return None;
        }
        let gid = homun.gid;
        let entity = self.game.entities.get(gid)?;
        let (mx, my) = entity.movement.cell_position();
        let motion = Motion::from_state(entity.state);
        let job = entity.job;
        // (re-borrow homun mutably after the immutable entity read)
        let homun = self.game.homunculus.as_mut()?;
        homun.job = job;
        let attack_range = homun.atk_range.max(1) as i32;
        let aspd_ms = homun.aspd.max(0) as u32;
        let skills = homun.skills.clone();
        let ctx = AiContext {
            my_gid: gid,
            my_x: mx as i32,
            my_y: my as i32,
            my_motion: motion,
            attack_range,
            aspd_ms,
            companion_type: homunculus_type_index(job),
            owner_gid,
            owner_pos,
            actors,
            skill_range: &|id| skill_range(&skills, id),
        };
        Some((gid, homun.ai.tick(delta, &ctx)))
    }

    fn tick_mercenary(
        &mut self,
        owner_gid: u32,
        owner_pos: Option<(i32, i32)>,
        actors: &[ActorView],
        delta: f32,
    ) -> Option<(u32, Vec<AiIntent>)> {
        let merc = self.game.mercenary.as_mut()?;
        if merc.gid == 0 {
            return None;
        }
        let gid = merc.gid;
        let has_cmd = merc.ai.has_pending_command();
        let Some(entity) = self.game.entities.get(gid) else {
            if has_cmd {
                tracing::info!("tick_mercenary: merc gid={gid} has a command but no entity in map");
            }
            return None;
        };
        let (mx, my) = entity.movement.cell_position();
        let motion = Motion::from_state(entity.state);
        let job = entity.job;
        let merc = self.game.mercenary.as_mut()?;
        merc.job = job;
        let attack_range = merc.atk_range.max(1) as i32;
        let aspd_ms = merc.aspd.max(0) as u32;
        let skills = merc.skills.clone();
        let ctx = AiContext {
            my_gid: gid,
            my_x: mx as i32,
            my_y: my as i32,
            my_motion: motion,
            attack_range,
            aspd_ms,
            companion_type: mercenary_type_index(job),
            owner_gid,
            owner_pos,
            actors,
            skill_range: &|id| skill_range(&skills, id),
        };
        Some((gid, merc.ai.tick(delta, &ctx)))
    }

    fn dispatch_companion_intents(&mut self, gid: u32, intents: &[AiIntent]) {
        let pv = self.config.packetver;
        for intent in intents {
            match *intent {
                AiIntent::MoveTo { x, y } => {
                    self.channel
                        .send_packet(build_companion_move_packet(gid, x as u16, y as u16, pv));
                }
                AiIntent::MoveToOwner => {
                    self.channel
                        .send_packet(build_companion_move_to_owner_packet(gid, pv));
                }
                AiIntent::Attack { target_gid } => {
                    self.channel
                        .send_packet(build_companion_attack_packet(gid, target_gid, pv));
                }
                AiIntent::SkillObject {
                    skill_id,
                    level,
                    target_gid,
                } => {
                    tracing::info!(
                        "companion {gid} skill_object intent: skill={skill_id} lvl={level} target={target_gid}"
                    );
                    self.channel.send_packet(build_use_skill_packet(
                        skill_id,
                        level as i16,
                        target_gid,
                        pv,
                    ));
                }
                AiIntent::SkillGround {
                    skill_id,
                    level,
                    x,
                    y,
                } => {
                    self.channel.send_packet(build_use_skill_to_ground_packet(
                        skill_id,
                        level as i16,
                        x as i16,
                        y as i16,
                        pv,
                    ));
                }
            }
        }
    }
}

fn skill_range(skills: &[SkillInfo], id: u16) -> i32 {
    skills
        .iter()
        .find(|s| s.id == id)
        .map(|s| s.attack_range as i32)
        .unwrap_or(9)
}
