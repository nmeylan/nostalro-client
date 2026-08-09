use crate::App;
use models::enums::effect_id::EffectId;
use ragnarok_game::event::GameEvent;
use ragnarok_network::build_marry_request_packet;

/// GRF base path for an NPC cutin illustration.
pub(crate) fn cutin_texture_path(image: &str) -> String {
    ragnarok_resources::ui::illust::named(image)
}

impl App {
    pub(super) fn handle_wedding_celebration(&mut self, account_id: u32) {
        let key = self.game.world.entities.resolve_key(account_id);
        self.effect_queue.spawn_on(EffectId::Colorpaper, key);
    }

    pub(super) fn handle_marriage_proposed(&mut self, aid: u32, gid: u32, name: String) {
        self.game.pending_confirms.pending_marriage_proposal = Some((aid, gid));
        let msg = format!("{name} wishes to marry you. Do you accept?");
        self.game.arm_confirm(&mut self.windows, &msg, |accept| {
            Some(GameEvent::RespondMarriageProposal { accept })
        });
    }

    /// Consumes the click that picks the proposal target. Clicking anything that
    /// is not another player keeps the cursor armed, as the original game does.
    pub(crate) fn try_marriage_target_click(&mut self) -> bool {
        if !self.game.pending_casts.marriage_targeting {
            return false;
        }
        if let Some(entity_id) = self.game.hover.hovered_player_id {
            self.game.pending_casts.marriage_targeting = false;
            let target_aid = self.game.world.entities.account_id_of(entity_id);
            self.channel.send_packet(build_marry_request_packet(
                target_aid,
                self.active_packetver,
            ));
        }
        true
    }

    pub(super) fn handle_divorced(&mut self, name: String) {
        self.game.character.partner_name.clear();
        self.windows
            .chat_window
            .add_system(format!("You have divorced {name}."));
    }

    pub(super) fn handle_npc_cutin(&mut self, image: String, position: u8) {
        if position == 255 {
            self.game.npc_cutins = [None, None, None];
            return;
        }
        let Some(slot) = self.game.npc_cutins.get_mut(position as usize) else {
            return;
        };
        if image.is_empty() {
            *slot = None;
            return;
        }
        let path = cutin_texture_path(&image);
        *slot = Some(image);
        if let (Some(grf), Some(renderer)) = (&self.grf, &mut self.renderer) {
            renderer.preload_textures(&[path.as_str()], grf);
        }
    }
}
