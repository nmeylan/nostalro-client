use std::time::{Duration, Instant};

pub const GUILD_PERM_INVITE: i32 = 0x001;
pub const GUILD_PERM_EXPEL: i32 = 0x010;
pub const GUILD_PERM_STORAGE: i32 = 0x100;

pub fn emblem_texture_key(gdid: u32, version: i32) -> String {
    format!("__guild_emblem_{gdid}_{version}")
}

/// The same emblem uploaded for the guild flag's 3D model, which samples sRGB.
pub fn emblem_model_texture_key(gdid: u32, version: i32) -> String {
    format!("__guild_emblem_model_{gdid}_{version}")
}

pub fn validate_emblem_bmp(bmp: &[u8]) -> Result<(), String> {
    const MAX_LEN: usize = 1782;
    if bmp.len() < 54 {
        return Err("emblem file is too small to be a valid BMP.".to_string());
    }
    if bmp[0] != 0x42 || bmp[1] != 0x4D {
        return Err("emblem is not a BMP image.".to_string());
    }
    let bf_size = u32::from_le_bytes([bmp[2], bmp[3], bmp[4], bmp[5]]) as usize;
    if bf_size != bmp.len() {
        return Err("emblem BMP header size does not match the file.".to_string());
    }
    let bf_off_bits = u32::from_le_bytes([bmp[10], bmp[11], bmp[12], bmp[13]]) as usize;
    if bf_off_bits >= bmp.len() {
        return Err("emblem BMP pixel offset is invalid.".to_string());
    }
    if bmp.len() > MAX_LEN {
        return Err("emblem is too large; it must be 24x24.".to_string());
    }
    let width = i32::from_le_bytes([bmp[18], bmp[19], bmp[20], bmp[21]]).abs();
    let height = i32::from_le_bytes([bmp[22], bmp[23], bmp[24], bmp[25]]).abs();
    if width != 24 || height != 24 {
        return Err(format!("emblem must be 24x24 (got {width}x{height})."));
    }
    Ok(())
}

#[derive(Debug, Clone, Default)]
pub struct Guild {
    pub gdid: u32,
    pub name: String,
    pub level: i32,
    pub exp: i32,
    pub max_exp: i32,
    pub member_num: i32,
    pub max_member_num: i32,
    pub avg_level: i32,
    pub point: i32,
    pub honor: i32,
    pub virtue: i32,
    pub master_name: String,
    pub manage_land: String,
    pub emblem_version: i32,
    pub emblem_bmp: Option<Vec<u8>>,
    pub notice_subject: String,
    pub notice_body: String,
    pub members: Vec<GuildMember>,
    pub positions: Vec<GuildPosition>,
    pub skills: Vec<GuildSkill>,
    pub skill_point: i16,
    pub ban_list: Vec<GuildBanEntry>,
    pub relations: Vec<GuildRelation>,
    pub other_guilds: Vec<OtherGuild>,
    pub my_right: i32,
    pub am_i_master: bool,
    pub disband_deadline: Option<Instant>,
}

#[derive(Debug, Clone, Default)]
pub struct GuildMember {
    pub aid: u32,
    pub gid: u32,
    pub name: String,
    pub job: i16,
    pub level: i16,
    pub head: i16,
    pub head_palette: i16,
    pub sex: i16,
    pub position_id: i32,
    pub position_name: String,
    pub contribution_exp: i32,
    pub online: bool,
    pub note: String,
    pub cur_map: String,
    pub last_offline: i32,
    pub x: u16,
    pub y: u16,
    pub has_live_position: bool,
}

#[derive(Debug, Clone, Default)]
pub struct GuildPosition {
    pub id: i32,
    pub name: String,
    pub right: i32,
    pub ranking: i32,
    pub pay_rate: i32,
}

#[derive(Debug, Clone, Default)]
pub struct GuildSkill {
    pub skid: u16,
    pub name: String,
    pub level: i16,
    pub sp_cost: i16,
    pub attack_range: i16,
    pub upgradable: bool,
    pub passive: bool,
}

#[derive(Debug, Clone, Default)]
pub struct GuildBanEntry {
    pub char_name: String,
    pub account: String,
    pub reason: String,
}

#[derive(Debug, Clone, Default)]
pub struct GuildRelation {
    pub gdid: i32,
    pub name: String,
    pub relation: i32,
}

#[derive(Debug, Clone, Default)]
pub struct OtherGuild {
    pub name: String,
    pub level: i32,
    pub member_size: i32,
    pub ranking: i32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GuildRights {
    pub can_invite: bool,
    pub can_expel: bool,
    pub can_storage: bool,
}

impl GuildRights {
    fn from_bits(right: i32) -> Self {
        Self {
            can_invite: right & GUILD_PERM_INVITE != 0,
            can_expel: right & GUILD_PERM_EXPEL != 0,
            can_storage: right & GUILD_PERM_STORAGE != 0,
        }
    }
}

impl Guild {
    pub fn member_by_gid(&self, gid: u32) -> Option<&GuildMember> {
        self.members.iter().find(|m| m.gid == gid)
    }

    pub fn position_by_id(&self, id: i32) -> Option<&GuildPosition> {
        self.positions.iter().find(|p| p.id == id)
    }

    fn ranking_of(&self, position_id: i32) -> i32 {
        self.position_by_id(position_id)
            .map(|p| p.ranking)
            .unwrap_or(i32::MAX)
    }

    /// Online members first, then by ascending position ranking (master's rank
    /// sorts to the top).
    pub fn sorted_members(&self) -> Vec<&GuildMember> {
        let mut members: Vec<&GuildMember> = self.members.iter().collect();
        members.sort_by(|a, b| {
            b.online.cmp(&a.online).then_with(|| {
                self.ranking_of(a.position_id)
                    .cmp(&self.ranking_of(b.position_id))
            })
        });
        members
    }

    pub fn my_rights(&self, my_gid: u32) -> GuildRights {
        let right = self
            .member_by_gid(my_gid)
            .and_then(|m| self.position_by_id(m.position_id))
            .map(|p| p.right)
            .unwrap_or(self.my_right);
        GuildRights::from_bits(right)
    }

    pub fn is_master(&self, my_gid: u32) -> bool {
        self.member_by_gid(my_gid)
            .map(|m| m.position_id == 0)
            .unwrap_or(self.am_i_master)
    }

    /// The server only sends a member's coordinates while it shares the local
    /// player's map, so a received position marks the member as on-screen.
    pub fn set_position(&mut self, aid: u32, x: u16, y: u16) {
        if let Some(m) = self.members.iter_mut().find(|m| m.aid == aid) {
            m.x = x;
            m.y = y;
            m.has_live_position = true;
        }
    }

    pub fn clear_position_of(&mut self, aid: u32) {
        if let Some(m) = self.members.iter_mut().find(|m| m.aid == aid) {
            m.has_live_position = false;
        }
    }

    pub fn clear_live_positions(&mut self) {
        for m in &mut self.members {
            m.has_live_position = false;
        }
    }

    pub fn disband_remaining(&self, now: Instant) -> Option<Duration> {
        self.disband_deadline
            .map(|deadline| deadline.saturating_duration_since(now))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn position(id: i32, ranking: i32, right: i32) -> GuildPosition {
        GuildPosition {
            id,
            name: format!("Rank{id}"),
            right,
            ranking,
            pay_rate: 0,
        }
    }

    fn member(gid: u32, position_id: i32, online: bool) -> GuildMember {
        GuildMember {
            gid,
            aid: gid,
            name: format!("Member{gid}"),
            position_id,
            online,
            ..Default::default()
        }
    }

    #[test]
    fn sorting_rights_and_master_cross_members_and_positions() {
        let mut guild = Guild {
            positions: vec![
                position(
                    0,
                    0,
                    GUILD_PERM_INVITE | GUILD_PERM_EXPEL | GUILD_PERM_STORAGE,
                ),
                position(1, 1, GUILD_PERM_INVITE),
                position(2, 2, 0),
            ],
            ..Default::default()
        };
        guild.members = vec![
            member(10, 2, false), // offline grunt
            member(11, 0, true),  // online master
            member(12, 1, true),  // online officer
        ];

        let order: Vec<u32> = guild.sorted_members().iter().map(|m| m.gid).collect();
        assert_eq!(order, vec![11, 12, 10]);

        assert!(guild.is_master(11));
        assert!(!guild.is_master(12));

        let officer = guild.my_rights(12);
        assert!(officer.can_invite && !officer.can_expel && !officer.can_storage);
        let master = guild.my_rights(11);
        assert!(master.can_invite && master.can_expel && master.can_storage);
    }

    #[test]
    fn live_position_sets_then_clears() {
        let mut guild = Guild::default();
        guild.members = vec![member(11, 0, true)];
        guild.set_position(11, 150, 160);
        let m = guild.member_by_gid(11).unwrap();
        assert_eq!((m.x, m.y, m.has_live_position), (150, 160, true));
        guild.clear_live_positions();
        assert!(!guild.member_by_gid(11).unwrap().has_live_position);
    }

    #[test]
    fn disband_deadline_counts_down() {
        let now = Instant::now();
        let mut guild = Guild::default();
        assert!(guild.disband_remaining(now).is_none());
        guild.disband_deadline = Some(now + Duration::from_secs(60));
        assert_eq!(guild.disband_remaining(now), Some(Duration::from_secs(60)));
        assert_eq!(
            guild.disband_remaining(now + Duration::from_secs(90)),
            Some(Duration::ZERO)
        );
    }
}
