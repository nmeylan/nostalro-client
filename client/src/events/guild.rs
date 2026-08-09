use crate::App;
use ragnarok_game::event::{GameEvent, GuildMemberAppearance};
use ragnarok_game::guild::{
    Guild, GuildBanEntry, GuildMember, GuildPosition, GuildRelation, GuildSkill, OtherGuild,
};

const GUILD_MENU_ON_MAP_ENTRY: [i32; 4] = [0, 1, 2, 3];

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

    /// Info, member list, positions and skills, pulled on every world entry.
    /// Without the member list arriving on its own, a guild mate's position
    /// packet has no row to land on and the minimap stays blank until the guild
    /// window is opened.
    pub(super) fn request_guild_data(&mut self) {
        for atype in GUILD_MENU_ON_MAP_ENTRY {
            self.channel
                .send_packet(ragnarok_network::build_req_guild_menu(
                    atype,
                    self.active_packetver,
                ));
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
        if let Some(player) = self.game.world.entities.player_mut() {
            player.guild_id = gdid;
            player.guild_emblem_version = emblem_version;
        }
        if request_emblem {
            self.channel
                .send_packet(ragnarok_network::build_req_guild_emblem_img(
                    gdid,
                    self.active_packetver,
                ));
        }
    }

    pub(super) fn handle_guild_members(&mut self, members: Vec<GuildMember>) {
        let guild = self.guild_mut();
        guild.members = members;
        Self::apply_position_names(guild);
        self.load_guild_member_sprites();
    }

    /// The server also replays every member's current status right after login, so
    /// only announce a status that actually flipped.
    pub(super) fn handle_guild_member_online(
        &mut self,
        aid: u32,
        gid: u32,
        online: bool,
        appearance: Option<GuildMemberAppearance>,
    ) {
        let Some(guild) = &mut self.game.guild else {
            return;
        };
        let Some(member) = guild
            .members
            .iter_mut()
            .find(|m| m.gid == gid || m.aid == aid)
        else {
            return;
        };
        let changed = member.online != online;
        member.online = online;
        if let Some(a) = appearance {
            member.sex = a.sex;
            member.head = a.head;
            member.head_palette = a.head_palette;
        }
        let name = member.name.clone();
        if !online {
            guild.clear_position_of(aid);
        }
        let is_master = guild.master_name == name;

        if changed && !name.is_empty() {
            let role = if is_master {
                "Guild master"
            } else {
                "Guild member"
            };
            let state = if online { "online" } else { "offline" };
            self.windows
                .chat_window
                .add_system(format!("{role} {name} is {state}."));
        }
        if appearance.is_some() {
            self.load_guild_member_sprites();
        }
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

    pub(super) fn handle_guild_member_positions_changed(&mut self, entries: Vec<(u32, u32, i32)>) {
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
            .map(|s| ragnarok_resources::ui::item::icon(&s.name.to_lowercase()))
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
            self.windows
                .chat_window
                .add_system(format!("[Guild] {subject}"));
        }
        if !body.is_empty() {
            self.windows.chat_window.add_system(body);
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
        self.channel
            .send_packet(ragnarok_network::build_req_guild_emblem_img(
                gdid,
                self.active_packetver,
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
            self.game.sprite_caches.guild_head_sprites.clear();
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
        if let Some(player) = self.game.world.entities.player_mut() {
            player.guild_id = gdid;
            player.guild_emblem_version = emblem_version;
        }
        self.request_entity_guild_emblem(gdid, emblem_version);
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
            self.windows.guild_window.open();
        }
        self.windows.chat_window.add_system(text);
    }

    pub(super) fn handle_guild_member_left(&mut self, name: String, reason: String) {
        let is_self = name == self.game.character.name;
        if is_self {
            self.game.guild = None;
            self.game.sprite_caches.guild_head_sprites.clear();
            self.windows
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
        self.windows.chat_window.add_system(text);
    }

    pub(super) fn handle_guild_member_expelled(&mut self, name: String, reason: String) {
        let is_self = name == self.game.character.name;
        if is_self {
            self.game.guild = None;
            self.game.sprite_caches.guild_head_sprites.clear();
            self.windows
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
        self.windows.chat_window.add_system(text);
    }

    pub(super) fn handle_guild_disband_result(&mut self, reason: i32) {
        if reason == 0 {
            self.game.guild = None;
            self.game.sprite_caches.guild_head_sprites.clear();
            self.windows
                .chat_window
                .add_system("The guild has been disbanded.".to_string());
        } else {
            self.windows
                .chat_window
                .add_system("Failed to disband the guild.".to_string());
        }
    }

    pub(super) fn handle_guild_invite_received(&mut self, gdid: u32, name: String) {
        let msg = format!("Join guild \"{name}\"?");
        self.game
            .arm_confirm(&mut self.windows, &msg, move |accept| {
                Some(GameEvent::RespondGuildInvite { gdid, accept })
            });
    }

    pub(super) fn handle_guild_ally_request_received(&mut self, aid: u32, name: String) {
        let msg = format!("Guild \"{name}\" requests an alliance. Accept?");
        self.game
            .arm_confirm(&mut self.windows, &msg, move |accept| {
                Some(GameEvent::RespondGuildAlly { aid, accept })
            });
    }

    pub(super) fn handle_guild_join_result(&mut self, answer: u8) {
        let msg = match answer {
            0 => "That character is already in a guild.",
            1 => "The guild invitation was rejected.",
            2 => return,
            _ => "The guild is full.",
        };
        self.windows.chat_window.add_system(msg.to_string());
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
        self.windows.chat_window.add_system(msg.to_string());
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
        self.windows.chat_window.add_system(msg.to_string());
    }
}

impl App {
    pub(crate) fn local_aid_gid(&self) -> (u32, u32) {
        let aid = self
            .game
            .session
            .login_session
            .as_ref()
            .map(|s| s.account_id)
            .unwrap_or(0);
        (aid, aid)
    }

    pub(crate) fn open_emblem_picker(&mut self) {
        use ragnarok_ui_component::game::emblem_picker_window::EmblemEntry;

        let dir = std::path::PathBuf::from(&self.config.emblem_path);
        let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(&dir)
            .ok()
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("bmp")))
            .collect();
        files.sort();

        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };
        let mut entries = Vec::new();
        for path in files {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let Ok(bytes) = std::fs::read(&path) else {
                continue;
            };
            let (valid, verdict) = match ragnarok_game::guild::validate_emblem_bmp(&bytes) {
                Ok(()) => (true, String::new()),
                Err(e) => (false, e),
            };
            let key = format!("__emblem_file_{name}");
            if renderer.texture_cache.texture_size(&key).is_none()
                && let Some(rgba) = ragnarok_renderer::texture::decode_emblem(&bytes)
            {
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
            entries.push(EmblemEntry {
                name,
                path: path.to_string_lossy().to_string(),
                key,
                valid,
                verdict,
            });
        }
        if entries.is_empty() {
            self.windows
                .chat_window
                .add_system(format!("No emblem .bmp found in '{}'.", dir.display()));
        }
        self.windows.emblem_picker_window.open(entries);
    }

    pub(crate) fn upload_emblem_file(&mut self, path: &str) {
        let Ok(bmp) = std::fs::read(path) else {
            self.windows
                .chat_window
                .add_system(format!("Failed to read emblem '{path}'."));
            return;
        };
        if let Err(e) = ragnarok_game::guild::validate_emblem_bmp(&bmp) {
            self.windows.chat_window.add_system(e);
            return;
        }
        let compressed = ragnarok_formats::zlib_compress(&bmp);
        self.channel
            .send_packet(ragnarok_network::build_register_guild_emblem(
                compressed,
                self.active_packetver,
            ));
        if let Some(guild) = &self.game.guild {
            self.channel
                .send_packet(ragnarok_network::build_req_guild_emblem_img(
                    guild.gdid,
                    self.active_packetver,
                ));
        }
    }
}

impl App {
    pub(super) fn handle_show_guild_member_menu(
        &mut self,
        aid: u32,
        gid: u32,
        name: String,
        x: f32,
        y: f32,
    ) {
        use ragnarok_ui_component::game::context_menu::{ContextMenuAction, ContextMenuItem};
        let local_gid = self
            .game
            .session
            .login_session
            .as_ref()
            .map(|s| s.account_id)
            .unwrap_or(0);
        let mut items = Vec::new();
        let is_self = gid == local_gid;
        if !is_self {
            items.push(ContextMenuItem {
                label: "Whisper".to_string(),
                action: ContextMenuAction::Whisper { name: name.clone() },
            });
        }
        if let Some(g) = &self.game.guild {
            let target_master = g
                .member_by_gid(gid)
                .map(|m| m.position_id == 0)
                .unwrap_or(false);
            if is_self && !g.is_master(local_gid) {
                items.push(ContextMenuItem {
                    label: "Leave Guild".to_string(),
                    action: ContextMenuAction::GuildLeave,
                });
            }
            if g.is_master(local_gid) && !target_master {
                for p in &g.positions {
                    items.push(ContextMenuItem {
                        label: format!("Set: {}", p.name),
                        action: ContextMenuAction::ChangeGuildPosition {
                            aid,
                            gid,
                            position_id: p.id,
                        },
                    });
                }
                items.push(ContextMenuItem {
                    label: "Expel".to_string(),
                    action: ContextMenuAction::ExpelFromGuild { aid, gid, name },
                });
            }
        }
        self.windows.context_menu.open_at(x, y, items);
    }
}
