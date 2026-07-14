use crate::App;
use ragnarok_game::guild::{
    Guild, GuildBanEntry, GuildMember, GuildPosition, GuildRelation, GuildSkill, OtherGuild,
};

impl App {
    fn guild_mut(&mut self) -> &mut Guild {
        self.game.guild.get_or_insert_with(Guild::default)
    }

    fn apply_position_names(guild: &mut Guild) {
        for member in &mut guild.members {
            member.position_name = guild
                .positions
                .iter()
                .find(|p| p.id == member.position_id)
                .map(|p| p.name.clone())
                .unwrap_or_default();
        }
    }

    pub(super) fn handle_guild_menu_flag(&mut self, flag: i32) {
        self.game.guild_menu_flag = flag;
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn handle_guild_info(
        &mut self,
        gdid: u32,
        name: String,
        level: i32,
        exp: i32,
        max_exp: i32,
        member_num: i32,
        max_member_num: i32,
        avg_level: i32,
        point: i32,
        master_name: String,
        manage_land: String,
        emblem_version: i32,
    ) {
        let request_emblem = {
            let guild = self.guild_mut();
            let changed = guild.gdid != gdid
                || guild.emblem_version != emblem_version
                || guild.emblem_bmp.is_none();
            guild.gdid = gdid;
            guild.name = name;
            guild.level = level;
            guild.exp = exp;
            guild.max_exp = max_exp;
            guild.member_num = member_num;
            guild.max_member_num = max_member_num;
            guild.avg_level = avg_level;
            guild.point = point;
            guild.master_name = master_name;
            guild.manage_land = manage_land;
            guild.emblem_version = emblem_version;
            changed && emblem_version != 0
        };
        if request_emblem {
            self.channel.send_packet(ragnarok_network::build_req_guild_emblem_img(
                gdid,
                self.config.packetver,
            ));
        }
    }

    pub(super) fn handle_guild_members(&mut self, members: Vec<GuildMember>) {
        let guild = self.guild_mut();
        guild.members = members;
        Self::apply_position_names(guild);
    }

    pub(super) fn handle_guild_positions(&mut self, positions: Vec<GuildPosition>) {
        let guild = self.guild_mut();
        for pos in positions {
            if let Some(existing) = guild.positions.iter_mut().find(|p| p.id == pos.id) {
                let name = std::mem::take(&mut existing.name);
                *existing = GuildPosition { name, ..pos };
            } else {
                guild.positions.push(pos);
            }
        }
        Self::apply_position_names(guild);
    }

    pub(super) fn handle_guild_position_names(&mut self, names: Vec<(i32, String)>) {
        let guild = self.guild_mut();
        for (id, name) in names {
            if let Some(existing) = guild.positions.iter_mut().find(|p| p.id == id) {
                existing.name = name;
            } else {
                guild.positions.push(GuildPosition {
                    id,
                    name,
                    ..GuildPosition::default()
                });
            }
        }
        Self::apply_position_names(guild);
    }

    pub(super) fn handle_guild_skills(&mut self, point: i16, skills: Vec<GuildSkill>) {
        let guild = self.guild_mut();
        guild.skill_point = point;
        guild.skills = skills;
    }

    pub(super) fn handle_guild_ban_list(&mut self, entries: Vec<GuildBanEntry>) {
        self.guild_mut().ban_list = entries;
    }

    pub(super) fn handle_guild_notice(&mut self, subject: String, body: String) {
        let guild = self.guild_mut();
        guild.notice_subject = subject.clone();
        guild.notice_body = body.clone();
        if !subject.is_empty() {
            self.game.chat_window.add_system(format!("[Guild] {subject}"));
        }
        if !body.is_empty() {
            self.game.chat_window.add_system(body);
        }
    }

    pub(super) fn handle_guild_other_list(&mut self, guilds: Vec<OtherGuild>) {
        self.guild_mut().other_guilds = guilds;
    }

    pub(super) fn handle_guild_relations(&mut self, relations: Vec<GuildRelation>) {
        self.guild_mut().relations = relations;
    }

    pub(super) fn handle_guild_emblem(&mut self, gdid: u32, version: i32, bmp: Vec<u8>) {
        let guild = self.guild_mut();
        if guild.gdid == gdid {
            guild.emblem_version = version;
            guild.emblem_bmp = Some(bmp);
        }
    }

    pub(super) fn handle_guild_identity_updated(
        &mut self,
        gdid: u32,
        emblem_version: i32,
        right: i32,
        is_master: bool,
        name: String,
    ) {
        if gdid == 0 {
            self.game.guild = None;
            return;
        }
        let guild = self.guild_mut();
        guild.gdid = gdid;
        guild.emblem_version = emblem_version;
        guild.my_right = right;
        guild.am_i_master = is_master;
        if !name.is_empty() {
            guild.name = name;
        }
    }

    pub(super) fn handle_guild_create_result(&mut self, result: u8) {
        let text = match result {
            0 => "Guild created.".to_string(),
            1 => "You are already in a guild.".to_string(),
            2 => "That guild name is already taken.".to_string(),
            3 => "You need an Emperium to create a guild.".to_string(),
            _ => "Failed to create guild.".to_string(),
        };
        if result == 0 {
            self.game.guild_window.open();
        }
        self.game.chat_window.add_system(text);
    }

    pub(super) fn handle_guild_member_left(&mut self, name: String, reason: String) {
        let is_self = name == self.game.character.name;
        if is_self {
            self.game.guild = None;
            self.game
                .chat_window
                .add_system("You have left the guild.".to_string());
            return;
        }
        if let Some(guild) = &mut self.game.guild {
            guild.members.retain(|m| m.name != name);
        }
        let text = if reason.is_empty() {
            format!("{name} has left the guild.")
        } else {
            format!("{name} has left the guild. ({reason})")
        };
        self.game.chat_window.add_system(text);
    }

    pub(super) fn handle_guild_disband_result(&mut self, reason: i32) {
        if reason == 0 {
            self.game.guild = None;
            self.game
                .chat_window
                .add_system("The guild has been disbanded.".to_string());
        } else {
            self.game
                .chat_window
                .add_system("Failed to disband the guild.".to_string());
        }
    }

    pub(super) fn handle_guild_invite_received(&mut self, _gdid: u32, name: String) {
        self.game
            .chat_window
            .add_system(format!("You have been invited to join guild \"{name}\"."));
    }

    pub(super) fn handle_guild_ally_request_received(&mut self, _aid: u32, name: String) {
        self.game
            .chat_window
            .add_system(format!("Guild \"{name}\" requests an alliance."));
    }
}
