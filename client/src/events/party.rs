use crate::App;
use ragnarok_game::event::PartyMemberData;
use ragnarok_game::party::{Party, PartyMember};
impl App {
    pub(super) fn handle_party_member_list(&mut self, name: String, members: Vec<PartyMemberData>) {
        let old = self.game.party.take();
        let mut party = Party::new(name);
        if let Some(old) = &old {
            party.exp_share = old.exp_share;
            party.item_pickup_rule = old.item_pickup_rule;
            party.item_division_rule = old.item_division_rule;
        }
        for m in members {
            let (hp, max_hp, x, y) = old
                .as_ref()
                .and_then(|p| p.member(m.aid))
                .map(|e| (e.hp, e.max_hp, e.x, e.y))
                .unwrap_or((None, None, 0, 0));
            party.members.push(PartyMember {
                aid: m.aid,
                name: m.name,
                map: m.map,
                leader: m.leader,
                online: m.online,
                hp,
                max_hp,
                x,
                y,
            });
        }
        self.game.party = Some(party);
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn handle_party_member_added(
        &mut self,
        aid: u32,
        name: String,
        map: String,
        leader: bool,
        online: bool,
        x: u16,
        y: u16,
    ) {
        let party = self
            .game
            .party
            .get_or_insert_with(|| Party::new(String::new()));
        party.upsert_member(PartyMember {
            aid,
            name,
            map,
            leader,
            online,
            hp: None,
            max_hp: None,
            x,
            y,
        });
    }

    pub(super) fn handle_party_member_removed(&mut self, aid: u32, _name: String, _result: u8) {
        let own_aid = self
            .game
            .login_session
            .as_ref()
            .map(|s| s.account_id)
            .unwrap_or(0);
        if aid == own_aid {
            self.game.party = None;
            self.game.party_friends_window.open = false;
            return;
        }
        if let Some(party) = &mut self.game.party {
            party.remove_member(aid);
        }
    }

    pub(super) fn handle_party_member_hp(&mut self, aid: u32, hp: u32, max_hp: u32) {
        if let Some(party) = &mut self.game.party {
            party.set_hp(aid, hp, max_hp);
        }
    }

    pub(super) fn handle_party_member_position(&mut self, aid: u32, x: u16, y: u16) {
        if let Some(party) = &mut self.game.party {
            party.set_position(aid, x, y);
        }
    }

    pub(super) fn handle_party_exp_option_changed(&mut self, exp_option: u32) {
        if let Some(party) = &mut self.game.party {
            party.exp_share = exp_option != 0;
        }
    }

    pub(super) fn handle_party_invite_received(&mut self, party_grid: u32, party_name: String) {
        if self.game.self_config.refuse_party_invite {
            self.channel
                .send_packet(ragnarok_network::build_join_party_reply_packet(
                    party_grid,
                    false,
                    self.config.packetver,
                ));
            return;
        }
        self.game.pending_confirms.pending_party_invite = Some(party_grid);
        self.game.pending_confirms.party_invite_result.set(None);
        let msg = format!("Join party \"{party_name}\"?");
        self.game.confirm_dialog.show_with_out(
            &msg,
            true,
            self.game.pending_confirms.party_invite_result.clone(),
            |_| {},
        );
    }

    pub(super) fn handle_party_invite_result(&mut self, name: String, answer: u8) {
        let text = match answer {
            0 => format!("{name} is already in a party."),
            1 => format!("{name} rejected the party invitation."),
            2 => format!("{name} joined the party."),
            3 => "The party is full.".to_string(),
            4 => format!("{name} is already a party member."),
            _ => format!("Party invitation to {name} failed."),
        };
        self.game.chat_window.add_system(text);
    }

    pub(super) fn handle_party_create_result(&mut self, result: u8) {
        if result == 0 {
            self.game.party_friends_window.open = true;
            // The party now exists server-side; send any invite that was deferred
            // while waiting for this ack.
            if let Some(aid) = self.game.pending_confirms.pending_invite_aid.take() {
                self.channel.send_packet(
                    ragnarok_network::build_req_join_party_packet(aid, self.config.packetver),
                );
            }
        } else {
            self.game.pending_confirms.pending_invite_aid = None;
            self.game
                .chat_window
                .add_system("Failed to create party.".to_string());
        }
    }

    pub(super) fn handle_party_chat_message(&mut self, aid: u32, message: String) {
        let sender = self
            .game
            .party
            .as_ref()
            .and_then(|p| p.member(aid))
            .map(|m| m.name.clone())
            .or_else(|| {
                self.game
                    .entities
                    .get(aid)
                    .and_then(|e| e.name.clone())
            });
        let text = match sender {
            Some(name) => format!("{name} : {message}"),
            None => message,
        };
        self.game.chat_window.add_party(text);
    }
}
