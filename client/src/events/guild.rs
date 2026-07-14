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
        honor: i32,
        virtue: i32,
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
            guild.honor = honor;
            guild.virtue = virtue;
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
        self.load_guild_member_sprites();
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

    pub(super) fn handle_guild_member_positions_changed(
        &mut self,
        entries: Vec<(u32, u32, i32)>,
    ) {
        let guild = self.guild_mut();
        for (aid, gid, position_id) in entries {
            if let Some(m) = guild
                .members
                .iter_mut()
                .find(|m| m.gid == gid && m.aid == aid)
            {
                m.position_id = position_id;
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
        let icon_paths = skills
            .iter()
            .map(|s| {
                format!(
                    "data/texture/유저인터페이스/item/{}.bmp",
                    s.name.to_lowercase()
                )
            })
            .collect();
        let guild = self.guild_mut();
        guild.skill_point = point;
        guild.skills = skills;
        self.preload_item_icons(icon_paths);
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

    /// Fetch another entity's guild emblem so it can be drawn over their head /
    /// beside their name. No-op when there is no emblem or it is already cached.
    pub(super) fn request_entity_guild_emblem(&mut self, gdid: u32, version: i32) {
        if gdid == 0 || version == 0 {
            return;
        }
        let key = ragnarok_game::guild::emblem_texture_key(gdid, version);
        let cached = self
            .renderer
            .as_ref()
            .is_some_and(|r| r.texture_cache.texture_size(&key).is_some());
        if cached || !self.game.requested_guild_emblems.insert((gdid, version)) {
            return;
        }
        self.channel.send_packet(ragnarok_network::build_req_guild_emblem_img(
            gdid,
            self.config.packetver,
        ));
    }

    pub(super) fn handle_guild_emblem(&mut self, gdid: u32, version: i32, bmp: Vec<u8>) {
        let key = ragnarok_game::guild::emblem_texture_key(gdid, version);
        if let Some(renderer) = self.renderer.as_mut()
            && renderer.texture_cache.texture_size(&key).is_none()
        {
            match ragnarok_renderer::texture::decode_emblem(&bmp) {
                Some(rgba) => {
                    let (w, h) = (rgba.width(), rgba.height());
                    let bg = ragnarok_renderer::texture::create_texture_bind_group_from_rgba(
                        &renderer.device.device,
                        &renderer.device.queue,
                        rgba.as_raw(),
                        w,
                        h,
                        &renderer.texture_cache.bind_group_layout,
                        &key,
                        ragnarok_renderer::wgpu::FilterMode::Nearest,
                        ragnarok_renderer::wgpu::TextureFormat::Rgba8Unorm,
                        ragnarok_renderer::wgpu::AddressMode::ClampToEdge,
                    );
                    renderer.texture_cache.insert(&key, bg, w, h);
                }
                None => tracing::warn!("Failed to decode guild emblem for guild {gdid}"),
            }
        }

        if let Some(guild) = self.game.guild.as_mut().filter(|g| g.gdid == gdid) {
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
            self.game.guild_head_sprites.clear();
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
            self.game.guild_head_sprites.clear();
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

    pub(super) fn handle_guild_member_expelled(&mut self, name: String, reason: String) {
        let is_self = name == self.game.character.name;
        if is_self {
            self.game.guild = None;
            self.game.guild_head_sprites.clear();
            self.game
                .chat_window
                .add_system("You have been expelled from the guild.".to_string());
            return;
        }
        if let Some(guild) = &mut self.game.guild {
            guild.members.retain(|m| m.name != name);
            guild.ban_list.push(GuildBanEntry {
                char_name: name.clone(),
                account: String::new(),
                reason: reason.clone(),
            });
        }
        let text = if reason.is_empty() {
            format!("{name} has been expelled from the guild.")
        } else {
            format!("{name} has been expelled from the guild. ({reason})")
        };
        self.game.chat_window.add_system(text);
    }

    pub(super) fn handle_guild_disband_result(&mut self, reason: i32) {
        if reason == 0 {
            self.game.guild = None;
            self.game.guild_head_sprites.clear();
            self.game
                .chat_window
                .add_system("The guild has been disbanded.".to_string());
        } else {
            self.game
                .chat_window
                .add_system("Failed to disband the guild.".to_string());
        }
    }

    pub(super) fn handle_guild_invite_received(&mut self, gdid: u32, name: String) {
        self.game.pending_guild_invite = Some(gdid);
        self.game.guild_invite_result.set(None);
        let msg = format!("Join guild \"{name}\"?");
        self.game.confirm_dialog.show_with_out(
            &msg,
            true,
            self.game.guild_invite_result.clone(),
            |_| {},
        );
    }

    pub(super) fn handle_guild_ally_request_received(&mut self, aid: u32, name: String) {
        self.game.pending_guild_ally = Some(aid);
        self.game.guild_ally_result.set(None);
        let msg = format!("Guild \"{name}\" requests an alliance. Accept?");
        self.game.confirm_dialog.show_with_out(
            &msg,
            true,
            self.game.guild_ally_result.clone(),
            |_| {},
        );
    }

    pub(super) fn handle_guild_join_result(&mut self, answer: u8) {
        let msg = match answer {
            0 => "That character is already in a guild.",
            1 => "The guild invitation was rejected.",
            2 => return,
            _ => "The guild is full.",
        };
        self.game.chat_window.add_system(msg.to_string());
    }

    pub(super) fn handle_guild_relation_deleted(&mut self, gdid: u32, relation: i32) {
        if let Some(guild) = &mut self.game.guild {
            guild
                .relations
                .retain(|r| !(r.gdid as u32 == gdid && r.relation == relation));
        }
    }

    pub(super) fn handle_guild_relation_added(&mut self, gdid: u32, relation: i32, name: String) {
        if let Some(guild) = &mut self.game.guild {
            let exists = guild
                .relations
                .iter()
                .any(|r| r.gdid as u32 == gdid && r.relation == relation);
            if !exists {
                guild.relations.push(GuildRelation {
                    gdid: gdid as i32,
                    relation,
                    name,
                });
            }
        }
    }

    pub(super) fn handle_guild_hostile_result(&mut self, result: u8) {
        let msg = match result {
            0 => "The guild has been set as an antagonist.",
            1 => "Your guild has too many antagonists.",
            2 => "This guild is already an antagonist.",
            _ => "Antagonist declarations are currently disabled.",
        };
        self.game.chat_window.add_system(msg.to_string());
    }

    pub(super) fn handle_guild_ally_result(&mut self, answer: u8) {
        let msg = match answer {
            0 => "Your guild is already allied with this guild.",
            1 => "The alliance offer was rejected.",
            2 => "The alliance offer was accepted.",
            3 => "The other guild has too many alliances.",
            4 => "Your guild has too many alliances.",
            _ => "Alliance requests are currently disabled.",
        };
        self.game.chat_window.add_system(msg.to_string());
    }
}
